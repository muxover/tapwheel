use std::path::{Path, PathBuf};

use crate::kind::{Kind, common_kind, ext_of, kind_of};
use crate::ops::{Format, Op};

#[derive(Clone, Debug, PartialEq)]
pub enum Work {
	Ffmpeg { input: PathBuf, args: Vec<String> },
	PngQuant { input: PathBuf },
	Subtitle { input: PathBuf, to: Format },
	Pack { inputs: Vec<PathBuf>, format: Format },
	Repack { input: PathBuf, format: Format },
	Extract { input: PathBuf },
}

/// One unit of background work and the path it will produce.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
	pub op: Op,
	pub output: PathBuf,
	pub work: Work,
	/// Whether the engine reports a duration we can turn into a percentage.
	pub timed: bool,
}

impl Job {
	pub fn inputs(&self) -> Vec<&Path> {
		match &self.work {
			Work::Pack { inputs, .. } => inputs.iter().map(PathBuf::as_path).collect(),
			Work::Ffmpeg { input, .. }
			| Work::PngQuant { input }
			| Work::Subtitle { input, .. }
			| Work::Repack { input, .. }
			| Work::Extract { input } => vec![input.as_path()],
		}
	}
}

/// File name without its extension, treating `.tar.gz` as one extension.
pub fn stem_of(path: &Path) -> String {
	let name = path
		.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default();
	let lower = name.to_lowercase();
	if path.is_dir() {
		return name;
	}
	if lower.ends_with(".tar.gz") {
		return name[..name.len() - 7].to_string();
	}
	match name.rfind('.') {
		Some(i) if i > 0 => name[..i].to_string(),
		_ => name,
	}
}

/// `dir/stem.ext`, or `dir/stem (1).ext` and so on when that name is taken.
pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
	let make = |n: u32| {
		let base = if n == 0 {
			stem.to_string()
		} else {
			format!("{stem} ({n})")
		};
		if ext.is_empty() {
			dir.join(base)
		} else {
			dir.join(format!("{base}.{ext}"))
		}
	};
	(0..)
		.map(make)
		.find(|p| !p.exists() && !part_path(p).exists())
		.expect("unbounded range")
}

/// Where a job writes until it finishes, so a half-done file never has the final name.
pub fn part_path(output: &Path) -> PathBuf {
	let name = output
		.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default();
	output.with_file_name(format!("{name}.tw-part"))
}

fn parent(path: &Path) -> PathBuf {
	path.parent().map(Path::to_path_buf).unwrap_or_default()
}

const STILL_ENCODABLE: &[&str] = &["jpg", "jpeg", "png", "webp", "avif", "tif", "tiff", "bmp", "gif"];
const AUDIO_ENCODABLE: &[&str] = &["mp3", "m4a", "wav", "flac", "ogg", "opus"];

fn image_format_for(ext: &str) -> Format {
	match ext {
		"png" => Format::Png,
		"webp" => Format::Webp,
		"avif" => Format::Avif,
		"tif" | "tiff" => Format::Tiff,
		"bmp" => Format::Bmp,
		"gif" => Format::Gif,
		_ => Format::Jpg,
	}
}

fn audio_format_for(ext: &str) -> Format {
	match ext {
		"m4a" | "aac" | "alac" => Format::M4a,
		"wav" => Format::Wav,
		"flac" => Format::Flac,
		"ogg" | "oga" => Format::Ogg,
		"opus" => Format::Opus,
		_ => Format::Mp3,
	}
}

/// Keeps the file's own format when we can write it, otherwise JPG.
fn same_image(ext: &str) -> Format {
	if STILL_ENCODABLE.contains(&ext) {
		image_format_for(ext)
	} else {
		Format::Jpg
	}
}

fn same_audio(ext: &str) -> Format {
	if AUDIO_ENCODABLE.contains(&ext) {
		audio_format_for(ext)
	} else {
		Format::Mp3
	}
}

/// Turns a chosen slice into jobs. Actions return nothing; the app runs those itself.
pub fn plan(op: Op, paths: &[PathBuf]) -> Vec<Job> {
	if paths.is_empty() || op.is_action() {
		return Vec::new();
	}
	match op {
		Op::Zip => vec![pack_job(op, paths, Format::Zip)],
		Op::Convert(f) if f.is_archive() => {
			if common_kind(paths) == Some(Kind::Archive) {
				paths.iter().map(|p| repack_job(op, p, f)).collect()
			} else {
				vec![pack_job(op, paths, f)]
			}
		}
		_ => paths.iter().filter_map(|p| file_job(op, p)).collect(),
	}
}

fn pack_job(op: Op, paths: &[PathBuf], format: Format) -> Job {
	let first = &paths[0];
	let stem = if paths.len() == 1 {
		stem_of(first)
	} else {
		"Archive".to_string()
	};
	Job {
		op,
		output: unique_path(&parent(first), &stem, format.ext()),
		work: Work::Pack {
			inputs: paths.to_vec(),
			format,
		},
		timed: false,
	}
}

fn repack_job(op: Op, path: &Path, format: Format) -> Job {
	Job {
		op,
		output: unique_path(&parent(path), &stem_of(path), format.ext()),
		work: Work::Repack {
			input: path.to_path_buf(),
			format,
		},
		timed: false,
	}
}

fn file_job(op: Op, input: &Path) -> Option<Job> {
	let ext = ext_of(input);
	let kind = kind_of(input);
	let dir = parent(input);
	let stem = stem_of(input);
	let out = |suffix: &str, f: Format| {
		let s = if suffix.is_empty() {
			stem.clone()
		} else {
			format!("{stem}-{suffix}")
		};
		unique_path(&dir, &s, f.ext())
	};
	let ff = |args: Vec<String>, output: PathBuf, timed: bool| Job {
		op,
		output,
		work: Work::Ffmpeg {
			input: input.to_path_buf(),
			args,
		},
		timed,
	};
	let job = match (op, kind) {
		(Op::Convert(to), Kind::Subtitle) => Job {
			op,
			output: out("", to),
			work: Work::Subtitle {
				input: input.to_path_buf(),
				to,
			},
			timed: false,
		},
		(Op::Convert(to), Kind::Image) => {
			let animated = ext == "gif" && to == Format::Mp4;
			if animated {
				ff(video_args(to, None, VideoQuality::Normal, false), out("", to), true)
			} else {
				ff(image_args(to, None, ImageQuality::High, false), out("", to), false)
			}
		}
		(Op::Convert(to), Kind::Audio) => ff(audio_args(to, None, AudioQuality::High), out("", to), true),
		(Op::Convert(to), Kind::Video) => {
			let args = if is_audio_format(to) {
				audio_args(to, None, AudioQuality::High)
			} else {
				video_args(to, None, VideoQuality::Normal, true)
			};
			ff(args, out("", to), true)
		}
		(Op::CompressImage, Kind::Image) => {
			if ext == "png" {
				Job {
					op,
					output: out("small", Format::Png),
					work: Work::PngQuant {
						input: input.to_path_buf(),
					},
					timed: false,
				}
			} else {
				let to = same_image(&ext);
				let to = if matches!(to, Format::Tiff | Format::Bmp) {
					Format::Jpg
				} else {
					to
				};
				ff(image_args(to, None, ImageQuality::Small, true), out("small", to), false)
			}
		}
		(Op::HalfSize, Kind::Image) => {
			let to = same_image(&ext);
			let vf = "scale=trunc(iw/2):trunc(ih/2):flags=lanczos";
			ff(
				image_args(to, Some(vf), ImageQuality::High, false),
				out("half", to),
				false,
			)
		}
		(Op::FitHd, Kind::Image) => {
			let to = same_image(&ext);
			let vf = "scale='min(1920,iw)':'min(1920,ih)':force_original_aspect_ratio=decrease:flags=lanczos";
			ff(
				image_args(to, Some(vf), ImageQuality::High, false),
				out("1920", to),
				false,
			)
		}
		(Op::Rotate, Kind::Image) => {
			let to = same_image(&ext);
			ff(
				image_args(to, Some("transpose=clock"), ImageQuality::High, false),
				out("rotated", to),
				false,
			)
		}
		(Op::StripMetadata, Kind::Image) => {
			let to = same_image(&ext);
			ff(image_args(to, None, ImageQuality::High, true), out("clean", to), false)
		}
		(Op::CompressVideo, Kind::Video) => ff(
			video_args(Format::Mp4, None, VideoQuality::Small, true),
			out("small", Format::Mp4),
			true,
		),
		(Op::Video720, Kind::Video) => ff(
			video_args(Format::Mp4, Some("scale=-2:'min(720,ih)'"), VideoQuality::Normal, true),
			out("720p", Format::Mp4),
			true,
		),
		(Op::Mute, Kind::Video) => {
			let to = video_container_for(&ext);
			let mut args = strs(&["-map", "0:v:0", "-an", "-c:v", "copy"]);
			if matches!(to, Format::Mp4 | Format::Mov) {
				args.extend(strs(&["-movflags", "+faststart"]));
			}
			args.extend(strs(&["-f", muxer(to)]));
			ff(args, out("muted", to), true)
		}
		(Op::ExtractAudio, Kind::Video) => ff(
			audio_args(Format::M4a, None, AudioQuality::High),
			out("", Format::M4a),
			true,
		),
		(Op::CompressAudio, Kind::Audio) => ff(
			audio_args(Format::Mp3, None, AudioQuality::Small),
			out("small", Format::Mp3),
			true,
		),
		(Op::Normalize, Kind::Audio) => {
			let to = same_audio(&ext);
			ff(
				audio_args(to, Some("loudnorm=I=-16:TP=-1.5:LRA=11"), AudioQuality::High),
				out("normalized", to),
				true,
			)
		}
		(Op::Extract, Kind::Archive) => Job {
			op,
			output: unique_path(&dir, &stem, ""),
			work: Work::Extract {
				input: input.to_path_buf(),
			},
			timed: false,
		},
		_ => return None,
	};
	Some(job)
}

fn strs(s: &[&str]) -> Vec<String> {
	s.iter().map(|s| s.to_string()).collect()
}

fn is_audio_format(f: Format) -> bool {
	matches!(
		f,
		Format::Mp3 | Format::M4a | Format::Wav | Format::Flac | Format::Ogg | Format::Opus
	)
}

fn video_container_for(ext: &str) -> Format {
	match ext {
		"mov" => Format::Mov,
		"mkv" => Format::Mkv,
		"webm" => Format::Webm,
		"avi" => Format::Avi,
		_ => Format::Mp4,
	}
}

pub fn muxer(f: Format) -> &'static str {
	match f {
		Format::Jpg | Format::Png | Format::Tiff | Format::Bmp => "image2",
		Format::Webp => "webp",
		Format::Avif => "avif",
		Format::Gif => "gif",
		Format::Ico => "ico",
		Format::Mp3 => "mp3",
		Format::M4a => "ipod",
		Format::Wav => "wav",
		Format::Flac => "flac",
		Format::Ogg => "ogg",
		Format::Opus => "opus",
		Format::Mp4 => "mp4",
		Format::Mov => "mov",
		Format::Mkv => "matroska",
		Format::Webm => "webm",
		Format::Avi => "avi",
		Format::Srt => "srt",
		Format::Vtt => "webvtt",
		Format::Txt | Format::Zip | Format::SevenZ | Format::Tar | Format::Tgz => "",
	}
}

#[derive(Clone, Copy, PartialEq)]
enum ImageQuality {
	High,
	Small,
}

#[derive(Clone, Copy, PartialEq)]
enum VideoQuality {
	Normal,
	Small,
}

#[derive(Clone, Copy, PartialEq)]
enum AudioQuality {
	High,
	Small,
}

// JPG and AVIF have no alpha here, so transparent pixels go onto white instead of black.
const ON_WHITE: &str =
	"split[tw_a][tw_b];[tw_a]format=rgb24,drawbox=c=white:t=fill[tw_bg];[tw_bg][tw_b]overlay=format=auto";
const EVEN: &str = "scale=trunc(iw/2)*2:trunc(ih/2)*2";

fn join_vf(parts: &[&str]) -> String {
	parts
		.iter()
		.filter(|p| !p.is_empty())
		.copied()
		.collect::<Vec<_>>()
		.join(",")
}

fn image_args(to: Format, filter: Option<&str>, q: ImageQuality, strip: bool) -> Vec<String> {
	let base = filter.unwrap_or("");
	let (vf, codec): (String, Vec<&str>) = match to {
		Format::Jpg => (
			join_vf(&[base, ON_WHITE, "format=yuvj444p"]),
			vec![
				"-c:v",
				"mjpeg",
				"-q:v",
				if q == ImageQuality::Small { "6" } else { "2" },
			],
		),
		Format::Png => (join_vf(&[base]), vec!["-c:v", "png"]),
		Format::Webp => (
			join_vf(&[base]),
			vec![
				"-c:v",
				"libwebp",
				"-quality",
				if q == ImageQuality::Small { "72" } else { "90" },
			],
		),
		Format::Avif => (
			join_vf(&[base, ON_WHITE, EVEN, "format=yuv420p"]),
			vec![
				"-c:v",
				"libaom-av1",
				"-still-picture",
				"1",
				"-crf",
				if q == ImageQuality::Small { "38" } else { "26" },
				"-b:v",
				"0",
				"-cpu-used",
				"6",
				"-row-mt",
				"1",
			],
		),
		Format::Tiff => (join_vf(&[base]), vec!["-c:v", "tiff", "-compression_algo", "lzw"]),
		Format::Bmp => (join_vf(&[base, "format=bgra"]), vec!["-c:v", "bmp"]),
		Format::Gif => (
			join_vf(&[
				base,
				"split[tw_a][tw_b];[tw_a]palettegen=reserve_transparent=1[tw_p];[tw_b][tw_p]paletteuse=alpha_threshold=128",
			]),
			vec!["-c:v", "gif"],
		),
		Format::Ico => (
			join_vf(&[
				base,
				"scale='min(256,iw)':'min(256,ih)':force_original_aspect_ratio=decrease",
				"format=rgba",
			]),
			vec!["-c:v", "png"],
		),
		_ => (String::new(), Vec::new()),
	};
	let mut args = Vec::new();
	if !vf.is_empty() {
		args.extend(strs(&["-vf", &vf]));
	}
	args.extend(strs(&codec));
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

fn video_args(to: Format, filter: Option<&str>, q: VideoQuality, with_audio: bool) -> Vec<String> {
	let base = filter.unwrap_or("");
	let mut args = strs(&["-map", "0:v:0"]);
	if with_audio && to != Format::Gif {
		args.extend(strs(&["-map", "0:a:0?"]));
	}
	let crf = |normal: &'static str, small: &'static str| {
		if q == VideoQuality::Small { small } else { normal }
	};
	match to {
		Format::Mp4 | Format::Mov | Format::Mkv => {
			args.extend(strs(&["-vf", &join_vf(&[base, EVEN, "format=yuv420p"])]));
			args.extend(strs(&["-c:v", "libx264", "-preset", "medium", "-crf", crf("20", "28")]));
			args.extend(strs(&[
				"-c:a",
				"aac",
				"-b:a",
				if q == VideoQuality::Small { "128k" } else { "192k" },
			]));
			if to != Format::Mkv {
				args.extend(strs(&["-movflags", "+faststart"]));
			}
		}
		Format::Webm => {
			args.extend(strs(&["-vf", &join_vf(&[base, EVEN, "format=yuv420p"])]));
			args.extend(strs(&[
				"-c:v",
				"libvpx-vp9",
				"-crf",
				crf("32", "40"),
				"-b:v",
				"0",
				"-row-mt",
				"1",
				"-deadline",
				"good",
				"-cpu-used",
				"4",
				"-c:a",
				"libopus",
				"-b:a",
				"128k",
			]));
		}
		Format::Avi => {
			args.extend(strs(&["-vf", &join_vf(&[base, EVEN])]));
			args.extend(strs(&[
				"-c:v",
				"mpeg4",
				"-q:v",
				"4",
				"-vtag",
				"XVID",
				"-c:a",
				"libmp3lame",
				"-q:a",
				"3",
			]));
		}
		Format::Gif => {
			args.extend(strs(&[
				"-vf",
				&join_vf(&[
					base,
					"fps=12",
					"scale='min(640,iw)':-2:flags=lanczos",
					"split[tw_a][tw_b];[tw_a]palettegen=stats_mode=diff[tw_p];[tw_b][tw_p]paletteuse=dither=bayer:bayer_scale=4",
				]),
			]));
			args.extend(strs(&["-loop", "0"]));
		}
		_ => {}
	}
	args.extend(strs(&["-f", muxer(to)]));
	args
}

fn audio_args(to: Format, filter: Option<&str>, q: AudioQuality) -> Vec<String> {
	let mut args = strs(&["-map", "0:a:0", "-vn", "-sn"]);
	if let Some(af) = filter {
		args.extend(strs(&["-af", af]));
	}
	let small = q == AudioQuality::Small;
	let codec: &[&str] = match to {
		Format::Mp3 => &["-c:a", "libmp3lame", "-q:a", if small { "6" } else { "2" }],
		Format::M4a => &["-c:a", "aac", "-b:a", "192k"],
		Format::Wav => &["-c:a", "pcm_s16le"],
		Format::Flac => &["-c:a", "flac"],
		Format::Ogg => &["-c:a", "libvorbis", "-q:a", "6"],
		Format::Opus => &["-c:a", "libopus", "-b:a", "128k"],
		_ => &[],
	};
	args.extend(strs(codec));
	args.extend(strs(&["-f", muxer(to)]));
	args
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;

	fn tmp() -> PathBuf {
		let dir = std::env::temp_dir().join(format!("tw-plan-{}-{}", std::process::id(), rand_suffix()));
		fs::create_dir_all(&dir).unwrap();
		dir
	}

	fn rand_suffix() -> u32 {
		static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
		NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
	}

	#[test]
	fn stems() {
		assert_eq!(stem_of(Path::new("a/photo.final.jpg")), "photo.final");
		assert_eq!(stem_of(Path::new("backup.tar.gz")), "backup");
		assert_eq!(stem_of(Path::new(".env")), ".env");
		assert_eq!(stem_of(Path::new("README")), "README");
	}

	#[test]
	fn unique_names_never_overwrite() {
		let dir = tmp();
		assert_eq!(unique_path(&dir, "a", "png"), dir.join("a.png"));
		fs::write(dir.join("a.png"), b"x").unwrap();
		assert_eq!(unique_path(&dir, "a", "png"), dir.join("a (1).png"));
		fs::write(dir.join("a (1).png"), b"x").unwrap();
		assert_eq!(unique_path(&dir, "a", "png"), dir.join("a (2).png"));
		assert_eq!(unique_path(&dir, "folder", ""), dir.join("folder"));
		fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn convert_lands_next_to_source() {
		let dir = tmp();
		let src = dir.join("photo.heic");
		fs::write(&src, b"x").unwrap();
		let jobs = plan(Op::Convert(Format::Png), std::slice::from_ref(&src));
		assert_eq!(jobs.len(), 1);
		assert_eq!(jobs[0].output, dir.join("photo.png"));
		assert!(matches!(&jobs[0].work, Work::Ffmpeg { input, .. } if *input == src));
		fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn png_compress_uses_quantizer() {
		let jobs = plan(Op::CompressImage, &[PathBuf::from("/x/shot.png")]);
		assert_eq!(jobs[0].output, PathBuf::from("/x/shot-small.png"));
		assert!(matches!(jobs[0].work, Work::PngQuant { .. }));
	}

	#[test]
	fn heic_tools_write_jpg() {
		let jobs = plan(Op::HalfSize, &[PathBuf::from("/x/IMG.HEIC")]);
		assert_eq!(jobs[0].output, PathBuf::from("/x/IMG-half.jpg"));
	}

	#[test]
	fn many_files_pack_into_one_archive() {
		let jobs = plan(Op::Zip, &[PathBuf::from("/x/a.txt"), PathBuf::from("/x/b.txt")]);
		assert_eq!(jobs.len(), 1);
		assert_eq!(jobs[0].output, PathBuf::from("/x/Archive.zip"));
	}

	#[test]
	fn archives_repack_one_by_one() {
		let jobs = plan(
			Op::Convert(Format::SevenZ),
			&[PathBuf::from("/x/a.zip"), PathBuf::from("/x/b.tar.gz")],
		);
		assert_eq!(jobs.len(), 2);
		assert_eq!(jobs[1].output, PathBuf::from("/x/b.7z"));
	}

	#[test]
	fn actions_and_mismatches_plan_nothing() {
		assert!(plan(Op::CopyPath, &[PathBuf::from("/x/a.png")]).is_empty());
		assert!(plan(Op::Mute, &[PathBuf::from("/x/a.png")]).is_empty());
	}

	#[test]
	fn image_outputs_name_their_muxer() {
		for f in [
			Format::Jpg,
			Format::Png,
			Format::Webp,
			Format::Avif,
			Format::Gif,
			Format::Ico,
		] {
			let args = image_args(f, None, ImageQuality::High, false);
			let i = args.iter().position(|a| a == "-f").unwrap();
			assert_eq!(args[i + 1], muxer(f));
		}
	}
}
