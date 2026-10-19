use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde::Deserialize;

use crate::Error;
use crate::format::Format;
use crate::recipe::Turn;

pub fn command(exe: &Path) -> Command {
	#[allow(unused_mut)]
	let mut cmd = Command::new(exe);
	#[cfg(windows)]
	{
		use std::os::windows::process::CommandExt;
		const CREATE_NO_WINDOW: u32 = 0x0800_0000;
		const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
		cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
	}
	cmd
}

/// Starts a converter. On Windows it joins a job that ends with TapWheel, so a crash or Quit
/// never leaves FFmpeg running on its own.
pub fn spawn(cmd: &mut Command) -> std::io::Result<Child> {
	let child = cmd.spawn()?;
	#[cfg(windows)]
	job::add(&child);
	Ok(child)
}

/// [`spawn`], then everything it prints.
pub fn output(cmd: &mut Command) -> std::io::Result<Output> {
	spawn(cmd.stdout(Stdio::piped()).stderr(Stdio::piped()))?.wait_with_output()
}

#[cfg(windows)]
pub mod job {
	use std::os::windows::io::AsRawHandle;
	use std::process::Child;
	use std::sync::OnceLock;

	use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
	use windows_sys::Win32::System::JobObjects::{
		AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
		JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
	};

	/// The job's handle, kept open until the process ends; closing it ends every process in it.
	static JOB: OnceLock<Option<usize>> = OnceLock::new();

	pub fn handle() -> Option<HANDLE> {
		let h = JOB.get_or_init(|| unsafe {
			let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
			if h.is_null() {
				return None;
			}
			let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
			info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
			let ok = SetInformationJobObject(
				h,
				JobObjectExtendedLimitInformation,
				&info as *const _ as *const core::ffi::c_void,
				std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
			);
			if ok == 0 {
				CloseHandle(h);
				return None;
			}
			Some(h as usize)
		});
		h.map(|h| h as HANDLE)
	}

	pub fn add(child: &Child) {
		if let Some(job) = handle() {
			unsafe { AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) };
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use windows_sys::Win32::System::JobObjects::IsProcessInJob;

		#[test]
		fn converters_end_with_the_app() {
			let mut child = super::super::spawn(
				std::process::Command::new("cmd")
					.args(["/c", "ping", "-n", "5", "127.0.0.1"])
					.stdout(std::process::Stdio::null()),
			)
			.unwrap();
			let mut inside = 0;
			unsafe { IsProcessInJob(child.as_raw_handle() as HANDLE, handle().unwrap(), &mut inside) };
			let _ = child.kill();
			assert_ne!(inside, 0);
		}
	}
}

fn missing(e: std::io::Error) -> Error {
	if e.kind() == std::io::ErrorKind::NotFound {
		Error::Engine("the converter is missing; reinstall TapWheel".into())
	} else {
		Error::Io(e)
	}
}

/// What FFprobe says about a file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Probe {
	pub duration: f64,
	pub width: u32,
	pub height: u32,
	pub fps: f64,
	pub video_codec: Option<String>,
	pub audio_codec: Option<String>,
	pub audio_kbps: Option<u32>,
	/// Degrees the picture is shown turned, from the file's own display matrix.
	pub rotation: i32,
	pub frames: Option<u64>,
}

impl Probe {
	pub fn has_video(&self) -> bool {
		self.video_codec.is_some()
	}

	pub fn has_audio(&self) -> bool {
		self.audio_codec.is_some()
	}

	/// Width and height as the picture is shown.
	pub fn shown_size(&self) -> (u32, u32) {
		if self.rotation.rem_euclid(180) == 90 {
			(self.height, self.width)
		} else {
			(self.width, self.height)
		}
	}
}

#[derive(Deserialize)]
struct RawProbe {
	#[serde(default)]
	streams: Vec<RawStream>,
	format: Option<RawFormat>,
}

#[derive(Deserialize)]
struct RawFormat {
	duration: Option<String>,
}

#[derive(Deserialize)]
struct RawStream {
	codec_type: Option<String>,
	codec_name: Option<String>,
	width: Option<u32>,
	height: Option<u32>,
	avg_frame_rate: Option<String>,
	r_frame_rate: Option<String>,
	duration: Option<String>,
	bit_rate: Option<String>,
	nb_frames: Option<String>,
	#[serde(default)]
	side_data_list: Vec<serde_json::Value>,
	#[serde(default)]
	tags: serde_json::Map<String, serde_json::Value>,
	#[serde(default)]
	disposition: serde_json::Map<String, serde_json::Value>,
}

fn rate(s: &str) -> f64 {
	match s.split_once('/') {
		Some((a, b)) => {
			let (a, b) = (a.parse::<f64>().unwrap_or(0.0), b.parse::<f64>().unwrap_or(0.0));
			if b > 0.0 { a / b } else { 0.0 }
		}
		None => s.parse().unwrap_or(0.0),
	}
}

pub fn parse_probe(json: &str) -> Option<Probe> {
	let raw: RawProbe = serde_json::from_str(json).ok()?;
	let mut p = Probe {
		duration: raw
			.format
			.and_then(|f| f.duration)
			.and_then(|d| d.parse().ok())
			.unwrap_or(0.0),
		..Probe::default()
	};
	for s in raw.streams {
		let cover = s.disposition.get("attached_pic").and_then(|v| v.as_i64()) == Some(1);
		match s.codec_type.as_deref() {
			Some("video") if p.video_codec.is_none() && !cover => {
				p.video_codec = s.codec_name;
				p.width = s.width.unwrap_or(0);
				p.height = s.height.unwrap_or(0);
				let fps = s.avg_frame_rate.as_deref().map(rate).unwrap_or(0.0);
				p.fps = if fps > 0.0 {
					fps
				} else {
					s.r_frame_rate.as_deref().map(rate).unwrap_or(0.0)
				};
				p.frames = s.nb_frames.and_then(|n| n.parse().ok());
				let from_side = s
					.side_data_list
					.iter()
					.find_map(|d| d.get("rotation").and_then(|r| r.as_f64()));
				let from_tag = s
					.tags
					.get("rotate")
					.and_then(|r| r.as_str())
					.and_then(|r| r.parse().ok());
				p.rotation = from_side.or(from_tag).unwrap_or(0.0).round() as i32;
				if p.duration <= 0.0 {
					p.duration = s.duration.and_then(|d| d.parse().ok()).unwrap_or(0.0);
				}
			}
			Some("audio") if p.audio_codec.is_none() => {
				p.audio_codec = s.codec_name;
				p.audio_kbps = s.bit_rate.and_then(|b| b.parse::<u32>().ok()).map(|b| b / 1000);
				if p.duration <= 0.0 {
					p.duration = s.duration.and_then(|d| d.parse().ok()).unwrap_or(0.0);
				}
			}
			_ => {}
		}
	}
	Some(p)
}

pub fn probe(ffprobe: &Path, input: &Path) -> Result<Probe, Error> {
	let out = output(
		command(ffprobe)
			.args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
			.arg(input)
			.stdin(Stdio::null()),
	)
	.map_err(missing)?;
	if !out.status.success() {
		let why = String::from_utf8_lossy(&out.stderr);
		let line = why.lines().last().unwrap_or("this file can't be read").trim();
		return Err(Error::Engine(line.to_string()));
	}
	parse_probe(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| Error::Engine("this file can't be read".into()))
}

/// A file's tags (title, artist and so on) as FFprobe reads them, plus whether it has cover art.
pub fn tags(ffprobe: &Path, input: &Path) -> Result<(Vec<(String, String)>, bool), Error> {
	let out = output(
		command(ffprobe)
			.args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
			.arg(input)
			.stdin(Stdio::null()),
	)
	.map_err(missing)?;
	let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_default();
	let tags = v["format"]["tags"]
		.as_object()
		.map(|m| {
			m.iter()
				.map(|(k, v)| (k.to_lowercase(), v.as_str().unwrap_or_default().to_string()))
				.collect()
		})
		.unwrap_or_default();
	let cover = v["streams"]
		.as_array()
		.is_some_and(|s| s.iter().any(|st| st["disposition"]["attached_pic"].as_i64() == Some(1)));
	Ok((tags, cover))
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
		"[out#",
		"[in#",
		// FFmpeg 8 prints its final summary after the error
		"frame=",
		"size=",
		"video:",
	];
	lines
		.iter()
		.rev()
		.map(|l| l.trim())
		.find(|l| !l.is_empty() && !skip.iter().any(|s| l.starts_with(s)))
		.map(|l| match l.find("] ") {
			// "[mjpeg @ 0x1234] message" reads better without the prefix
			Some(i) if l.starts_with('[') => l[i + 2..].to_string(),
			_ => l.to_string(),
		})
		.unwrap_or_else(|| "the converter stopped without saying why".into())
}

enum Line {
	Out(String),
	Err(String),
}

/// One FFmpeg run. `args` holds everything, inputs included; the output path goes last.
pub struct Run<'a> {
	pub exe: &'a Path,
	pub args: Vec<OsString>,
	/// Known length of what's being written, for progress. Read from the log when `None`.
	pub duration: Option<f64>,
	/// Where this run sits in a multi-pass job, as (offset, share) of the whole bar.
	pub span: (f32, f32),
}

impl<'a> Run<'a> {
	pub fn new(exe: &'a Path) -> Self {
		Run {
			exe,
			args: Vec::new(),
			duration: None,
			span: (0.0, 1.0),
		}
	}

	pub fn arg(mut self, a: impl Into<OsString>) -> Self {
		self.args.push(a.into());
		self
	}

	pub fn args<I, S>(mut self, a: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<OsString>,
	{
		self.args.extend(a.into_iter().map(Into::into));
		self
	}

	pub fn input(self, path: &Path) -> Self {
		self.arg("-i").arg(path)
	}

	pub fn duration(mut self, d: f64) -> Self {
		if d > 0.0 {
			self.duration = Some(d);
		}
		self
	}

	pub fn span(mut self, offset: f32, share: f32) -> Self {
		self.span = (offset, share);
		self
	}

	/// Runs to completion, returning FFmpeg's log for recipes that read it (silence, loudness).
	pub fn go(self, progress: &mut dyn FnMut(f32), cancel: &AtomicBool) -> Result<Vec<String>, Error> {
		let mut cmd = command(self.exe);
		cmd.args(["-hide_banner", "-nostdin", "-y", "-loglevel", "info"])
			.args(&self.args)
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped());
		let mut child = spawn(&mut cmd).map_err(missing)?;

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

		let mut duration = self.duration;
		let mut log = Vec::new();
		let mut tail: VecDeque<String> = VecDeque::new();
		let mut cancelled = false;
		let (offset, share) = self.span;
		loop {
			if cancel.load(Ordering::Relaxed) {
				let _ = child.kill();
				cancelled = true;
				break;
			}
			match rx.recv_timeout(Duration::from_millis(80)) {
				Ok(Line::Err(l)) => {
					if duration.is_none() {
						duration = duration_in(&l);
					}
					if log.len() < 20_000 {
						log.push(l.clone());
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
						let f = (us / 1e6 / total).clamp(0.0, 1.0) as f32;
						progress(offset + f * share);
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
		Ok(log)
	}
}

/// Every run reports progress on stdout; callers add `out(path)` at the end.
pub fn out(path: &Path) -> Vec<OsString> {
	vec!["-progress".into(), "pipe:1".into(), "-nostats".into(), path.into()]
}

/// Decodes any picture FFmpeg reads into RGBA8 pixels, through `filter` (the upright turn) when
/// there is one. `size` is the size after the filter.
pub fn decode_rgba(ffmpeg: &Path, input: &Path, size: (u32, u32), filter: &str) -> Result<Vec<u8>, Error> {
	crate::image::check_pixels(size.0, size.1)?;
	let mut cmd = command(ffmpeg);
	cmd.args(["-hide_banner", "-nostdin", "-loglevel", "error"]);
	if !filter.is_empty() {
		// the filter does the turn; FFmpeg mustn't turn it a second time
		cmd.arg("-noautorotate");
	}
	cmd.arg("-i").arg(input);
	if !filter.is_empty() {
		cmd.args(["-vf", filter]);
	}
	let mut child = spawn(
		cmd.args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgba", "-"])
			.stdin(Stdio::null())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped()),
	)
	.map_err(missing)?;
	let mut buf = Vec::with_capacity((size.0 * size.1 * 4) as usize);
	child.stdout.take().expect("piped").read_to_end(&mut buf)?;
	let mut err = String::new();
	let _ = child.stderr.take().expect("piped").read_to_string(&mut err);
	let status = child.wait()?;
	if !status.success() || buf.len() < (size.0 * size.1 * 4) as usize {
		let line = err.lines().last().unwrap_or("this picture can't be read").to_string();
		return Err(Error::Engine(line));
	}
	buf.truncate((size.0 * size.1 * 4) as usize);
	Ok(buf)
}

pub fn strs(s: &[&str]) -> Vec<String> {
	s.iter().map(|s| s.to_string()).collect()
}

// JPG and AVIF have no alpha here, so transparent pixels go onto white instead of black.
pub const ON_WHITE: &str =
	"split[tw_a][tw_b];[tw_a]format=rgb24,drawbox=c=white:t=fill[tw_bg];[tw_bg][tw_b]overlay=format=auto";
pub const EVEN: &str = "scale=trunc(iw/2)*2:trunc(ih/2)*2";

pub fn join_vf(parts: &[&str]) -> String {
	parts
		.iter()
		.filter(|p| !p.is_empty())
		.copied()
		.collect::<Vec<_>>()
		.join(",")
}

pub fn turn_filter(t: Turn) -> &'static str {
	match t {
		Turn::Cw => "transpose=clock",
		Turn::Ccw => "transpose=cclock",
		Turn::Half => "hflip,vflip",
		Turn::FlipH => "hflip",
		Turn::FlipV => "vflip",
	}
}

pub fn muxer(f: Format) -> &'static str {
	match f {
		Format::Jpg | Format::Png | Format::Tiff | Format::Bmp | Format::Tga | Format::Qoi => "image2",
		Format::Webp => "webp",
		Format::Avif => "avif",
		Format::Gif => "gif",
		Format::Ico => "ico",
		Format::Mp3 => "mp3",
		Format::M4a | Format::M4r => "ipod",
		Format::Wav => "wav",
		Format::Flac => "flac",
		Format::Ogg => "ogg",
		Format::Opus => "opus",
		Format::Aiff => "aiff",
		Format::Wma | Format::Wmv => "asf",
		Format::Mp4 => "mp4",
		Format::Mov => "mov",
		Format::Mkv => "matroska",
		Format::Webm => "webm",
		Format::Avi => "avi",
		Format::Srt => "srt",
		Format::Vtt => "webvtt",
		_ => "",
	}
}

/// JPG's 2 to 31 scale from a 1 to 100 quality.
fn jpg_q(quality: u8) -> String {
	(1 + (100 - quality.min(100)) as u32 / 4).clamp(2, 31).to_string()
}

fn avif_crf(quality: u8) -> String {
	((100 - quality.min(100)) as f32 * 0.6 + 10.0)
		.round()
		.clamp(0.0, 63.0)
		.to_string()
}

/// Encoder settings for writing one picture. `filter` runs first; `strip` drops tags.
pub fn image_args(to: Format, filter: &str, quality: u8, strip: bool) -> Vec<String> {
	let (vf, codec): (String, Vec<String>) = match to {
		Format::Jpg => (
			join_vf(&[filter, ON_WHITE, "format=yuvj444p"]),
			vec![
				"-c:v".into(),
				"mjpeg".into(),
				"-q:v".into(),
				jpg_q(quality),
				"-huffman".into(),
				"optimal".into(),
			],
		),
		Format::Png => (join_vf(&[filter]), strs(&["-c:v", "png", "-compression_level", "9"])),
		Format::Webp => (
			join_vf(&[filter]),
			vec!["-c:v".into(), "libwebp".into(), "-quality".into(), quality.to_string()],
		),
		Format::Avif => (
			join_vf(&[filter, ON_WHITE, EVEN, "format=yuv420p"]),
			vec![
				"-c:v".into(),
				"libaom-av1".into(),
				"-still-picture".into(),
				"1".into(),
				"-crf".into(),
				avif_crf(quality),
				"-b:v".into(),
				"0".into(),
				"-cpu-used".into(),
				"6".into(),
				"-row-mt".into(),
				"1".into(),
			],
		),
		Format::Tiff => (join_vf(&[filter]), strs(&["-c:v", "tiff", "-compression_algo", "lzw"])),
		Format::Bmp => (join_vf(&[filter, "format=bgra"]), strs(&["-c:v", "bmp"])),
		Format::Tga => (join_vf(&[filter, "format=bgra"]), strs(&["-c:v", "targa"])),
		Format::Qoi => (join_vf(&[filter, "format=rgba"]), strs(&["-c:v", "qoi"])),
		Format::Gif => (
			join_vf(&[
				filter,
				"split[tw_a][tw_b];[tw_a]palettegen=reserve_transparent=1[tw_p];[tw_b][tw_p]paletteuse=alpha_threshold=128",
			]),
			strs(&["-c:v", "gif"]),
		),
		Format::Ico => (
			join_vf(&[
				filter,
				"scale='min(256,iw)':'min(256,ih)':force_original_aspect_ratio=decrease",
				"format=rgba",
			]),
			strs(&["-c:v", "png"]),
		),
		_ => (String::new(), Vec::new()),
	};
	let mut args = Vec::new();
	if !vf.is_empty() {
		args.extend(strs(&["-vf", &vf]));
	}
	args.extend(codec);
	if strip {
		args.extend(strs(&["-map_metadata", "-1"]));
	}
	args.extend(strs(&["-frames:v", "1"]));
	if muxer(to) == "image2" {
		args.extend(strs(&["-update", "1"]));
	}
	args.extend(strs(&["-f", muxer(to)]));
	args
}

/// Video encoder settings for a container. `crf` is x264's scale; other codecs are mapped from it.
pub fn video_codec_args(to: Format, crf: u8, audio: bool, audio_kbps: u32) -> Vec<String> {
	let a = format!("{audio_kbps}k");
	let mut args = match to {
		Format::Webm => vec![
			"-c:v".into(),
			"libvpx-vp9".into(),
			"-crf".into(),
			(crf as u32 + 10).min(63).to_string(),
			"-b:v".into(),
			"0".into(),
			"-row-mt".into(),
			"1".into(),
			"-deadline".into(),
			"good".into(),
			"-cpu-used".into(),
			"4".into(),
			// a GIF's palette would otherwise pick a format browsers and players can't show
			"-pix_fmt".into(),
			"yuv420p".into(),
		],
		Format::Avi => strs(&["-c:v", "mpeg4", "-q:v", "4", "-vtag", "XVID"]),
		Format::Wmv => strs(&["-c:v", "wmv2", "-q:v", "4"]),
		_ => vec![
			"-c:v".into(),
			"libx264".into(),
			"-preset".into(),
			"medium".into(),
			"-crf".into(),
			crf.to_string(),
			"-pix_fmt".into(),
			"yuv420p".into(),
		],
	};
	if audio {
		args.extend(match to {
			Format::Webm => vec!["-c:a".into(), "libopus".into(), "-b:a".into(), "128k".into()],
			Format::Avi => strs(&["-c:a", "libmp3lame", "-q:a", "3"]),
			Format::Wmv => vec!["-c:a".into(), "wmav2".into(), "-b:a".into(), a],
			_ => vec!["-c:a".into(), "aac".into(), "-b:a".into(), a],
		});
	} else {
		args.push("-an".into());
	}
	if matches!(to, Format::Mp4 | Format::Mov) {
		args.extend(strs(&["-movflags", "+faststart"]));
	}
	args.extend(strs(&["-f", muxer(to)]));
	args
}

/// Audio encoder settings. `kbps` applies to the lossy codecs.
pub fn audio_codec_args(to: Format, kbps: u32) -> Vec<String> {
	let b = format!("{kbps}k");
	let mut args: Vec<String> = match to {
		Format::Mp3 => {
			// VBR for the usual sizes, a fixed rate for anything else
			let q = match kbps {
				0..=80 => "7",
				81..=112 => "5",
				113..=144 => "4",
				145..=176 => "3",
				177..=208 => "2",
				_ => "0",
			};
			strs(&["-c:a", "libmp3lame", "-q:a", q])
		}
		Format::M4a | Format::M4r => vec!["-c:a".into(), "aac".into(), "-b:a".into(), b],
		Format::Wav => strs(&["-c:a", "pcm_s16le"]),
		Format::Aiff => strs(&["-c:a", "pcm_s16be"]),
		Format::Flac => strs(&["-c:a", "flac", "-compression_level", "8"]),
		Format::Ogg => vec!["-c:a".into(), "libvorbis".into(), "-b:a".into(), b],
		Format::Opus => vec![
			"-c:a".into(),
			"libopus".into(),
			"-b:a".into(),
			format!("{}k", kbps.min(256)),
		],
		Format::Wma => vec!["-c:a".into(), "wmav2".into(), "-b:a".into(), b],
		_ => Vec::new(),
	};
	args.extend(strs(&["-f", muxer(to)]));
	args
}

/// `atempo` only takes 0.5 to 2, so bigger changes chain several.
pub fn atempo(factor: f32) -> String {
	let mut f = factor.clamp(0.05, 100.0);
	let mut parts = Vec::new();
	while f > 2.0 {
		parts.push("atempo=2.0".to_string());
		f /= 2.0;
	}
	while f < 0.5 {
		parts.push("atempo=0.5".to_string());
		f /= 0.5;
	}
	parts.push(format!("atempo={f:.4}"));
	parts.join(",")
}

/// Seconds as FFmpeg reads them.
pub fn secs(t: f64) -> String {
	format!("{:.3}", t.max(0.0))
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
	}

	#[test]
	fn reason_skips_boilerplate() {
		let lines: VecDeque<String> = [
			"[mjpeg @ 0x55] Invalid pixel format",
			"Error while opening encoder",
			"Conversion failed!",
			"frame=    0 fps=0.0 q=0.0 Lsize=       0KiB time=N/A bitrate=N/A speed=N/A elapsed=0:00:00.00",
		]
		.iter()
		.map(|s| s.to_string())
		.collect();
		assert_eq!(reason(&lines), "Invalid pixel format");
	}

	#[test]
	fn tempo_chains_past_the_limits() {
		assert_eq!(atempo(4.0), "atempo=2.0,atempo=2.0000");
		assert_eq!(atempo(0.25), "atempo=0.5,atempo=0.5000");
		assert_eq!(atempo(1.5), "atempo=1.5000");
	}

	#[test]
	fn probe_reads_rotation_and_streams() {
		let json = r#"{"streams":[
			{"codec_type":"video","codec_name":"h264","width":1920,"height":1080,"avg_frame_rate":"30000/1001",
			 "side_data_list":[{"side_data_type":"Display Matrix","rotation":-90}]},
			{"codec_type":"audio","codec_name":"aac","bit_rate":"128000"},
			{"codec_type":"video","codec_name":"mjpeg","disposition":{"attached_pic":1}}],
			"format":{"duration":"12.5"}}"#;
		let p = parse_probe(json).unwrap();
		assert_eq!(p.video_codec.as_deref(), Some("h264"));
		assert_eq!(p.audio_kbps, Some(128));
		assert_eq!(p.rotation, -90);
		assert_eq!(p.shown_size(), (1080, 1920));
		assert!((p.fps - 29.97).abs() < 0.01);
		assert_eq!(p.duration, 12.5);
	}

	#[test]
	fn image_outputs_name_their_muxer() {
		for f in [
			Format::Jpg,
			Format::Png,
			Format::Webp,
			Format::Avif,
			Format::Gif,
			Format::Qoi,
		] {
			let args = image_args(f, "", 90, false);
			let i = args.iter().position(|a| a == "-f").unwrap();
			assert_eq!(args[i + 1], muxer(f));
		}
	}
}
