use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::Error;
use crate::ff::{self, Probe, Run, atempo, audio_codec_args, join_vf, out, secs, strs, turn_filter, video_codec_args};
use crate::format::Format;
use crate::kind::{Kind, ext_of, kind_of};
use crate::plan::{audio_format_for, video_format_for};
use crate::recipe::{Recipe, SnapAt, Span, Tags};
use crate::run::Engines;
use crate::settings::Defaults;

type Progress<'a> = &'a mut dyn FnMut(f32);

fn os(v: Vec<String>) -> Vec<OsString> {
	v.into_iter().map(OsString::from).collect()
}

/// The container and codecs this file already uses, for recipes that keep the format.
fn own_format(input: &Path) -> Format {
	let ext = ext_of(input);
	if kind_of(input) == Kind::Audio {
		audio_format_for(&ext)
	} else {
		video_format_for(&ext)
	}
}

fn encode_args(to: Format, p: &Probe, d: &Defaults, crf: u8) -> Vec<String> {
	if to.is_audio() {
		audio_codec_args(to, d.audio_kbps)
	} else {
		video_codec_args(to, crf, p.has_audio(), d.audio_kbps.min(192))
	}
}

/// Maps the first video and audio stream, the way most recipes want.
fn maps(p: &Probe) -> Vec<String> {
	let mut m = Vec::new();
	if p.has_video() {
		m.extend(strs(&["-map", "0:v:0"]));
	}
	if p.has_audio() {
		m.extend(strs(&["-map", "0:a:0"]));
	}
	m
}

/// Whether the streams can move into this container untouched.
fn remuxable(p: &Probe, to: Format) -> bool {
	let v = p.video_codec.as_deref();
	let a = p.audio_codec.as_deref();
	let v_ok = match to {
		Format::Mp4 | Format::Mov => matches!(v, Some("h264" | "hevc" | "av1" | "mpeg4")),
		Format::Mkv => v.is_some(),
		Format::Webm => matches!(v, Some("vp8" | "vp9" | "av1")),
		_ => false,
	};
	let a_ok = match (to, a) {
		(_, None) => true,
		(Format::Mp4 | Format::Mov, Some(c)) => matches!(c, "aac" | "mp3" | "alac" | "ac3" | "eac3"),
		(Format::Mkv, Some(_)) => true,
		(Format::Webm, Some(c)) => matches!(c, "opus" | "vorbis"),
		_ => false,
	};
	v_ok && a_ok
}

pub fn convert(
	e: &Engines,
	d: &Defaults,
	input: &Path,
	to: Format,
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	let p = ff::probe(&e.ffprobe, input)?;
	let run = Run::new(&e.ffmpeg).input(input).duration(p.duration);
	let args: Vec<String> = if to.is_audio() {
		if !p.has_audio() {
			return Err(Error::Engine("this file has no sound".into()));
		}
		let mut a = strs(&["-map", "0:a:0", "-vn", "-sn"]);
		if to == Format::M4r {
			a.extend(strs(&["-t", "40", "-af", "afade=t=out:st=38:d=2"]));
		}
		a.extend(audio_codec_args(to, d.audio_kbps));
		a
	} else if to == Format::Gif {
		gif_args(None, d.gif_fps, d.gif_width)
	} else if remuxable(&p, to) {
		let mut a = maps(&p);
		a.extend(strs(&["-c", "copy", "-f", ff::muxer(to)]));
		if matches!(to, Format::Mp4 | Format::Mov) {
			a.extend(strs(&["-movflags", "+faststart"]));
		}
		a
	} else {
		let mut a = maps(&p);
		a.extend(strs(&["-vf", ff::EVEN]));
		a.extend(encode_args(to, &p, d, d.video_crf));
		a
	};
	run.args(args).args(out(part)).go(pr, c)?;
	Ok(())
}

fn gif_args(span: Option<Span>, fps: u32, width: u32) -> Vec<String> {
	let mut a = Vec::new();
	if let Some(s) = span {
		a.extend(strs(&["-ss", &secs(s.start), "-t", &secs(s.end - s.start)]));
	}
	let vf = join_vf(&[
		&format!("fps={fps}"),
		&format!("scale='min({width},iw)':-2:flags=lanczos"),
		"split[tw_a][tw_b];[tw_a]palettegen=stats_mode=diff[tw_p];[tw_b][tw_p]paletteuse=dither=sierra2_4a",
	]);
	a.extend(strs(&["-map", "0:v:0", "-vf", &vf, "-loop", "0", "-f", "gif"]));
	a
}

/// Runs a recipe that reads one video or sound file and writes `part`.
pub fn run(
	e: &Engines,
	d: &Defaults,
	r: &Recipe,
	input: &Path,
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	let p = ff::probe(&e.ffprobe, input)?;
	let to = own_format(input);
	let dur = p.duration;
	let base = || Run::new(&e.ffmpeg).input(input).duration(dur);
	let crf = d.video_crf;
	match r {
		Recipe::Trim { keep, separate, exact } => trim(e, d, &p, input, keep, *separate, *exact, part, pr, c),
		Recipe::CropVideo { x, y, w, h } => {
			let crop = format!("crop={}:{}:{}:{}", w & !1, h & !1, x, y);
			let mut a = maps(&p);
			a.extend(strs(&["-vf", &crop]));
			a.extend(encode_args(to, &p, d, crf));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::CompressVideo {
			crf,
			max_height,
			target_mb,
		} => compress_video(e, d, &p, input, *crf, *max_height, *target_mb, part, pr, c),
		Recipe::ResizeVideo { height, vertical } => {
			let vf = if *vertical {
				format!(
					"crop='min(iw,ih*9/16)':ih,scale=-2:'min({height}*16/9,ih)',{}",
					ff::EVEN
				)
			} else {
				format!("scale=-2:'min({height},ih)':flags=lanczos,{}", ff::EVEN)
			};
			let mut a = maps(&p);
			a.extend(strs(&["-vf", &vf]));
			a.extend(video_codec_args(Format::Mp4, crf, p.has_audio(), 160));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::Speed { factor } => {
			let f = factor.clamp(0.1, 16.0);
			let mut a = maps(&p);
			if p.has_video() {
				a.extend(strs(&["-vf", &format!("setpts=PTS/{f}")]));
			}
			if p.has_audio() {
				a.extend(strs(&["-af", &atempo(f)]));
			}
			a.extend(encode_args(to, &p, d, crf));
			base()
				.duration(dur / f as f64)
				.args(a)
				.args(out(part))
				.go(pr, c)
				.map(drop)
		}
		Recipe::MakeGif { span, fps, width } => {
			let len = span.map(|s| s.end - s.start).unwrap_or(dur);
			base()
				.duration(len)
				.args(gif_args(*span, *fps, *width))
				.args(out(part))
				.go(pr, c)
				.map(drop)
		}
		Recipe::Snapshot { at } => snapshot(e, &p, input, *at, part, pr, c),
		Recipe::Mute => {
			let mut a = strs(&["-map", "0:v:0", "-an", "-c:v", "copy"]);
			if matches!(to, Format::Mp4 | Format::Mov) {
				a.extend(strs(&["-movflags", "+faststart"]));
			}
			a.extend(strs(&["-f", ff::muxer(to)]));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::ExtractAudio => {
			if !p.has_audio() {
				return Err(Error::Engine("this video has no sound".into()));
			}
			let mut a = strs(&["-map", "0:a:0", "-vn"]);
			if p.audio_codec.as_deref() == Some("aac") {
				a.extend(strs(&["-c:a", "copy", "-f", "ipod"]));
			} else {
				a.extend(audio_codec_args(Format::M4a, 192));
			}
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::RotateVideo { turn } => {
			let mut a = maps(&p);
			a.extend(strs(&["-vf", turn_filter(*turn)]));
			a.extend(encode_args(to, &p, d, crf));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::Reverse => {
			if dur > 600.0 {
				return Err(Error::Engine("reversing works on clips up to 10 minutes".into()));
			}
			let mut a = maps(&p);
			if p.has_video() {
				a.extend(strs(&["-vf", "reverse"]));
			}
			if p.has_audio() {
				a.extend(strs(&["-af", "areverse"]));
			}
			a.extend(encode_args(to, &p, d, crf));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::Fade { fade_in, fade_out } => {
			let mut af = Vec::new();
			if *fade_in > 0.0 {
				af.push(format!("afade=t=in:st=0:d={fade_in}"));
			}
			if *fade_out > 0.0 {
				af.push(format!("afade=t=out:st={}:d={fade_out}", secs(dur - *fade_out as f64)));
			}
			let mut a = maps(&p);
			a.extend(strs(&["-af", &af.join(",")]));
			if p.has_video() {
				let mut vf = Vec::new();
				if *fade_in > 0.0 {
					vf.push(format!("fade=t=in:st=0:d={fade_in}"));
				}
				if *fade_out > 0.0 {
					vf.push(format!("fade=t=out:st={}:d={fade_out}", secs(dur - *fade_out as f64)));
				}
				a.extend(strs(&["-vf", &vf.join(",")]));
			}
			a.extend(encode_args(to, &p, d, crf));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::Normalize => audio_filter(e, d, &p, input, "loudnorm=I=-14:TP=-1:LRA=11", to, part, pr, c),
		Recipe::Volume { db } => audio_filter(e, d, &p, input, &format!("volume={db}dB"), to, part, pr, c),
		Recipe::Channels { mono } => {
			let mut a = maps(&p);
			a.extend(strs(&["-ac", if *mono { "1" } else { "2" }]));
			a.extend(encode_args(to, &p, d, crf));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::CompressAudio { kbps } => {
			let to = match to {
				Format::Wav | Format::Flac | Format::Aiff | Format::M4r => Format::Mp3,
				f => f,
			};
			let mut a = strs(&["-map", "0:a:0", "-vn"]);
			a.extend(audio_codec_args(to, *kbps));
			base().args(a).args(out(part)).go(pr, c).map(drop)
		}
		Recipe::Ringtone => {
			let mut a = strs(&["-map", "0:a:0", "-vn", "-t", "40", "-af", "afade=t=out:st=38:d=2"]);
			a.extend(audio_codec_args(Format::M4r, 256));
			base()
				.duration(dur.min(40.0))
				.args(a)
				.args(out(part))
				.go(pr, c)
				.map(drop)
		}
		Recipe::RemoveSilence { threshold_db, min_gap } => {
			let f = format!(
				"silenceremove=start_periods=1:start_threshold={threshold_db}dB:stop_periods=-1:stop_duration={min_gap}:stop_threshold={threshold_db}dB"
			);
			audio_filter(e, d, &p, input, &f, to, part, pr, c)
		}
		Recipe::Tags { tags } => write_tags(e, &p, input, tags, to, part, pr, c),
		_ => Err(Error::Engine("this tool doesn't fit this file".into())),
	}
}

#[allow(clippy::too_many_arguments)]
fn audio_filter(
	e: &Engines,
	d: &Defaults,
	p: &Probe,
	input: &Path,
	filter: &str,
	to: Format,
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	if !p.has_audio() {
		return Err(Error::Engine("this file has no sound".into()));
	}
	let mut a = maps(p);
	a.extend(strs(&["-af", filter]));
	if p.has_video() {
		// the picture is copied untouched; only the sound is re-encoded
		a.extend(strs(&["-c:v", "copy"]));
		a.extend(audio_codec_args_for_video(to, d.audio_kbps));
		a.extend(strs(&["-f", ff::muxer(to)]));
	} else {
		a.extend(audio_codec_args(to, d.audio_kbps));
	}
	Run::new(&e.ffmpeg)
		.input(input)
		.duration(p.duration)
		.args(a)
		.args(out(part))
		.go(pr, c)
		.map(drop)
}

fn audio_codec_args_for_video(to: Format, kbps: u32) -> Vec<String> {
	match to {
		Format::Webm => strs(&["-c:a", "libopus", "-b:a", "128k"]),
		Format::Avi => strs(&["-c:a", "libmp3lame", "-q:a", "3"]),
		Format::Wmv => vec!["-c:a".into(), "wmav2".into(), "-b:a".into(), format!("{kbps}k")],
		_ => vec![
			"-c:a".into(),
			"aac".into(),
			"-b:a".into(),
			format!("{}k", kbps.min(256)),
		],
	}
}

#[allow(clippy::too_many_arguments)]
fn trim(
	e: &Engines,
	d: &Defaults,
	p: &Probe,
	input: &Path,
	keep: &[Span],
	separate: bool,
	exact: bool,
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	let to = own_format(input);
	let spans: Vec<Span> = keep
		.iter()
		.map(|s| Span {
			start: s.start.clamp(0.0, p.duration.max(s.start)),
			end: s.end.min(if p.duration > 0.0 { p.duration } else { s.end }),
		})
		.filter(|s| s.end - s.start > 0.05)
		.collect();
	if spans.is_empty() {
		return Err(Error::Engine("nothing left to keep".into()));
	}
	let one = |span: Span, dest: &Path, pr: Progress, offset: f32, share: f32| -> Result<(), Error> {
		let len = span.end - span.start;
		let mut a: Vec<String> = strs(&["-ss", &secs(span.start)]);
		a.extend(strs(&["-i"]));
		a.push(input.to_string_lossy().into_owned());
		a.extend(strs(&["-t", &secs(len)]));
		a.extend(maps(p));
		if exact {
			a.extend(encode_args(to, p, d, d.video_crf.saturating_sub(2)));
		} else {
			a.extend(strs(&[
				"-c",
				"copy",
				"-avoid_negative_ts",
				"make_zero",
				"-f",
				ff::muxer(to),
			]));
		}
		Run::new(&e.ffmpeg)
			.args(os(a))
			.duration(len)
			.span(offset, share)
			.args(out(dest))
			.go(pr, c)
			.map(drop)
	};
	if spans.len() == 1 {
		return one(spans[0], part, pr, 0.0, 1.0);
	}
	if separate {
		fs::create_dir_all(part)?;
		let n = spans.len() as f32;
		for (i, s) in spans.iter().enumerate() {
			let dest = part.join(format!("Part {}.{}", i + 1, to.ext()));
			one(*s, &dest, pr, i as f32 / n, 1.0 / n)?;
		}
		return Ok(());
	}
	// several kept parts glued back together, with exact cuts
	let mut fc = String::new();
	let mut labels = String::new();
	for (i, s) in spans.iter().enumerate() {
		if p.has_video() {
			fc.push_str(&format!(
				"[0:v:0]trim=start={}:end={},setpts=PTS-STARTPTS[v{i}];",
				secs(s.start),
				secs(s.end)
			));
			labels.push_str(&format!("[v{i}]"));
		}
		if p.has_audio() {
			fc.push_str(&format!(
				"[0:a:0]atrim=start={}:end={},asetpts=PTS-STARTPTS[a{i}];",
				secs(s.start),
				secs(s.end)
			));
			labels.push_str(&format!("[a{i}]"));
		}
	}
	let (v, a) = (p.has_video() as u8, p.has_audio() as u8);
	fc.push_str(&format!("{labels}concat=n={}:v={v}:a={a}", spans.len()));
	let mut maps_out = Vec::new();
	if v == 1 {
		fc.push_str("[ov]");
		maps_out.extend(strs(&["-map", "[ov]"]));
	}
	if a == 1 {
		fc.push_str("[oa]");
		maps_out.extend(strs(&["-map", "[oa]"]));
	}
	let total: f64 = spans.iter().map(|s| s.end - s.start).sum();
	let mut args = strs(&["-filter_complex", &fc]);
	args.extend(maps_out);
	args.extend(encode_args(to, p, d, d.video_crf.saturating_sub(2)));
	Run::new(&e.ffmpeg)
		.input(input)
		.duration(total)
		.args(args)
		.args(out(part))
		.go(pr, c)
		.map(drop)
}

#[allow(clippy::too_many_arguments)]
fn compress_video(
	e: &Engines,
	_d: &Defaults,
	p: &Probe,
	input: &Path,
	crf: u8,
	max_height: Option<u32>,
	target_mb: Option<f64>,
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	let mut vf = Vec::new();
	if let Some(h) = max_height {
		vf.push(format!("scale=-2:'min({h},ih)':flags=lanczos"));
	}
	let audio_k: u32 = if target_mb.is_some_and(|t| t < 15.0) { 96 } else { 128 };
	let Some(target) = target_mb else {
		let mut a = maps(p);
		vf.push(ff::EVEN.into());
		a.extend(strs(&["-vf", &vf.join(",")]));
		a.extend(video_codec_args(Format::Mp4, crf, p.has_audio(), audio_k));
		return Run::new(&e.ffmpeg)
			.input(input)
			.duration(p.duration)
			.args(a)
			.args(out(part))
			.go(pr, c)
			.map(drop);
	};
	if p.duration <= 0.0 {
		return Err(Error::Engine(
			"this video's length is unknown, so a size can't be aimed at".into(),
		));
	}
	// bits we can spend on the picture once the sound and the container take theirs
	let total_kbit = target * 8.0 * 1024.0 * 0.97;
	let audio_kbit = if p.has_audio() {
		audio_k as f64 * p.duration
	} else {
		0.0
	};
	let video_kbps = ((total_kbit - audio_kbit) / p.duration).floor();
	if video_kbps < 60.0 {
		return Err(Error::Engine(format!(
			"{target:.0} MB is too small for a video this long"
		)));
	}
	// shrink big frames when the budget is tight, so the result isn't a smear
	let pixels_ok = (video_kbps * 1000.0 / 0.08 / p.fps.max(24.0)) as u32;
	let (w, h) = p.shown_size();
	if max_height.is_none() && (w * h) > pixels_ok && h > 480 {
		let scale = ((pixels_ok as f64) / (w as f64 * h as f64)).sqrt();
		let nh = ((h as f64 * scale) as u32).clamp(360, h) & !1;
		vf.push(format!("scale=-2:{nh}:flags=lanczos"));
	}
	vf.push(ff::EVEN.into());
	let passlog = e.scratch.join(format!("pass-{}", std::process::id()));
	fs::create_dir_all(&e.scratch)?;
	let vb = format!("{video_kbps}k");
	let common = |pass: &str| {
		let mut a = strs(&["-map", "0:v:0", "-vf", &vf.join(",")]);
		a.extend(strs(&[
			"-c:v",
			"libx264",
			"-preset",
			"medium",
			"-b:v",
			&vb,
			"-pix_fmt",
			"yuv420p",
			"-pass",
			pass,
			"-passlogfile",
		]));
		a.push(passlog.to_string_lossy().into_owned());
		a
	};
	let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
	let mut first = common("1");
	first.extend(strs(&["-an", "-f", "null"]));
	Run::new(&e.ffmpeg)
		.input(input)
		.duration(p.duration)
		.span(0.0, 0.45)
		.args(first)
		.args(out(Path::new(null)))
		.go(pr, c)?;
	let mut second = common("2");
	if p.has_audio() {
		second.extend(strs(&["-map", "0:a:0", "-c:a", "aac", "-b:a", &format!("{audio_k}k")]));
	}
	second.extend(strs(&["-movflags", "+faststart", "-f", "mp4"]));
	let result = Run::new(&e.ffmpeg)
		.input(input)
		.duration(p.duration)
		.span(0.45, 0.55)
		.args(second)
		.args(out(part))
		.go(pr, c);
	for ext in ["-0.log", "-0.log.mbtree"] {
		let _ = fs::remove_file(format!("{}{ext}", passlog.display()));
	}
	result.map(drop)
}

fn snapshot(
	e: &Engines,
	p: &Probe,
	input: &Path,
	at: SnapAt,
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	match at {
		SnapAt::Every(n) => {
			fs::create_dir_all(part)?;
			let n = n.max(0.5);
			Run::new(&e.ffmpeg)
				.input(input)
				.duration(p.duration)
				// the first frame always counts, then one every n seconds
				.args(os(strs(&[
					"-map",
					"0:v:0",
					"-vf",
					&format!("select='isnan(prev_selected_t)+gte(t-prev_selected_t\\,{n})'"),
					"-fps_mode",
					"vfr",
					"-c:v",
					"png",
				])))
				.args(out(&part.join("Frame %03d.png")))
				.go(pr, c)
				.map(drop)
		}
		_ => {
			let t = match at {
				SnapAt::First => 0.0,
				SnapAt::At(t) => t,
				_ => p.duration / 2.0,
			};
			Run::new(&e.ffmpeg)
				.args(os(strs(&["-ss", &secs(t)])))
				.input(input)
				.args(os(strs(&[
					"-map",
					"0:v:0",
					"-frames:v",
					"1",
					"-c:v",
					"png",
					"-f",
					"image2",
					"-update",
					"1",
				])))
				.args(out(part))
				.go(pr, c)
				.map(drop)
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn write_tags(
	e: &Engines,
	p: &Probe,
	input: &Path,
	t: &Tags,
	to: Format,
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	let mut run = Run::new(&e.ffmpeg).input(input).duration(p.duration);
	let mut a: Vec<String> = Vec::new();
	let cover = t
		.cover
		.as_ref()
		.filter(|_| matches!(to, Format::Mp3 | Format::M4a | Format::Flac | Format::M4r));
	if let Some(cv) = cover {
		run = run.input(cv);
		a.extend(strs(&[
			"-map",
			"0:a:0",
			"-map",
			"1:v:0",
			"-c:a",
			"copy",
			"-c:v",
			"mjpeg",
			"-disposition:v:0",
			"attached_pic",
		]));
	} else {
		a.extend(strs(&["-map", "0", "-c", "copy"]));
	}
	if t.clear {
		a.extend(strs(&["-map_metadata", "-1"]));
	}
	let fields = [
		("title", &t.title),
		("artist", &t.artist),
		("album", &t.album),
		("album_artist", &t.album_artist),
		("date", &t.year),
		("genre", &t.genre),
		("track", &t.track),
		("comment", &t.comment),
	];
	for (k, v) in fields {
		if let Some(v) = v {
			a.push("-metadata".into());
			a.push(format!("{k}={v}"));
		}
	}
	if to == Format::Mp3 {
		a.extend(strs(&["-id3v2_version", "3"]));
	}
	a.extend(strs(&["-f", ff::muxer(to)]));
	run.args(a).args(out(part)).go(pr, c).map(drop)
}

/// Joins videos or sounds end to end. Different sizes are fitted into the first one's frame.
pub fn join(
	e: &Engines,
	d: &Defaults,
	inputs: &[PathBuf],
	part: &Path,
	pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	let probes: Vec<Probe> = inputs
		.iter()
		.map(|i| ff::probe(&e.ffprobe, i))
		.collect::<Result<_, _>>()?;
	let first = &probes[0];
	let to = own_format(&inputs[0]);
	let video = first.has_video() && probes.iter().all(Probe::has_video);
	let audio = probes.iter().all(Probe::has_audio);
	let total: f64 = probes.iter().map(|p| p.duration).sum();
	let (w, h) = first.shown_size();
	let (w, h) = (w.max(2) & !1, h.max(2) & !1);
	let fps = if first.fps > 0.0 { first.fps } else { 30.0 };
	let mut run = Run::new(&e.ffmpeg).duration(total);
	for i in inputs {
		run = run.input(i);
	}
	let mut fc = String::new();
	let mut labels = String::new();
	for (i, _) in inputs.iter().enumerate() {
		if video {
			fc.push_str(&format!(
				"[{i}:v:0]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps={fps:.3},format=yuv420p[v{i}];"
			));
			labels.push_str(&format!("[v{i}]"));
		}
		if audio {
			fc.push_str(&format!(
				"[{i}:a:0]aformat=sample_rates=48000:channel_layouts=stereo[a{i}];"
			));
			labels.push_str(&format!("[a{i}]"));
		}
	}
	fc.push_str(&format!(
		"{labels}concat=n={}:v={}:a={}",
		inputs.len(),
		video as u8,
		audio as u8
	));
	let mut a = Vec::new();
	match (video, audio) {
		(true, true) => {
			fc.push_str("[ov][oa]");
			a.extend(strs(&["-filter_complex", &fc, "-map", "[ov]", "-map", "[oa]"]));
		}
		(true, false) => {
			fc.push_str("[ov]");
			a.extend(strs(&["-filter_complex", &fc, "-map", "[ov]"]));
		}
		(false, true) => {
			fc.push_str("[oa]");
			a.extend(strs(&["-filter_complex", &fc, "-map", "[oa]"]));
		}
		_ => return Err(Error::Engine("these files can't be joined".into())),
	}
	if video {
		a.extend(video_codec_args(to, d.video_crf, audio, d.audio_kbps.min(192)));
	} else {
		a.extend(audio_codec_args(to, d.audio_kbps));
	}
	run.args(a).args(out(part)).go(pr, c).map(drop)
}

/// Encodes a WAV (from the speech voice) into MP3.
pub fn wav_to_mp3(e: &Engines, wav: &Path, part: &Path, pr: Progress, c: &AtomicBool) -> Result<(), Error> {
	let mut a = strs(&["-map", "0:a:0"]);
	a.extend(audio_codec_args(Format::Mp3, 128));
	Run::new(&e.ffmpeg)
		.input(wav)
		.args(a)
		.args(out(part))
		.go(pr, c)
		.map(drop)
}

/// A strip of `count` frames across the video, each `height` pixels tall, for a timeline.
/// `count` frames spread over a video of `duration` seconds, side by side, `height` px tall. Only
/// keyframes are decoded, so it stays quick however long the video is.
pub fn filmstrip(
	e: &Engines,
	input: &Path,
	duration: f64,
	count: u32,
	height: u32,
	out_png: &Path,
) -> Result<(), Error> {
	let every = (duration / count as f64).max(0.05);
	let vf = format!("fps=1/{every:.3},scale=-2:{height}:flags=fast_bilinear,tile={count}x1");
	Run::new(&e.ffmpeg)
		.arg("-skip_frame")
		.arg("nokey")
		.input(input)
		.args(os(strs(&[
			"-map",
			"0:v:0",
			"-vf",
			&vf,
			"-frames:v",
			"1",
			"-c:v",
			"png",
			"-f",
			"image2",
			"-update",
			"1",
		])))
		.args(out(out_png))
		.go(&mut |_| {}, &AtomicBool::new(false))
		.map(drop)
}

/// Whether the editor's browser can play this file as it is.
pub fn plays_in_browser(input: &Path, p: &Probe) -> bool {
	let ext = ext_of(input);
	let v_ok = match p.video_codec.as_deref() {
		None => true,
		Some(c) => {
			matches!(c, "h264" | "vp8" | "vp9" | "av1")
				&& matches!(ext.as_str(), "mp4" | "m4v" | "mov" | "webm" | "mkv")
		}
	};
	let a_ok = match p.audio_codec.as_deref() {
		None => true,
		Some(c) => matches!(
			c,
			"aac" | "mp3" | "opus" | "vorbis" | "flac" | "pcm_s16le" | "pcm_s24le" | "pcm_f32le"
		),
	};
	v_ok && a_ok
}

/// A quick copy the editor can play, with the same timing as the original.
pub fn preview_copy(e: &Engines, input: &Path, p: &Probe, out_file: &Path, c: &AtomicBool) -> Result<(), Error> {
	let mut a = maps(p);
	if p.has_video() {
		a.extend(strs(&[
			"-vf",
			"scale=-2:'min(720,ih)'",
			"-c:v",
			"libx264",
			"-preset",
			"ultrafast",
			"-crf",
			"26",
			"-pix_fmt",
			"yuv420p",
		]));
		a.extend(strs(&[
			"-c:a",
			"aac",
			"-b:a",
			"128k",
			"-movflags",
			"+faststart",
			"-f",
			"mp4",
		]));
	} else {
		a.extend(strs(&["-c:a", "aac", "-b:a", "160k", "-f", "ipod"]));
	}
	Run::new(&e.ffmpeg)
		.input(input)
		.duration(p.duration)
		.args(a)
		.args(out(out_file))
		.go(&mut |_| {}, c)
		.map(drop)
}

/// Peaks for drawing a waveform, `buckets` values from 0 to 1.
pub fn waveform(e: &Engines, input: &Path, buckets: usize) -> Result<Vec<f32>, Error> {
	use std::io::Read;
	let mut child = ff::command(&e.ffmpeg)
		.args(["-hide_banner", "-nostdin", "-loglevel", "error", "-i"])
		.arg(input)
		.args(["-map", "0:a:0", "-ac", "1", "-ar", "4000", "-f", "s16le", "-"])
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::null())
		.spawn()?;
	let mut raw = Vec::new();
	child.stdout.take().expect("piped").read_to_end(&mut raw)?;
	let _ = child.wait();
	let samples: Vec<i16> = raw
		.as_chunks::<2>()
		.0
		.iter()
		.map(|b| i16::from_le_bytes([b[0], b[1]]))
		.collect();
	if samples.is_empty() || buckets == 0 {
		return Ok(vec![0.0; buckets]);
	}
	let per = samples.len().div_ceil(buckets);
	Ok(samples
		.chunks(per)
		.map(|ch| ch.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0) as f32 / 32768.0)
		.collect())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn probe(v: Option<&str>, a: Option<&str>) -> Probe {
		Probe {
			video_codec: v.map(Into::into),
			audio_codec: a.map(Into::into),
			..Probe::default()
		}
	}

	#[test]
	fn remux_only_when_codecs_fit() {
		assert!(remuxable(&probe(Some("h264"), Some("aac")), Format::Mp4));
		assert!(remuxable(&probe(Some("h264"), Some("opus")), Format::Mkv));
		assert!(!remuxable(&probe(Some("h264"), Some("opus")), Format::Mp4));
		assert!(!remuxable(&probe(Some("h264"), Some("aac")), Format::Webm));
		assert!(!remuxable(&probe(Some("wmv2"), None), Format::Mp4));
	}
}
