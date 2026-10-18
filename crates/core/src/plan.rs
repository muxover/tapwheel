use std::path::{Path, PathBuf};

use crate::format::Format;
use crate::kind::{Kind, common_kind, ext_of, is_raw, kind_of};
use crate::ops::Op;
use crate::recipe::{Case, Layout, Recipe, SnapAt, SortBy, Turn};

/// One unit of background work and the path it will produce.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
	pub op: Op,
	pub recipe: Recipe,
	pub inputs: Vec<PathBuf>,
	/// Empty for checks that only report, like testing an archive.
	pub output: PathBuf,
	/// Whether the engine reports a duration we can turn into a percentage.
	pub timed: bool,
}

/// File name without its extension, treating `.tar.gz` and friends as one extension.
pub fn stem_of(path: &Path) -> String {
	let name = path
		.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default();
	if path.is_dir() {
		return name;
	}
	let lower = name.to_lowercase();
	for double in [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"] {
		if lower.ends_with(double) {
			return name[..name.len() - double.len()].to_string();
		}
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

pub fn parent(path: &Path) -> PathBuf {
	path.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// Picture formats FFmpeg can write back as themselves.
const STILL_ENCODABLE: &[&str] = &[
	"jpg", "jpeg", "jfif", "jpe", "png", "webp", "avif", "tif", "tiff", "bmp", "gif", "tga", "qoi",
];
const AUDIO_LOSSY: &[&str] = &["mp3", "m4a", "aac", "ogg", "oga", "opus", "wma"];

pub fn image_format_for(ext: &str) -> Option<Format> {
	Some(match ext {
		"jpg" | "jpeg" | "jfif" | "jpe" => Format::Jpg,
		"png" | "apng" => Format::Png,
		"webp" => Format::Webp,
		"avif" => Format::Avif,
		"tif" | "tiff" => Format::Tiff,
		"bmp" | "dib" => Format::Bmp,
		"gif" => Format::Gif,
		"tga" => Format::Tga,
		"qoi" => Format::Qoi,
		_ => return None,
	})
}

/// Keeps a picture's own format when we can write it. Camera files and HEIC become JPG,
/// layered and HDR files become PNG so nothing is flattened into a lossy copy.
pub fn same_image(ext: &str) -> Format {
	if STILL_ENCODABLE.contains(&ext) {
		return image_format_for(ext).unwrap_or(Format::Jpg);
	}
	if is_raw(ext) || matches!(ext, "heic" | "heif" | "hif" | "jxl" | "jp2" | "j2k") {
		Format::Jpg
	} else {
		Format::Png
	}
}

pub fn audio_format_for(ext: &str) -> Format {
	match ext {
		"m4a" | "aac" | "alac" | "m4b" => Format::M4a,
		"wav" | "wave" | "w64" => Format::Wav,
		"flac" => Format::Flac,
		"ogg" | "oga" => Format::Ogg,
		"opus" => Format::Opus,
		"aif" | "aiff" | "aifc" => Format::Aiff,
		"wma" => Format::Wma,
		"m4r" => Format::M4r,
		_ => Format::Mp3,
	}
}

pub fn video_format_for(ext: &str) -> Format {
	match ext {
		"mov" | "qt" => Format::Mov,
		"mkv" => Format::Mkv,
		"webm" => Format::Webm,
		"avi" => Format::Avi,
		"wmv" | "asf" => Format::Wmv,
		_ => Format::Mp4,
	}
}

/// Turns a recipe into jobs. One job per file, or one for the whole selection when the recipe joins them.
pub fn plan(op: Op, recipe: &Recipe, paths: &[PathBuf]) -> Vec<Job> {
	if paths.is_empty() {
		return Vec::new();
	}
	let packs_many =
		matches!(recipe, Recipe::Convert { to } if to.is_archive()) && common_kind(paths) != Some(Kind::Archive);
	if recipe.takes_all() || packs_many {
		return whole_job(op, recipe, paths).into_iter().collect();
	}
	paths.iter().filter_map(|p| file_job(op, recipe, p)).collect()
}

fn whole_job(op: Op, recipe: &Recipe, paths: &[PathBuf]) -> Option<Job> {
	let first = &paths[0];
	let dir = parent(first);
	let one = paths.len() == 1;
	let ext = ext_of(first);
	let (stem, out_ext): (String, String) = match recipe {
		Recipe::Convert { to } | Recipe::Pack { format: to } | Recipe::Protect { format: to, .. } => {
			let stem = if one { stem_of(first) } else { "Archive".into() };
			(stem, to.ext().into())
		}
		Recipe::Join => {
			let to = match kind_of(first) {
				Kind::Audio => audio_format_for(&ext),
				_ => video_format_for(&ext),
			};
			("Joined".into(), to.ext().into())
		}
		Recipe::Merge => ("Merged".into(), if ext.is_empty() { "txt".into() } else { ext }),
		Recipe::Combine { layout, .. } => {
			let to = if *layout == Layout::Gif { "gif" } else { "png" };
			("Combined".into(), to.into())
		}
		_ => return None,
	};
	Some(Job {
		op,
		recipe: recipe.clone(),
		inputs: paths.to_vec(),
		output: unique_path(&dir, &stem, &out_ext),
		timed: matches!(recipe, Recipe::Join),
	})
}

fn file_job(op: Op, recipe: &Recipe, input: &Path) -> Option<Job> {
	let ext = ext_of(input);
	let kind = kind_of(input);
	let dir = parent(input);
	let stem = stem_of(input);
	let file = |suffix: &str, ext: &str| {
		let s = if suffix.is_empty() {
			stem.clone()
		} else {
			format!("{stem}-{suffix}")
		};
		unique_path(&dir, &s, ext)
	};
	let folder = |name: &str| unique_path(&dir, &format!("{stem} {name}"), "");
	let video = kind == Kind::Video;
	let audio = kind == Kind::Audio;
	let media = video || audio;
	let own = |suffix: &str| {
		let to = if video {
			video_format_for(&ext)
		} else if audio {
			audio_format_for(&ext)
		} else {
			same_image(&ext)
		};
		file(suffix, to.ext())
	};
	let turned = |t: &Turn| match t {
		Turn::FlipH => "mirrored",
		Turn::FlipV => "flipped",
		_ => "rotated",
	};

	let output = match recipe {
		Recipe::Convert { to } if to.is_archive() => file("", to.ext()),
		Recipe::Convert { to } => file("", to.ext()),
		Recipe::CompressImage { .. } => {
			let to = match same_image(&ext) {
				Format::Tiff | Format::Bmp | Format::Tga | Format::Qoi => Format::Jpg,
				f => f,
			};
			file("small", to.ext())
		}
		Recipe::ResizeImage { width, percent, .. } => {
			let tag = match (percent, width) {
				(Some(p), _) => format!("{p}%"),
				(None, Some(w)) => w.to_string(),
				_ => "resized".into(),
			};
			own(&tag)
		}
		Recipe::RotateImage { turn } => own(turned(turn)),
		Recipe::CropImage { .. } => own("cropped"),
		Recipe::StripMetadata { .. } => own("clean"),
		Recipe::RemoveBackground => file("cutout", "png"),
		Recipe::Trace => file("", "svg"),
		Recipe::Rendered { suffix, to, .. } => {
			let to = to.unwrap_or_else(|| match kind {
				Kind::Code | Kind::Text => Format::Png,
				_ => same_image(&ext),
			});
			file(suffix, to.ext())
		}
		Recipe::Trim {
			separate: true, keep, ..
		} if keep.len() > 1 => folder("parts"),
		Recipe::Trim { .. } => own("trimmed"),
		Recipe::CropVideo { .. } => own("cropped"),
		Recipe::CompressVideo { .. } => file("small", "mp4"),
		Recipe::ResizeVideo { height, vertical, .. } => {
			let tag = if *vertical {
				"vertical".to_string()
			} else {
				format!("{height}p")
			};
			file(&tag, "mp4")
		}
		Recipe::Speed { factor } => own(&format!("{factor}x")),
		Recipe::MakeGif { .. } => file("", "gif"),
		Recipe::Snapshot { at: SnapAt::Every(_) } => folder("frames"),
		Recipe::Snapshot { .. } => file("frame", "png"),
		Recipe::Mute => own("muted"),
		Recipe::ExtractAudio => file("", "m4a"),
		Recipe::RotateVideo { turn } => own(turned(turn)),
		Recipe::Reverse => own("reversed"),
		Recipe::Fade { .. } => own("faded"),
		Recipe::Normalize => own("normalized"),
		Recipe::Volume { db } => own(if *db >= 0.0 { "louder" } else { "quieter" }),
		Recipe::Channels { mono } => own(if *mono { "mono" } else { "stereo" }),
		Recipe::CompressAudio { .. } => {
			let to = if AUDIO_LOSSY.contains(&ext.as_str()) {
				audio_format_for(&ext)
			} else {
				Format::Mp3
			};
			file("small", to.ext())
		}
		Recipe::Ringtone => file("", "m4r"),
		Recipe::RemoveSilence { .. } => own("tight"),
		Recipe::Tags { .. } => own("tagged"),
		Recipe::IconSet { sizes } if sizes.len() == 1 => file(&sizes[0].to_string(), "png"),
		Recipe::IconSet { .. } | Recipe::ExtractIcons => folder("icons"),
		Recipe::Favicon => folder("favicon"),
		Recipe::WindowsIcon => file("", "ico"),
		Recipe::MacIcon => file("", "icns"),
		Recipe::ReadAloud { .. } => file("", "mp3"),
		Recipe::ToUtf8 => file("utf8", &ext),
		Recipe::LineEndings { crlf } => file(if *crlf { "crlf" } else { "lf" }, &ext),
		Recipe::CleanUp => file("clean", &ext),
		Recipe::SortLines { by } => file(
			match by {
				SortBy::Za => "sorted-za",
				_ => "sorted",
			},
			&ext,
		),
		Recipe::Dedupe => file("unique", &ext),
		Recipe::ChangeCase { to } => file(
			match to {
				Case::Upper => "upper",
				Case::Lower => "lower",
				Case::Title => "title",
				Case::Sentence => "sentence",
			},
			&ext,
		),
		Recipe::Pretty => file("pretty", &ext),
		Recipe::Minify => file("min", &ext),
		Recipe::SplitText { .. } | Recipe::SplitArchive { .. } => folder("parts"),
		Recipe::Indent { spaces } => file(
			&if *spaces == 0 {
				"tabs".to_string()
			} else {
				format!("{spaces}spaces")
			},
			&ext,
		),
		Recipe::Extract { .. } => unique_path(&dir, &stem, ""),
		Recipe::Test => PathBuf::new(),
		Recipe::Duplicate => unique_path(&dir, &format!("{stem} - Copy"), &ext),
		Recipe::Pack { .. } | Recipe::Join | Recipe::Merge | Recipe::Combine { .. } | Recipe::Protect { .. } => {
			return None;
		}
	};
	// recipes that don't fit this kind of file are skipped rather than failing later
	let fits = match recipe {
		Recipe::Trim { .. }
		| Recipe::Speed { .. }
		| Recipe::Fade { .. }
		| Recipe::Reverse
		| Recipe::Normalize
		| Recipe::Volume { .. }
		| Recipe::Channels { .. } => media,
		Recipe::CropVideo { .. }
		| Recipe::CompressVideo { .. }
		| Recipe::ResizeVideo { .. }
		| Recipe::MakeGif { .. }
		| Recipe::Snapshot { .. }
		| Recipe::Mute
		| Recipe::ExtractAudio
		| Recipe::RotateVideo { .. } => video || (matches!(recipe, Recipe::MakeGif { .. }) && ext == "gif"),
		Recipe::CompressAudio { .. } | Recipe::Ringtone | Recipe::RemoveSilence { .. } | Recipe::Tags { .. } => {
			audio || (video && matches!(recipe, Recipe::Ringtone))
		}
		Recipe::Extract { .. } | Recipe::Test | Recipe::SplitArchive { .. } => kind == Kind::Archive,
		_ => true,
	};
	if !fits {
		return None;
	}
	let timed = media
		&& !matches!(
			recipe,
			Recipe::Snapshot { .. } | Recipe::Tags { .. } | Recipe::Convert { to: Format::Gif }
		);
	Some(Job {
		op,
		recipe: recipe.clone(),
		inputs: vec![input.to_path_buf()],
		output,
		timed,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::Span;
	use std::fs;

	fn tmp() -> PathBuf {
		static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
		let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let dir = std::env::temp_dir().join(format!("tw-plan-{}-{n}", std::process::id()));
		fs::create_dir_all(&dir).unwrap();
		dir
	}

	fn one(op: Op, r: Recipe, path: &str) -> Job {
		plan(op, &r, &[PathBuf::from(path)]).remove(0)
	}

	#[test]
	fn stems() {
		assert_eq!(stem_of(Path::new("a/photo.final.jpg")), "photo.final");
		assert_eq!(stem_of(Path::new("backup.tar.gz")), "backup");
		assert_eq!(stem_of(Path::new("backup.tar.zst")), "backup");
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
	fn outputs_land_next_to_the_source() {
		let j = one(
			Op::Convert(Format::Png),
			Recipe::Convert { to: Format::Png },
			"/x/photo.heic",
		);
		assert_eq!(j.output, PathBuf::from("/x/photo.png"));
		let j = one(
			Op::Compress,
			Recipe::CompressImage {
				quality: 70,
				max_edge: None,
				target_kb: None,
			},
			"/x/IMG.HEIC",
		);
		assert_eq!(j.output, PathBuf::from("/x/IMG-small.jpg"));
		let j = one(Op::RemoveBackground, Recipe::RemoveBackground, "/x/me.jpg");
		assert_eq!(j.output, PathBuf::from("/x/me-cutout.png"));
		let j = one(Op::Minify, Recipe::Minify, "/x/site.css");
		assert_eq!(j.output, PathBuf::from("/x/site-min.css"));
	}

	#[test]
	fn trims_keep_the_container() {
		let r = Recipe::Trim {
			keep: vec![Span { start: 1.0, end: 2.0 }],
			separate: false,
			exact: true,
		};
		assert_eq!(
			one(Op::Trim, r.clone(), "/x/clip.mov").output,
			PathBuf::from("/x/clip-trimmed.mov")
		);
		assert_eq!(
			one(Op::Trim, r, "/x/song.flac").output,
			PathBuf::from("/x/song-trimmed.flac")
		);
	}

	#[test]
	fn many_files_become_one_when_joined() {
		let files = [PathBuf::from("/x/a.mp4"), PathBuf::from("/x/b.mp4")];
		let jobs = plan(Op::Join, &Recipe::Join, &files);
		assert_eq!(jobs.len(), 1);
		assert_eq!(jobs[0].output, PathBuf::from("/x/Joined.mp4"));
		let jobs = plan(Op::Zip, &Recipe::Pack { format: Format::Zip }, &files);
		assert_eq!(jobs[0].output, PathBuf::from("/x/Archive.zip"));
	}

	#[test]
	fn archives_repack_one_by_one() {
		let r = Recipe::Convert { to: Format::SevenZ };
		let jobs = plan(
			Op::Convert(Format::SevenZ),
			&r,
			&[PathBuf::from("/x/a.zip"), PathBuf::from("/x/b.tar.gz")],
		);
		assert_eq!(jobs.len(), 2);
		assert_eq!(jobs[1].output, PathBuf::from("/x/b.7z"));
	}

	#[test]
	fn mismatches_plan_nothing() {
		assert!(plan(Op::Mute, &Recipe::Mute, &[PathBuf::from("/x/a.png")]).is_empty());
		assert!(
			plan(
				Op::Extract,
				&Recipe::Extract { only: Vec::new() },
				&[PathBuf::from("/x/a.txt")]
			)
			.is_empty()
		);
	}
}
