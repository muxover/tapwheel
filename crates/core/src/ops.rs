use std::path::Path;

use crate::kind::{Kind, common_kind, ext_of, kind_of};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Wheel {
	Convert,
	Tools,
	Actions,
}

impl Wheel {
	pub fn title(self) -> &'static str {
		match self {
			Wheel::Convert => "Convert",
			Wheel::Tools => "Tools",
			Wheel::Actions => "Actions",
		}
	}

	pub const ALL: [Wheel; 3] = [Wheel::Convert, Wheel::Tools, Wheel::Actions];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
	Jpg,
	Png,
	Webp,
	Avif,
	Tiff,
	Bmp,
	Gif,
	Ico,
	Mp3,
	M4a,
	Wav,
	Flac,
	Ogg,
	Opus,
	Mp4,
	Mov,
	Mkv,
	Webm,
	Avi,
	Srt,
	Vtt,
	Txt,
	Zip,
	SevenZ,
	Tar,
	Tgz,
}

impl Format {
	pub fn ext(self) -> &'static str {
		match self {
			Format::Jpg => "jpg",
			Format::Png => "png",
			Format::Webp => "webp",
			Format::Avif => "avif",
			Format::Tiff => "tiff",
			Format::Bmp => "bmp",
			Format::Gif => "gif",
			Format::Ico => "ico",
			Format::Mp3 => "mp3",
			Format::M4a => "m4a",
			Format::Wav => "wav",
			Format::Flac => "flac",
			Format::Ogg => "ogg",
			Format::Opus => "opus",
			Format::Mp4 => "mp4",
			Format::Mov => "mov",
			Format::Mkv => "mkv",
			Format::Webm => "webm",
			Format::Avi => "avi",
			Format::Srt => "srt",
			Format::Vtt => "vtt",
			Format::Txt => "txt",
			Format::Zip => "zip",
			Format::SevenZ => "7z",
			Format::Tar => "tar",
			Format::Tgz => "tar.gz",
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Format::Jpg => "JPG",
			Format::Png => "PNG",
			Format::Webp => "WebP",
			Format::Avif => "AVIF",
			Format::Tiff => "TIFF",
			Format::Bmp => "BMP",
			Format::Gif => "GIF",
			Format::Ico => "ICO",
			Format::Mp3 => "MP3",
			Format::M4a => "M4A",
			Format::Wav => "WAV",
			Format::Flac => "FLAC",
			Format::Ogg => "OGG",
			Format::Opus => "Opus",
			Format::Mp4 => "MP4",
			Format::Mov => "MOV",
			Format::Mkv => "MKV",
			Format::Webm => "WebM",
			Format::Avi => "AVI",
			Format::Srt => "SRT",
			Format::Vtt => "VTT",
			Format::Txt => "TXT",
			Format::Zip => "ZIP",
			Format::SevenZ => "7Z",
			Format::Tar => "TAR",
			Format::Tgz => "TAR.GZ",
		}
	}

	/// Extensions that already are this format, so a file is never offered its own format.
	fn same_as(self, ext: &str) -> bool {
		match self {
			Format::Jpg => matches!(ext, "jpg" | "jpeg" | "jfif"),
			Format::Tiff => matches!(ext, "tif" | "tiff"),
			Format::Ogg => matches!(ext, "ogg" | "oga"),
			Format::Tgz => ext == "tgz",
			f => f.ext() == ext,
		}
	}

	pub fn is_archive(self) -> bool {
		matches!(self, Format::Zip | Format::SevenZ | Format::Tar | Format::Tgz)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
	Convert(Format),
	CompressImage,
	HalfSize,
	FitHd,
	Rotate,
	StripMetadata,
	CompressVideo,
	Video720,
	Mute,
	ExtractAudio,
	CompressAudio,
	Normalize,
	Extract,
	Zip,
	CopyPath,
	CopyFile,
	Terminal,
	Reveal,
}

impl Op {
	pub fn label(self) -> &'static str {
		match self {
			Op::Convert(f) => f.label(),
			Op::CompressImage | Op::CompressVideo | Op::CompressAudio => "Compress",
			Op::HalfSize => "Half size",
			Op::FitHd => "Fit 1920",
			Op::Rotate => "Rotate",
			Op::StripMetadata => "Strip data",
			Op::Video720 => "720p",
			Op::Mute => "Mute",
			Op::ExtractAudio => "Audio",
			Op::Normalize => "Normalize",
			Op::Extract => "Extract",
			Op::Zip => "ZIP",
			Op::CopyPath => "Copy path",
			Op::CopyFile => "Copy",
			Op::Terminal => "Terminal",
			Op::Reveal => "Show",
		}
	}

	/// What the slice does, for the wheel's centre and for screen readers.
	pub fn describe(self) -> String {
		match self {
			Op::Convert(f) if f.is_archive() => format!("Pack into {}", f.label()),
			Op::Convert(f) => format!("Convert to {}", f.label()),
			Op::CompressImage => "Make the image smaller".into(),
			Op::HalfSize => "Resize to half".into(),
			Op::FitHd => "Fit inside 1920 pixels".into(),
			Op::Rotate => "Rotate a quarter turn".into(),
			Op::StripMetadata => "Remove location and camera data".into(),
			Op::CompressVideo => "Make the video smaller".into(),
			Op::Video720 => "Scale down to 720p".into(),
			Op::Mute => "Remove the sound".into(),
			Op::ExtractAudio => "Save the sound as M4A".into(),
			Op::CompressAudio => "Make the audio smaller".into(),
			Op::Normalize => "Even out the volume".into(),
			Op::Extract => "Unpack here".into(),
			Op::Zip => "Pack into one ZIP".into(),
			Op::CopyPath => "Copy the full path".into(),
			Op::CopyFile => "Copy to paste elsewhere".into(),
			Op::Terminal => "Open a terminal here".into(),
			Op::Reveal => "Show in File Explorer".into(),
		}
	}

	/// Runs in the app on the UI thread instead of as a background job.
	pub fn is_action(self) -> bool {
		matches!(self, Op::CopyPath | Op::CopyFile | Op::Terminal | Op::Reveal)
	}
}

const IMAGE_TARGETS: &[Format] = &[
	Format::Png,
	Format::Jpg,
	Format::Webp,
	Format::Avif,
	Format::Gif,
	Format::Tiff,
	Format::Bmp,
	Format::Ico,
];
const AUDIO_TARGETS: &[Format] = &[
	Format::Mp3,
	Format::M4a,
	Format::Wav,
	Format::Flac,
	Format::Ogg,
	Format::Opus,
];
const VIDEO_TARGETS: &[Format] = &[
	Format::Mp4,
	Format::Mov,
	Format::Mkv,
	Format::Webm,
	Format::Avi,
	Format::Gif,
	Format::Mp3,
];
const SUBTITLE_TARGETS: &[Format] = &[Format::Srt, Format::Vtt, Format::Txt];
const ARCHIVE_TARGETS: &[Format] = &[Format::Zip, Format::SevenZ, Format::Tar, Format::Tgz];

pub const MAX_SLICES: usize = 8;

/// The slices a wheel shows for these files, in order.
pub fn choices(wheel: Wheel, paths: &[impl AsRef<Path>]) -> Vec<Op> {
	if paths.is_empty() {
		return Vec::new();
	}
	let kind = common_kind(paths);
	let exts: Vec<String> = paths.iter().map(|p| ext_of(p.as_ref())).collect();
	let not_same = |f: &&Format| !exts.iter().all(|e| f.same_as(e));

	let mut ops: Vec<Op> = match wheel {
		Wheel::Convert => {
			let targets: Vec<Format> = match kind {
				Some(Kind::Image) => {
					let mut t: Vec<Format> = IMAGE_TARGETS.iter().filter(not_same).copied().collect();
					// an animated GIF is more useful as a video than as an icon
					if exts.iter().all(|e| e == "gif") {
						t.retain(|f| *f != Format::Ico);
						t.push(Format::Mp4);
					}
					t
				}
				Some(Kind::Audio) => AUDIO_TARGETS.iter().filter(not_same).copied().collect(),
				Some(Kind::Video) => VIDEO_TARGETS.iter().filter(not_same).copied().collect(),
				Some(Kind::Subtitle) => SUBTITLE_TARGETS.iter().filter(not_same).copied().collect(),
				Some(Kind::Archive) => ARCHIVE_TARGETS.iter().filter(not_same).copied().collect(),
				Some(Kind::Folder) | Some(Kind::Other) | None => ARCHIVE_TARGETS.to_vec(),
			};
			targets.into_iter().map(Op::Convert).collect()
		}
		Wheel::Tools => {
			let mut t = match kind {
				Some(Kind::Image) => vec![
					Op::CompressImage,
					Op::HalfSize,
					Op::FitHd,
					Op::Rotate,
					Op::StripMetadata,
				],
				Some(Kind::Video) => {
					vec![Op::CompressVideo, Op::Video720, Op::Mute, Op::ExtractAudio]
				}
				Some(Kind::Audio) => vec![Op::CompressAudio, Op::Normalize],
				Some(Kind::Archive) => vec![Op::Extract],
				_ => Vec::new(),
			};
			t.push(Op::Zip);
			t
		}
		Wheel::Actions => vec![Op::CopyPath, Op::CopyFile, Op::Terminal, Op::Reveal],
	};
	ops.truncate(MAX_SLICES);
	ops
}

/// Whether a wheel has anything for these files.
pub fn has_choices(wheel: Wheel, paths: &[impl AsRef<Path>]) -> bool {
	!choices(wheel, paths).is_empty()
}

/// Kind used to pick icons and wording for a whole selection.
pub fn selection_kind(paths: &[impl AsRef<Path>]) -> Option<Kind> {
	match paths {
		[one] => Some(kind_of(one.as_ref())),
		_ => common_kind(paths),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	fn p(names: &[&str]) -> Vec<PathBuf> {
		names.iter().map(PathBuf::from).collect()
	}

	#[test]
	fn image_never_offers_its_own_format() {
		let ops = choices(Wheel::Convert, &p(&["photo.JPEG"]));
		assert!(!ops.contains(&Op::Convert(Format::Jpg)));
		assert!(ops.contains(&Op::Convert(Format::Png)));
		assert!(ops.len() <= MAX_SLICES);
	}

	#[test]
	fn heic_gets_every_image_target() {
		let ops = choices(Wheel::Convert, &p(&["IMG_0001.HEIC"]));
		assert_eq!(ops.len(), IMAGE_TARGETS.len());
	}

	#[test]
	fn gif_can_become_a_video() {
		let ops = choices(Wheel::Convert, &p(&["funny.gif"]));
		assert!(ops.contains(&Op::Convert(Format::Mp4)));
		assert!(!ops.contains(&Op::Convert(Format::Gif)));
	}

	#[test]
	fn mixed_files_can_only_be_packed() {
		let ops = choices(Wheel::Convert, &p(&["a.png", "b.mp3"]));
		assert!(ops.iter().all(|o| matches!(o, Op::Convert(f) if f.is_archive())));
	}

	#[test]
	fn tools_always_end_with_zip() {
		for files in [p(&["a.mp4"]), p(&["a.txt"]), p(&["a.png", "b.png"])] {
			assert_eq!(choices(Wheel::Tools, &files).last(), Some(&Op::Zip));
		}
	}

	#[test]
	fn actions_for_anything() {
		assert_eq!(choices(Wheel::Actions, &p(&["x.bin"])).len(), 4);
		let none: Vec<PathBuf> = Vec::new();
		assert!(choices(Wheel::Actions, &none).is_empty());
	}

	#[test]
	fn same_archive_format_is_not_offered() {
		let ops = choices(Wheel::Convert, &p(&["backup.tar.gz"]));
		assert!(!ops.contains(&Op::Convert(Format::Tgz)));
		assert!(ops.contains(&Op::Convert(Format::Zip)));
	}
}
