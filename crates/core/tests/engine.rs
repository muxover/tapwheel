//! Runs every slice on every wheel, for every kind of file, through the real engines.
//! Point TAPWHEEL_FFMPEG at an ffmpeg build (ffprobe next to it); without it these tests are skipped.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use tapwheel_core::recipe::{Layout, Recipe, Span};
use tapwheel_core::run::{Engines, run};
use tapwheel_core::{Defaults, Format, Kind, Op, Wheel, choices, plan, recipe_for};

fn engines(name: &str) -> Option<(Engines, PathBuf)> {
	let ff = PathBuf::from(std::env::var_os("TAPWHEEL_FFMPEG")?);
	let dir = std::env::temp_dir().join(format!("tw-engine-{name}-{}", std::process::id()));
	let _ = fs::remove_dir_all(&dir);
	fs::create_dir_all(&dir).unwrap();
	let mut e = Engines::in_dir(ff.parent().unwrap(), dir.join(".scratch"));
	e.ffmpeg = ff;
	if let Some(m) = std::env::var_os("TAPWHEEL_MODEL_DIR") {
		let m = PathBuf::from(m);
		let found = Engines::in_dir(&m, dir.join(".scratch"));
		e.model = found.model;
		e.onnxruntime = found.onnxruntime;
	}
	Some((e, dir))
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

fn size(e: &Engines, file: &Path) -> (u32, u32) {
	let p = tapwheel_core::ff::probe(&e.ffprobe, file).unwrap();
	(p.width, p.height)
}

/// A JPG that says "turn me a quarter clockwise", like most phone photos.
fn sideways_jpeg(ff: &Path, dir: &Path) -> PathBuf {
	let plain = dir.join("plain.jpg");
	make(
		ff,
		&["-f", "lavfi", "-i", "testsrc2=s=200x100:d=1", "-frames:v", "1"],
		&plain,
	);
	let mut tiff = b"II*\0\x08\0\0\0".to_vec();
	tiff.extend(1u16.to_le_bytes());
	tiff.extend(0x0112u16.to_le_bytes());
	tiff.extend(3u16.to_le_bytes());
	tiff.extend(1u32.to_le_bytes());
	tiff.extend(6u16.to_le_bytes());
	tiff.extend([0, 0, 0, 0, 0, 0]);
	let with = tapwheel_core::metadata::insert_exif(&fs::read(&plain).unwrap(), &tiff).unwrap();
	let out = dir.join("phone.jpg");
	fs::write(&out, with).unwrap();
	fs::remove_file(plain).unwrap();
	out
}

fn fixtures(e: &Engines, dir: &Path) -> Vec<PathBuf> {
	let ff = &e.ffmpeg;
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
	let jpg = sideways_jpeg(ff, dir);
	let gif = dir.join("anim.gif");
	make(ff, &["-f", "lavfi", "-i", "testsrc2=s=120x90:d=1:r=8"], &gif);
	let wav = dir.join("tone.wav");
	make(ff, &["-f", "lavfi", "-i", "sine=f=440:d=3", "-af", "volume=0.5"], &wav);
	let mp4 = dir.join("clip.mp4");
	make(
		ff,
		&[
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=322x182:d=3:r=15",
			"-f",
			"lavfi",
			"-i",
			"sine=d=3",
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
	let svg = dir.join("logo.svg");
	fs::write(
		&svg,
		r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><circle cx="32" cy="32" r="28" fill="#b6f23a"/><text x="32" y="40" font-size="20" text-anchor="middle">T</text></svg>"##,
	)
	.unwrap();
	let srt = dir.join("subs.srt");
	fs::write(&srt, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
	let csv = dir.join("people.csv");
	fs::write(&csv, "name,age,city\nzed,31,Cairo\nann,25,Dubai\nann,25,Dubai\n").unwrap();
	let json = dir.join("data.json");
	fs::write(
		&json,
		r#"{"shop":{"name":"Corner","items":[{"n":"tea","p":2},{"n":"cake","p":3.5}]}}"#,
	)
	.unwrap();
	let md = dir.join("notes.md");
	fs::write(&md, "# Notes\n\nSome *text* and a list:\n\n- one\n- two\n").unwrap();
	let html = dir.join("page.html");
	fs::write(&html, "<html><body><h1>Hi</h1><p>A <b>bold</b> word.</p></body></html>").unwrap();
	let rtf = dir.join("letter.rtf");
	fs::write(&rtf, r"{\rtf1\ansi Dear friend,\par Thanks!\par}").unwrap();
	let rs = dir.join("main.rs");
	fs::write(&rs, "fn main() {\n\tlet x = 1;\n\tprintln!(\"{x}\");\n}\n").unwrap();
	let css = dir.join("site.css");
	fs::write(&css, "body { color : red ; }\n/* note */\na{margin:0 0 0 0}\n").unwrap();
	let folder = dir.join("album");
	fs::create_dir_all(folder.join("inner")).unwrap();
	fs::write(folder.join("a.txt"), "alpha").unwrap();
	fs::write(folder.join("inner").join("b.txt"), "beta").unwrap();
	let zip = dir.join("album.zip");
	tapwheel_core::archive::pack(
		std::slice::from_ref(&folder),
		Format::Zip,
		&zip,
		&AtomicBool::new(false),
	)
	.unwrap();
	let mut all = vec![
		png, jpg, gif, wav, mp4, svg, srt, csv, json, md, html, rtf, rs, css, folder, zip,
	];
	// HEIC needs a newer FFmpeg than some test machines have
	let heic = dir.join("photo.heic");
	fs::copy(
		Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/photo.heic"),
		&heic,
	)
	.unwrap();
	if decodes(ff, &heic) {
		all.push(heic);
	}
	all
}

/// Skipped where the engine isn't here: speech needs Windows, the cut-out needs its model.
fn available(op: Op, e: &Engines) -> bool {
	match op {
		Op::ReadAloud => cfg!(windows),
		Op::RemoveBackground => e.model.exists() && e.onnxruntime.exists(),
		_ => true,
	}
}

fn check_output(e: &Engines, out: &Path, label: &str) {
	assert!(out.exists(), "{label}: nothing at {}", out.display());
	if out.is_dir() {
		assert!(fs::read_dir(out).unwrap().next().is_some(), "{label}: empty folder");
		return;
	}
	let len = fs::metadata(out).unwrap().len();
	assert!(len > 0, "{label}: empty file");
	let ext = tapwheel_core::kind::ext_of(out);
	let media = matches!(
		tapwheel_core::kind::kind_of_ext(&ext),
		Kind::Image | Kind::Video | Kind::Audio
	) && ext != "m4r";
	// older FFmpeg can't read animated WebP back, so those are checked by their structure
	let animated_webp = ext == "webp" && fs::read(out).unwrap().windows(4).any(|w| w == b"ANIM");
	if media && ext != "ico" && !animated_webp {
		assert!(decodes(&e.ffmpeg, out), "{label}: {} doesn't decode", out.display());
	}
}

#[test]
fn every_slice_on_every_wheel_works() {
	let Some((e, dir)) = engines("all") else {
		eprintln!("TAPWHEEL_FFMPEG not set, skipping");
		return;
	};
	let d = Defaults::default();
	let never = AtomicBool::new(false);
	let mut ran = 0;
	for input in fixtures(&e, &dir) {
		let kind = tapwheel_core::ops::selection_kind(std::slice::from_ref(&input));
		for wheel in [Wheel::Convert, Wheel::Tools] {
			for op in choices(wheel, std::slice::from_ref(&input)) {
				if !available(op, &e) {
					continue;
				}
				let presets: Vec<Option<&str>> = match op.presets(kind) {
					[] => vec![None],
					ps => ps.iter().map(|p| Some(p.id)).collect(),
				};
				for preset in presets {
					let Some(recipe) = recipe_for(op, preset, kind, &d) else {
						continue;
					};
					let label = format!("{} {op:?} {preset:?}", input.display());
					for job in plan(op, &recipe, std::slice::from_ref(&input)) {
						let out = run(&job, &e, &d, &mut |_| {}, &never).unwrap_or_else(|err| panic!("{label}: {err}"));
						if out.note.is_none() {
							check_output(&e, &out.output, &label);
						}
						ran += 1;
					}
				}
			}
		}
	}
	assert!(ran > 150, "only {ran} jobs ran");
}

#[test]
fn phone_photos_come_out_upright_with_their_details() {
	let Some((e, dir)) = engines("upright") else { return };
	let jpg = sideways_jpeg(&e.ffmpeg, &dir);
	let d = Defaults::default();
	let job = plan(
		Op::Convert(Format::Png),
		&Recipe::Convert { to: Format::Png },
		std::slice::from_ref(&jpg),
	)
	.remove(0);
	let out = run(&job, &e, &d, &mut |_| {}, &AtomicBool::new(false)).unwrap();
	assert_eq!(size(&e, &out.output), (100, 200));
	let bytes = fs::read(&out.output).unwrap();
	assert!(
		tapwheel_core::metadata::exif_range(&bytes).is_some(),
		"details carried over"
	);
	assert_eq!(tapwheel_core::metadata::orientation(&bytes), 1, "and marked upright");
}

#[test]
fn trims_cut_keep_and_split() {
	let Some((e, dir)) = engines("trim") else { return };
	let mp4 = fixtures(&e, &dir)
		.into_iter()
		.find(|p| p.ends_with("clip.mp4"))
		.unwrap();
	let d = Defaults::default();
	let never = AtomicBool::new(false);
	let spans = vec![Span { start: 0.0, end: 1.0 }, Span { start: 2.0, end: 3.0 }];
	for (separate, exact) in [(false, true), (true, false), (true, true)] {
		let r = Recipe::Trim {
			keep: spans.clone(),
			separate,
			exact,
		};
		let job = plan(Op::Trim, &r, std::slice::from_ref(&mp4)).remove(0);
		let out = run(&job, &e, &d, &mut |_| {}, &never).unwrap();
		if separate {
			assert_eq!(fs::read_dir(&out.output).unwrap().count(), 2);
		} else {
			let p = tapwheel_core::ff::probe(&e.ffprobe, &out.output).unwrap();
			assert!((p.duration - 2.0).abs() < 0.3, "kept {}", p.duration);
		}
	}
}

#[test]
fn compress_hits_a_target_size() {
	let Some((e, dir)) = engines("target") else { return };
	let mp4 = dir.join("big.mp4");
	make(
		&e.ffmpeg,
		&[
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=640x360:d=6:r=30",
			"-c:v",
			"libx264",
			"-crf",
			"8",
			"-pix_fmt",
			"yuv420p",
		],
		&mp4,
	);
	let r = Recipe::CompressVideo {
		crf: 23,
		max_height: None,
		target_mb: Some(0.25),
	};
	let job = plan(Op::Compress, &r, std::slice::from_ref(&mp4)).remove(0);
	let out = run(&job, &e, &Defaults::default(), &mut |_| {}, &AtomicBool::new(false)).unwrap();
	assert!(
		out.bytes_out <= (0.25 * 1024.0 * 1024.0 * 1.08) as u64,
		"{} bytes",
		out.bytes_out
	);

	let jpg = sideways_jpeg(&e.ffmpeg, &dir);
	let r = Recipe::CompressImage {
		quality: 90,
		max_edge: None,
		target_kb: Some(4),
	};
	let job = plan(Op::Compress, &r, std::slice::from_ref(&jpg)).remove(0);
	let out = run(&job, &e, &Defaults::default(), &mut |_| {}, &AtomicBool::new(false)).unwrap();
	assert!(out.bytes_out <= 4 * 1024, "{} bytes", out.bytes_out);
}

#[test]
fn many_files_join_merge_and_combine() {
	let Some((e, dir)) = engines("many") else { return };
	let files = fixtures(&e, &dir);
	let find = |n: &str| files.iter().find(|p| p.ends_with(n)).unwrap().clone();
	let d = Defaults::default();
	let never = AtomicBool::new(false);
	let mp4 = find("clip.mp4");
	let other = dir.join("other.mp4");
	make(
		&e.ffmpeg,
		&[
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=200x200:d=1:r=25",
			"-f",
			"lavfi",
			"-i",
			"sine=d=1",
			"-shortest",
		],
		&other,
	);
	let jobs = plan(Op::Join, &Recipe::Join, &[mp4, other]);
	let out = run(&jobs[0], &e, &d, &mut |_| {}, &never).unwrap();
	let p = tapwheel_core::ff::probe(&e.ffprobe, &out.output).unwrap();
	assert!((p.duration - 4.0).abs() < 0.3, "{}", p.duration);
	assert_eq!((p.width, p.height), (322, 182));

	let csv2 = dir.join("more.csv");
	fs::write(&csv2, "name,age,city\nbo,40,Rome\n").unwrap();
	let jobs = plan(Op::Merge, &Recipe::Merge, &[find("people.csv"), csv2]);
	let out = run(&jobs[0], &e, &d, &mut |_| {}, &never).unwrap();
	let text = fs::read_to_string(&out.output).unwrap();
	assert_eq!(text.matches("name,age,city").count(), 1);
	assert!(text.contains("bo,40,Rome"));

	for layout in [Layout::Grid, Layout::Row, Layout::Column, Layout::Gif] {
		let r = Recipe::Combine {
			layout,
			gap: 8,
			background: "#ffffff".into(),
		};
		let jobs = plan(Op::Combine, &r, &[find("alpha.png"), find("phone.jpg")]);
		let out = run(&jobs[0], &e, &d, &mut |_| {}, &never).unwrap();
		assert!(decodes(&e.ffmpeg, &out.output), "{layout:?}");
	}
}

#[test]
fn cancel_stops_and_leaves_nothing() {
	let Some((e, dir)) = engines("cancel") else { return };
	let src = dir.join("long.mp4");
	make(
		&e.ffmpeg,
		&[
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=1280x720:d=60:r=30",
			"-c:v",
			"libx264",
			"-preset",
			"ultrafast",
		],
		&src,
	);
	let job = plan(
		Op::Convert(Format::Webm),
		&Recipe::Convert { to: Format::Webm },
		std::slice::from_ref(&src),
	)
	.remove(0);
	let stop = AtomicBool::new(false);
	let err = run(
		&job,
		&e,
		&Defaults::default(),
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
	let left = fs::read_dir(&dir)
		.unwrap()
		.flatten()
		.filter(|f| f.path().to_string_lossy().ends_with(".tw-part"));
	assert_eq!(left.count(), 0);
}

#[test]
fn an_existing_file_is_never_replaced() {
	let Some((e, dir)) = engines("keep") else { return };
	let src = dir.join("a.wav");
	make(&e.ffmpeg, &["-f", "lavfi", "-i", "sine=d=1"], &src);
	let job = plan(
		Op::Convert(Format::Mp3),
		&Recipe::Convert { to: Format::Mp3 },
		std::slice::from_ref(&src),
	)
	.remove(0);
	// someone saves a file with the same name while the job runs
	fs::write(&job.output, b"mine").unwrap();
	let out = run(&job, &e, &Defaults::default(), &mut |_| {}, &AtomicBool::new(false)).unwrap();
	assert_eq!(fs::read(&job.output).unwrap(), b"mine");
	assert_ne!(out.output, job.output);
}

#[test]
fn background_comes_off() {
	let Some((e, dir)) = engines("cutout") else { return };
	if !e.model.exists() {
		eprintln!("TAPWHEEL_MODEL_DIR not set, skipping");
		return;
	}
	let photo = match std::env::var_os("TAPWHEEL_BG_SAMPLE") {
		Some(p) => PathBuf::from(p),
		None => Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/photo.heic"),
	};
	let input = dir.join(photo.file_name().unwrap());
	fs::copy(&photo, &input).unwrap();
	let job = plan(
		Op::RemoveBackground,
		&Recipe::RemoveBackground,
		std::slice::from_ref(&input),
	)
	.remove(0);
	let started = std::time::Instant::now();
	let out = run(&job, &e, &Defaults::default(), &mut |_| {}, &AtomicBool::new(false)).unwrap();
	eprintln!("cut out in {:?}", started.elapsed());
	let (_, _, rgba) = tapwheel_core::image::read_png(&out.output).unwrap();
	let alphas: Vec<u8> = rgba.as_chunks::<4>().0.iter().map(|p| p[3]).collect();
	let clear = alphas.iter().filter(|a| **a == 0).count() as f32 / alphas.len() as f32;
	let solid = alphas.iter().filter(|a| **a == 255).count() as f32 / alphas.len() as f32;
	assert!(clear > 0.05 && solid > 0.05, "clear {clear}, solid {solid}");
}

#[test]
fn a_redacted_photo_keeps_neither_location_nor_the_old_thumbnail() {
	let Some((e, dir)) = engines("redact") else { return };
	let plain = dir.join("plain.jpg");
	make(
		&e.ffmpeg,
		&["-f", "lavfi", "-i", "testsrc2=s=200x100:d=1", "-frames:v", "1"],
		&plain,
	);
	// EXIF with a GPS latitude reference in IFD0's GPS directory, and IFD1 holding a thumbnail
	let thumb = [0xff, 0xd8, 0x5a, 0x5b, 0x5c, 0x5d, 0xff, 0xd9];
	let mut t = b"II*\0\x08\0\0\0".to_vec();
	t.extend(1u16.to_le_bytes());
	t.extend(0x8825u16.to_le_bytes());
	t.extend(4u16.to_le_bytes());
	t.extend(1u32.to_le_bytes());
	t.extend(26u32.to_le_bytes());
	t.extend(44u32.to_le_bytes());
	// GPS directory at 26
	t.extend(1u16.to_le_bytes());
	t.extend(1u16.to_le_bytes());
	t.extend(2u16.to_le_bytes());
	t.extend(2u32.to_le_bytes());
	t.extend(b"N\0\0\0");
	t.extend(0u32.to_le_bytes());
	// IFD1 at 44 with the thumbnail's offset and length, thumbnail at 74
	t.extend(2u16.to_le_bytes());
	t.extend(0x0201u16.to_le_bytes());
	t.extend(4u16.to_le_bytes());
	t.extend(1u32.to_le_bytes());
	t.extend(74u32.to_le_bytes());
	t.extend(0x0202u16.to_le_bytes());
	t.extend(4u16.to_le_bytes());
	t.extend(1u32.to_le_bytes());
	t.extend((thumb.len() as u32).to_le_bytes());
	t.extend(0u32.to_le_bytes());
	t.extend(thumb);
	let photo = dir.join("street.jpg");
	fs::write(
		&photo,
		tapwheel_core::metadata::insert_exif(&fs::read(&plain).unwrap(), &t).unwrap(),
	)
	.unwrap();
	assert!(tapwheel_core::metadata::has_location(&fs::read(&photo).unwrap()));
	// what the Redact window hands back: its drawing as a PNG
	let drawn = dir.join("drawn.png");
	make(
		&e.ffmpeg,
		&["-f", "lavfi", "-i", "color=black:s=200x100:d=1", "-frames:v", "1"],
		&drawn,
	);
	let recipe = Recipe::Rendered {
		file: drawn,
		suffix: "redacted".into(),
		to: None,
	};
	let job = plan(Op::Redact, &recipe, std::slice::from_ref(&photo)).remove(0);
	let out = run(&job, &e, &Defaults::default(), &mut |_| {}, &AtomicBool::new(false)).unwrap();
	let bytes = fs::read(&out.output).unwrap();
	assert!(!tapwheel_core::metadata::has_location(&bytes));
	assert!(
		!bytes.windows(thumb.len()).any(|w| w == thumb),
		"the unredacted thumbnail is still inside"
	);
}

#[test]
fn icons_made_at_the_same_time_keep_their_own_picture() {
	let Some((e, dir)) = engines("icons-together") else {
		return;
	};
	let mut icos = Vec::new();
	for colour in ["red", "blue"] {
		let ico = dir.join(format!("{colour}.ico"));
		make(
			&e.ffmpeg,
			&[
				"-f",
				"lavfi",
				"-i",
				&format!("color={colour}:s=64x64:d=1"),
				"-frames:v",
				"1",
			],
			&ico,
		);
		icos.push(ico);
	}
	let red = |px: &[u8]| px[0] > 200 && px[2] < 60;
	for round in 0..4 {
		let jobs: Vec<_> = icos
			.iter()
			.map(|i| plan(Op::WindowsIcon, &Recipe::WindowsIcon, std::slice::from_ref(i)).remove(0))
			.collect();
		let outs: Vec<_> = std::thread::scope(|s| {
			let handles: Vec<_> = jobs
				.iter()
				.map(|job| s.spawn(|| run(job, &e, &Defaults::default(), &mut |_| {}, &AtomicBool::new(false))))
				.collect();
			handles.into_iter().map(|h| h.join().unwrap()).collect()
		});
		for (out, want_red) in outs.into_iter().zip([true, false]) {
			let out = out.unwrap_or_else(|err| panic!("round {round}: {err}"));
			for (w, _, px) in tapwheel_core::icons::read_icon_file(&out.output).unwrap() {
				assert_eq!(
					red(&px[..4]),
					want_red,
					"round {round}, {w} px came from the other icon"
				);
			}
			fs::remove_file(&out.output).unwrap();
		}
	}
}
