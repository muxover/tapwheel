use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crate::Error;
use crate::plan::{Job, Work, part_path, stem_of, unique_path};
use crate::{archive, pngquant, subs};

pub struct Engines {
	pub ffmpeg: PathBuf,
	/// Scratch space for archive repacking.
	pub scratch: PathBuf,
}

#[derive(Debug)]
pub struct Outcome {
	pub output: PathBuf,
	pub bytes_in: u64,
	pub bytes_out: u64,
}

fn size_of(path: &Path) -> u64 {
	match fs::symlink_metadata(path) {
		Ok(m) if m.is_dir() => fs::read_dir(path)
			.map(|rd| rd.flatten().map(|e| size_of(&e.path())).sum())
			.unwrap_or(0),
		Ok(m) => m.len(),
		Err(_) => 0,
	}
}

/// Runs a job into a `.tw-part` file, then gives it its final name.
/// The final name is re-checked at the end, so a file that appeared meanwhile is never replaced.
pub fn run(job: &Job, engines: &Engines, progress: &mut dyn FnMut(f32), cancel: &AtomicBool) -> Result<Outcome, Error> {
	let part = part_path(&job.output);
	let result = execute(job, &part, engines, progress, cancel);
	if let Err(e) = result {
		remove(&part);
		return Err(e);
	}
	let output = if job.output.exists() {
		let dir = job.output.parent().unwrap_or(Path::new(""));
		let ext = if part.is_dir() {
			String::new()
		} else {
			let name = job
				.output
				.file_name()
				.unwrap_or_default()
				.to_string_lossy()
				.into_owned();
			let stem = stem_of(&job.output);
			name.strip_prefix(&format!("{stem}.")).unwrap_or("").to_string()
		};
		unique_path(dir, &stem_of(&job.output), &ext)
	} else {
		job.output.clone()
	};
	if let Err(e) = fs::rename(&part, &output) {
		remove(&part);
		return Err(e.into());
	}
	keep_date(job, &output);
	let bytes_in = job.inputs().iter().map(|p| size_of(p)).sum();
	Ok(Outcome {
		bytes_out: size_of(&output),
		output,
		bytes_in,
	})
}

/// A converted photo keeps the date it was taken, so it sorts next to the original.
fn keep_date(job: &Job, output: &Path) {
	let [input] = job.inputs()[..] else { return };
	if output.is_dir() {
		return;
	}
	if let Ok(time) = fs::metadata(input).and_then(|m| m.modified())
		&& let Ok(f) = fs::File::options().write(true).open(output)
	{
		let _ = f.set_modified(time);
	}
}

fn remove(path: &Path) {
	let _ = if path.is_dir() {
		fs::remove_dir_all(path)
	} else {
		fs::remove_file(path)
	};
}

fn execute(
	job: &Job,
	part: &Path,
	engines: &Engines,
	progress: &mut dyn FnMut(f32),
	cancel: &AtomicBool,
) -> Result<(), Error> {
	match &job.work {
		Work::Ffmpeg { input, args } => ffmpeg(&engines.ffmpeg, input, args, part, progress, cancel),
		Work::PngQuant { input } => pngquant::compress(input, part),
		Work::Subtitle { input, to } => {
			let text = fs::read(input)?;
			let text = String::from_utf8_lossy(&text);
			fs::write(part, subs::convert(&text, *to))?;
			Ok(())
		}
		Work::Pack { inputs, format } => archive::pack(inputs, *format, part, cancel),
		Work::Extract { input } => archive::extract(input, part, cancel),
		Work::Repack { input, format } => {
			let scratch = engines.scratch.join(format!(
				"repack-{}-{}",
				std::process::id(),
				part.file_name().unwrap_or_default().to_string_lossy()
			));
			let result = archive::repack(input, *format, part, &scratch, cancel);
			let _ = fs::remove_dir_all(&scratch);
			result
		}
	}
}

/// `HH:MM:SS.ss` to seconds.
fn parse_clock(s: &str) -> Option<f64> {
	let mut total = 0.0;
	for part in s.trim().split(':') {
		total = total * 60.0 + part.parse::<f64>().ok()?;
	}
	Some(total)
}

fn duration_in(line: &str) -> Option<f64> {
	let rest = line.trim().strip_prefix("Duration:")?;
	let clock = rest.split(',').next()?.trim();
	parse_clock(clock).filter(|d| *d > 0.0)
}

/// The most useful line of ffmpeg's complaint, without the noise.
fn reason(lines: &VecDeque<String>) -> String {
	let skip = [
		"Conversion failed",
		"Error while",
		"Error opening output",
		"Exiting normally",
	];
	lines
		.iter()
		.rev()
		.map(|l| l.trim())
		.find(|l| !l.is_empty() && !skip.iter().any(|s| l.starts_with(s)))
		.map(|l| {
			// "[mjpeg @ 0x1234] message" reads better without the prefix
			match l.find("] ") {
				Some(i) if l.starts_with('[') => l[i + 2..].to_string(),
				_ => l.to_string(),
			}
		})
		.unwrap_or_else(|| "the converter stopped without saying why".into())
}

enum Line {
	Out(String),
	Err(String),
}

pub fn ffmpeg(
	exe: &Path,
	input: &Path,
	args: &[String],
	output: &Path,
	progress: &mut dyn FnMut(f32),
	cancel: &AtomicBool,
) -> Result<(), Error> {
	let mut cmd = Command::new(exe);
	cmd.args(["-hide_banner", "-nostdin", "-y", "-loglevel", "info", "-i"])
		.arg(input)
		.args(args)
		.args(["-progress", "pipe:1", "-nostats"])
		.arg(output)
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped());
	#[cfg(windows)]
	{
		use std::os::windows::process::CommandExt;
		const CREATE_NO_WINDOW: u32 = 0x0800_0000;
		const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
		cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
	}
	let mut child = cmd.spawn().map_err(|e| {
		if e.kind() == std::io::ErrorKind::NotFound {
			Error::Engine("the converter is missing; reinstall TapWheel".into())
		} else {
			Error::Io(e)
		}
	})?;

	let (tx, rx) = mpsc::channel();
	let out = child.stdout.take().expect("piped");
	let err = child.stderr.take().expect("piped");
	let tx2 = tx.clone();
	let t1 = std::thread::spawn(move || {
		for l in BufReader::new(out).lines().map_while(Result::ok) {
			if tx.send(Line::Out(l)).is_err() {
				break;
			}
		}
	});
	let t2 = std::thread::spawn(move || {
		for l in BufReader::new(err).lines().map_while(Result::ok) {
			if tx2.send(Line::Err(l)).is_err() {
				break;
			}
		}
	});

	let mut duration = None;
	let mut tail: VecDeque<String> = VecDeque::new();
	let mut cancelled = false;
	loop {
		if cancel.load(Ordering::Relaxed) {
			let _ = child.kill();
			cancelled = true;
			break;
		}
		match rx.recv_timeout(Duration::from_millis(100)) {
			Ok(Line::Err(l)) => {
				if duration.is_none() {
					duration = duration_in(&l);
				}
				tail.push_back(l);
				if tail.len() > 40 {
					tail.pop_front();
				}
			}
			Ok(Line::Out(l)) => {
				if let (Some(total), Some(v)) = (duration, l.strip_prefix("out_time_us="))
					&& let Ok(us) = v.trim().parse::<f64>()
				{
					progress((us / 1e6 / total).clamp(0.0, 1.0) as f32);
				}
			}
			Err(mpsc::RecvTimeoutError::Timeout) => {}
			Err(mpsc::RecvTimeoutError::Disconnected) => break,
		}
	}
	let status = child.wait()?;
	let _ = t1.join();
	let _ = t2.join();
	if cancelled {
		return Err(Error::Cancelled);
	}
	if !status.success() {
		return Err(Error::Engine(reason(&tail)));
	}
	if !output.exists() {
		return Err(Error::Engine("nothing was written".into()));
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clock_and_duration() {
		assert_eq!(parse_clock("00:01:02.50"), Some(62.5));
		assert_eq!(
			duration_in("  Duration: 00:00:05.00, start: 0.000000, bitrate: 2 kb/s"),
			Some(5.0)
		);
		assert_eq!(duration_in("  Duration: N/A, bitrate: N/A"), None);
		assert_eq!(duration_in("Stream #0:0"), None);
	}

	#[test]
	fn reason_skips_boilerplate() {
		let lines: VecDeque<String> = [
			"[mjpeg @ 0x55] Invalid pixel format",
			"Error while opening encoder",
			"Conversion failed!",
		]
		.iter()
		.map(|s| s.to_string())
		.collect();
		assert_eq!(reason(&lines), "Invalid pixel format");
	}
}
