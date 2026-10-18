//! Runs every slice the wheels offer through the real converter.
//! Point TAPWHEEL_FFMPEG at an ffmpeg build; without it these tests are skipped.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use tapwheel_core::run::{Engines, run};
use tapwheel_core::{Op, Wheel, choices, plan};

fn ffmpeg() -> Option<PathBuf> {
	std::env::var_os("TAPWHEEL_FFMPEG").map(PathBuf::from)
}

fn scratch(name: &str) -> PathBuf {
	let dir = std::env::temp_dir().join(format!("tw-engine-{name}-{}", std::process::id()));
	let _ = fs::remove_dir_all(&dir);
	fs::create_dir_all(&dir).unwrap();
	dir
}

fn make(ff: &Path, args: &[&str], out: &Path) {
	let ok = Command::new(ff)
		.args(["-hide_banner", "-v", "error", "-y"])
		.args(args)
		.arg(out)
		.status()
		.unwrap()
		.success();
	assert!(ok, "fixture {} failed", out.display());
}

/// Decodes the whole file; a broken output fails here.
fn decodes(ff: &Path, file: &Path) -> bool {
	Command::new(ff)
		.args(["-v", "error", "-i"])
		.arg(file)
		.args(["-f", "null", "-"])
		.status()
		.map(|s| s.success())
		.unwrap_or(false)
}

fn fixtures(ff: &Path, dir: &Path) -> Vec<PathBuf> {
	let png = dir.join("alpha.png");
	make(
		ff,
		&[
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=160x120:d=1",
			"-vf",
			"format=rgba,geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='if(lt(X,40),0,255)'",
			"-frames:v",
			"1",
		],
		&png,
	);
	let jpg = dir.join("photo.jpg");
	make(
		ff,
		&["-f", "lavfi", "-i", "testsrc2=s=161x121:d=1", "-frames:v", "1"],
		&jpg,
	);
	let gif = dir.join("anim.gif");
	make(ff, &["-f", "lavfi", "-i", "testsrc2=s=120x90:d=1:r=8"], &gif);
	let wav = dir.join("tone.wav");
	make(ff, &["-f", "lavfi", "-i", "sine=f=440:d=2"], &wav);
	let mp4 = dir.join("clip.mp4");
	make(
		ff,
		&[
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=322x182:d=2:r=15",
			"-f",
			"lavfi",
			"-i",
			"sine=d=2",
			"-c:v",
			"libx264",
			"-pix_fmt",
			"yuv420p",
			"-c:a",
			"aac",
			"-shortest",
		],
		&mp4,
	);
	let srt = dir.join("subs.srt");
	fs::write(&srt, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
	let heic = dir.join("photo.heic");
	fs::copy(
		Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/photo.heic"),
		&heic,
	)
	.unwrap();
	vec![png, jpg, gif, wav, mp4, srt, heic]
}

#[test]
fn every_slice_produces_a_readable_file() {
	let Some(ff) = ffmpeg() else {
		eprintln!("TAPWHEEL_FFMPEG not set, skipping");
		return;
	};
	let dir = scratch("all");
	let engines = Engines {
		ffmpeg: ff.clone(),
		scratch: dir.join(".scratch"),
	};
	let never = AtomicBool::new(false);
	let mut ran = 0;
	let mut failures = Vec::new();
	for input in fixtures(&ff, &dir) {
		let before = fs::read(&input).unwrap();
		for wheel in [Wheel::Convert, Wheel::Tools] {
			for op in choices(wheel, std::slice::from_ref(&input)) {
				for job in plan(op, std::slice::from_ref(&input)) {
					let mut last = 0.0;
					match run(&job, &engines, &mut |p| last = p, &never) {
						Ok(out) => {
							ran += 1;
							assert!(out.output.exists(), "{op:?} on {}", input.display());
							assert!(out.bytes_out > 0, "{op:?} on {} wrote nothing", input.display());
							let check_decode = !out.output.is_dir()
								&& !matches!(op, Op::Convert(f) if f.is_archive() || matches!(f, tapwheel_core::Format::Txt | tapwheel_core::Format::Srt | tapwheel_core::Format::Vtt))
								&& op != Op::Zip;
							if check_decode && !decodes(&ff, &out.output) {
								failures.push(format!("{op:?} on {}: output does not decode", input.display()));
							}
						}
						Err(e) => failures.push(format!("{op:?} on {}: {e}", input.display())),
					}
				}
			}
		}
		assert_eq!(fs::read(&input).unwrap(), before, "{} was changed", input.display());
	}
	let leftovers: Vec<_> = fs::read_dir(&dir)
		.unwrap()
		.flatten()
		.filter(|e| e.file_name().to_string_lossy().ends_with(".tw-part"))
		.collect();
	assert!(leftovers.is_empty(), "part files left behind: {leftovers:?}");
	assert!(
		failures.is_empty(),
		"{} of {} failed:\n{}",
		failures.len(),
		ran + failures.len(),
		failures.join("\n")
	);
	assert!(ran > 60, "only {ran} jobs ran");
	fs::remove_dir_all(dir).unwrap();
}

#[test]
fn cancel_kills_the_converter_and_cleans_up() {
	let Some(ff) = ffmpeg() else {
		return;
	};
	let dir = scratch("cancel");
	let src = dir.join("long.mp4");
	make(
		&ff,
		&[
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=640x360:d=30:r=30",
			"-c:v",
			"libx264",
			"-preset",
			"ultrafast",
		],
		&src,
	);
	let job = plan(Op::Convert(tapwheel_core::Format::Webm), std::slice::from_ref(&src)).remove(0);
	let stop = AtomicBool::new(false);
	let engines = Engines {
		ffmpeg: ff,
		scratch: dir.join(".scratch"),
	};
	let err = run(
		&job,
		&engines,
		&mut |p| {
			if p > 0.0 {
				stop.store(true, std::sync::atomic::Ordering::Relaxed);
			}
		},
		&stop,
	)
	.unwrap_err();
	assert!(matches!(err, tapwheel_core::Error::Cancelled), "{err}");
	assert!(!job.output.exists());
	assert!(!tapwheel_core::plan::part_path(&job.output).exists());
	fs::remove_dir_all(dir).unwrap();
}

#[test]
fn outputs_never_replace_existing_files() {
	let Some(ff) = ffmpeg() else {
		return;
	};
	let dir = scratch("names");
	let src = dir.join("a.jpg");
	make(&ff, &["-f", "lavfi", "-i", "testsrc2=s=64x64", "-frames:v", "1"], &src);
	let job = plan(Op::Convert(tapwheel_core::Format::Png), std::slice::from_ref(&src)).remove(0);
	// someone writes a.png after the job was planned
	fs::write(dir.join("a.png"), b"mine").unwrap();
	let engines = Engines {
		ffmpeg: ff,
		scratch: dir.join(".scratch"),
	};
	let out = run(&job, &engines, &mut |_| {}, &AtomicBool::new(false)).unwrap();
	assert_eq!(out.output, dir.join("a (1).png"));
	assert_eq!(fs::read(dir.join("a.png")).unwrap(), b"mine");
	fs::remove_dir_all(dir).unwrap();
}
