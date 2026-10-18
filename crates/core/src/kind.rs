use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
	Image,
	Audio,
	Video,
	Subtitle,
	Archive,
	Folder,
	Other,
}

pub const IMAGE_EXTS: &[&str] = &[
	"jpg", "jpeg", "jfif", "png", "webp", "avif", "heic", "heif", "tif", "tiff", "bmp", "gif", "ico",
];
pub const AUDIO_EXTS: &[&str] = &[
	"mp3", "m4a", "aac", "wav", "flac", "ogg", "oga", "opus", "aif", "aiff", "wma", "alac",
];
pub const VIDEO_EXTS: &[&str] = &[
	"mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "mpg", "mpeg", "ts", "mts", "m2ts", "3gp", "flv",
];
pub const SUBTITLE_EXTS: &[&str] = &["srt", "vtt"];
pub const ARCHIVE_EXTS: &[&str] = &["zip", "7z", "tar", "tgz", "gz"];

/// Lowercase extension, with `.tar.gz` folded into `tgz`.
pub fn ext_of(path: &Path) -> String {
	let name = path
		.file_name()
		.map(|n| n.to_string_lossy().to_lowercase())
		.unwrap_or_default();
	if name.ends_with(".tar.gz") {
		return "tgz".into();
	}
	path.extension()
		.map(|e| e.to_string_lossy().to_lowercase())
		.unwrap_or_default()
}

pub fn kind_of(path: &Path) -> Kind {
	if path.is_dir() {
		return Kind::Folder;
	}
	kind_of_ext(&ext_of(path))
}

pub fn kind_of_ext(ext: &str) -> Kind {
	let ext = ext.trim_start_matches('.');
	if IMAGE_EXTS.contains(&ext) {
		Kind::Image
	} else if AUDIO_EXTS.contains(&ext) {
		Kind::Audio
	} else if VIDEO_EXTS.contains(&ext) {
		Kind::Video
	} else if SUBTITLE_EXTS.contains(&ext) {
		Kind::Subtitle
	} else if ARCHIVE_EXTS.contains(&ext) {
		Kind::Archive
	} else {
		Kind::Other
	}
}

/// The kind shared by every path, or `None` for a mixed selection.
pub fn common_kind(paths: &[impl AsRef<Path>]) -> Option<Kind> {
	let mut kinds = paths.iter().map(|p| kind_of(p.as_ref()));
	let first = kinds.next()?;
	kinds.all(|k| k == first).then_some(first)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn kinds_by_extension() {
		assert_eq!(kind_of_ext("JPG".to_lowercase().as_str()), Kind::Image);
		assert_eq!(kind_of(Path::new("a/b/Song.FLAC")), Kind::Audio);
		assert_eq!(kind_of(Path::new("clip.mkv")), Kind::Video);
		assert_eq!(kind_of(Path::new("subs.vtt")), Kind::Subtitle);
		assert_eq!(kind_of(Path::new("notes.txt")), Kind::Other);
		assert_eq!(kind_of(Path::new("noext")), Kind::Other);
	}

	#[test]
	fn tar_gz_is_one_extension() {
		assert_eq!(ext_of(Path::new("backup.tar.gz")), "tgz");
		assert_eq!(kind_of(Path::new("backup.tar.gz")), Kind::Archive);
		assert_eq!(ext_of(Path::new("data.gz")), "gz");
	}

	#[test]
	fn mixed_selection_has_no_common_kind() {
		let same = [PathBuf::from("a.png"), PathBuf::from("b.HEIC")];
		assert_eq!(common_kind(&same), Some(Kind::Image));
		let mixed = [PathBuf::from("a.png"), PathBuf::from("b.mp3")];
		assert_eq!(common_kind(&mixed), None);
		let empty: [PathBuf; 0] = [];
		assert_eq!(common_kind(&empty), None);
	}
}
