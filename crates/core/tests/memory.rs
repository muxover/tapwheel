//! Peak memory of the FFmpeg runs behind Make GIF and Reverse, which once held every frame of
//! what they worked on. Linux only, where the system reports the largest child's peak; it runs
//! alone in this file so no other test's FFmpeg counts. Needs TAPWHEEL_FFMPEG.
#![cfg(target_os = "linux")]

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicBool;

use tapwheel_core::recipe::Recipe;
use tapwheel_core::run::{Engines, run};
use tapwheel_core::{Defaults, Op, plan};

/// The largest peak resident size of any child this process has waited for, in MB.
fn peak_child_mb() -> u64 {
	let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
	unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut ru) };
	ru.ru_maxrss as u64 / 1024
}

#[test]
fn make_gif_and_reverse_stream_instead_of_holding_every_frame() {
	let Some(ff) = std::env::var_os("TAPWHEEL_FFMPEG").map(PathBuf::from) else {
		return;
	};
	let dir = std::env::temp_dir().join(format!("tw-memory-{}", std::process::id()));
	let _ = std::fs::remove_dir_all(&dir);
	std::fs::create_dir_all(&dir).unwrap();
	let mut e = Engines::in_dir(ff.parent().unwrap(), dir.join(".scratch"));
	e.ffmpeg = ff.clone();
	// a minute of 640 × 360: holding every frame takes 600 MB to 800 MB, streaming a few dozen
	let video = dir.join("minute.mp4");
	let made = Command::new(&ff)
		.args([
			"-hide_banner",
			"-v",
			"error",
			"-y",
			"-f",
			"lavfi",
			"-i",
			"testsrc2=s=640x360:r=30:d=60",
		])
		.args([
			"-f",
			"lavfi",
			"-i",
			"sine=f=440:d=60",
			"-c:v",
			"libx264",
			"-preset",
			"ultrafast",
		])
		.args(["-g", "30", "-c:a", "aac", "-shortest"])
		.arg(&video)
		.status()
		.unwrap();
	assert!(made.success());
	let setup = peak_child_mb();
	let d = Defaults::default();
	let never = AtomicBool::new(false);

	let gif = Recipe::MakeGif {
		span: None,
		fps: 15,
		width: 640,
	};
	let job = plan(Op::MakeGif, &gif, std::slice::from_ref(&video)).remove(0);
	let out = run(&job, &e, &d, &mut |_| {}, &never).unwrap();
	assert!(std::fs::metadata(&out.output).unwrap().len() > 0);
	let after_gif = peak_child_mb();
	assert!(after_gif < setup.max(300), "Make GIF's FFmpeg peaked at {after_gif} MB");

	let job = plan(Op::Reverse, &Recipe::Reverse, std::slice::from_ref(&video)).remove(0);
	let out = run(&job, &e, &d, &mut |_| {}, &never).unwrap();
	let after_reverse = peak_child_mb();
	assert!(
		after_reverse < setup.max(300),
		"Reverse's FFmpeg peaked at {after_reverse} MB"
	);

	// the pieces join into the whole video, back to front: compared with FFmpeg's own reverse of
	// the original, frame by frame
	let p = tapwheel_core::ff::probe(&e.ffprobe, &out.output).unwrap();
	assert!((p.duration - 60.0).abs() < 0.2, "{}", p.duration);
	let frames = |f: &PathBuf| {
		let counted = Command::new(&e.ffprobe)
			.args(["-v", "error", "-select_streams", "v:0", "-count_packets"])
			.args(["-show_entries", "stream=nb_read_packets", "-of", "csv=p=0"])
			.arg(f)
			.output()
			.unwrap();
		String::from_utf8_lossy(&counted.stdout).trim().to_string()
	};
	assert_eq!(frames(&out.output), frames(&video), "Reverse dropped or added frames");
	// the pieces are Matroska, whose timestamps are whole milliseconds, so frames are paired by
	// their number: paired by time, every third frame would meet its neighbour
	let compared = Command::new(&ff)
		.args(["-hide_banner", "-nostats", "-i"])
		.arg(&out.output)
		.arg("-i")
		.arg(&video)
		.args([
			"-lavfi",
			"[0:v]settb=1/30,setpts=N[o];[1:v]reverse,settb=1/30,setpts=N[r];[o][r]psnr",
			"-f",
			"null",
			"-",
		])
		.output()
		.unwrap();
	let log = String::from_utf8_lossy(&compared.stderr);
	let average: f64 = log
		.split("average:")
		.nth(1)
		.and_then(|s| s.split_whitespace().next())
		.and_then(|s| s.parse().ok())
		.unwrap_or_else(|| panic!("no PSNR in {log}"));
	assert!(
		average > 30.0,
		"reversed frames match the original at only {average} dB"
	);
	let _ = std::fs::remove_dir_all(dir);
}
