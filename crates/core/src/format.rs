use serde::{Deserialize, Serialize};

use crate::kind::Kind;

/// Every format TapWheel can write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
	// images
	Jpg,
	Png,
	Webp,
	Avif,
	Gif,
	Tiff,
	Bmp,
	Tga,
	Qoi,
	// icons
	Ico,
	Icns,
	Svg,
	// video
	Mp4,
	Mov,
	Mkv,
	Webm,
	Avi,
	Wmv,
	// audio
	Mp3,
	M4a,
	Wav,
	Flac,
	Ogg,
	Opus,
	Aiff,
	Wma,
	M4r,
	// text
	Txt,
	Md,
	Html,
	Csv,
	Tsv,
	Json,
	Jsonl,
	Yaml,
	Toml,
	Xml,
	Srt,
	Vtt,
	// archives
	Zip,
	SevenZ,
	Tar,
	Tgz,
	Txz,
	Tbz2,
	Tzst,
}

impl Format {
	pub fn ext(self) -> &'static str {
		match self {
			Format::Jpg => "jpg",
			Format::Png => "png",
			Format::Webp => "webp",
			Format::Avif => "avif",
			Format::Gif => "gif",
			Format::Tiff => "tiff",
			Format::Bmp => "bmp",
			Format::Tga => "tga",
			Format::Qoi => "qoi",
			Format::Ico => "ico",
			Format::Icns => "icns",
			Format::Svg => "svg",
			Format::Mp4 => "mp4",
			Format::Mov => "mov",
			Format::Mkv => "mkv",
			Format::Webm => "webm",
			Format::Avi => "avi",
			Format::Wmv => "wmv",
			Format::Mp3 => "mp3",
			Format::M4a => "m4a",
			Format::Wav => "wav",
			Format::Flac => "flac",
			Format::Ogg => "ogg",
			Format::Opus => "opus",
			Format::Aiff => "aiff",
			Format::Wma => "wma",
			Format::M4r => "m4r",
			Format::Txt => "txt",
			Format::Md => "md",
			Format::Html => "html",
			Format::Csv => "csv",
			Format::Tsv => "tsv",
			Format::Json => "json",
			Format::Jsonl => "jsonl",
			Format::Yaml => "yaml",
			Format::Toml => "toml",
			Format::Xml => "xml",
			Format::Srt => "srt",
			Format::Vtt => "vtt",
			Format::Zip => "zip",
			Format::SevenZ => "7z",
			Format::Tar => "tar",
			Format::Tgz => "tar.gz",
			Format::Txz => "tar.xz",
			Format::Tbz2 => "tar.bz2",
			Format::Tzst => "tar.zst",
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Format::Jpg => "JPG",
			Format::Png => "PNG",
			Format::Webp => "WebP",
			Format::Avif => "AVIF",
			Format::Gif => "GIF",
			Format::Tiff => "TIFF",
			Format::Bmp => "BMP",
			Format::Tga => "TGA",
			Format::Qoi => "QOI",
			Format::Ico => "ICO",
			Format::Icns => "ICNS",
			Format::Svg => "SVG",
			Format::Mp4 => "MP4",
			Format::Mov => "MOV",
			Format::Mkv => "MKV",
			Format::Webm => "WebM",
			Format::Avi => "AVI",
			Format::Wmv => "WMV",
			Format::Mp3 => "MP3",
			Format::M4a => "M4A",
			Format::Wav => "WAV",
			Format::Flac => "FLAC",
			Format::Ogg => "OGG",
			Format::Opus => "Opus",
			Format::Aiff => "AIFF",
			Format::Wma => "WMA",
			Format::M4r => "M4R",
			Format::Txt => "TXT",
			Format::Md => "Markdown",
			Format::Html => "HTML",
			Format::Csv => "CSV",
			Format::Tsv => "TSV",
			Format::Json => "JSON",
			Format::Jsonl => "JSONL",
			Format::Yaml => "YAML",
			Format::Toml => "TOML",
			Format::Xml => "XML",
			Format::Srt => "SRT",
			Format::Vtt => "VTT",
			Format::Zip => "ZIP",
			Format::SevenZ => "7Z",
			Format::Tar => "TAR",
			Format::Tgz => "TAR.GZ",
			Format::Txz => "TAR.XZ",
			Format::Tbz2 => "TAR.BZ2",
			Format::Tzst => "TAR.ZST",
		}
	}

	/// Short stable name used in settings and editor messages.
	pub fn id(self) -> &'static str {
		match self {
			Format::SevenZ => "7z",
			Format::Tgz => "tgz",
			Format::Txz => "txz",
			Format::Tbz2 => "tbz2",
			Format::Tzst => "tzst",
			f => f.ext(),
		}
	}

	pub fn from_id(id: &str) -> Option<Format> {
		ALL.iter().copied().find(|f| f.id() == id)
	}

	/// Extensions that already are this format, so a file is never offered its own format.
	pub fn same_as(self, ext: &str) -> bool {
		match self {
			Format::Jpg => matches!(ext, "jpg" | "jpeg" | "jfif" | "jpe"),
			Format::Tiff => matches!(ext, "tif" | "tiff"),
			Format::Bmp => matches!(ext, "bmp" | "dib"),
			Format::Ogg => matches!(ext, "ogg" | "oga"),
			Format::Aiff => matches!(ext, "aif" | "aiff" | "aifc"),
			Format::Wav => matches!(ext, "wav" | "wave"),
			Format::Md => matches!(ext, "md" | "markdown" | "mdown" | "mkd"),
			Format::Html => matches!(ext, "html" | "htm" | "xhtml"),
			Format::Yaml => matches!(ext, "yaml" | "yml"),
			Format::Jsonl => matches!(ext, "jsonl" | "ndjson"),
			Format::Mov => matches!(ext, "mov" | "qt"),
			Format::Svg => matches!(ext, "svg" | "svgz"),
			Format::Txt => matches!(ext, "txt" | "text"),
			Format::Tbz2 => matches!(ext, "tbz2" | "tbz"),
			f => f.id() == ext,
		}
	}

	pub fn kind(self) -> Kind {
		use Format::*;
		match self {
			Jpg | Png | Webp | Avif | Gif | Tiff | Bmp | Tga | Qoi => Kind::Image,
			Ico | Icns | Svg => Kind::Icon,
			Mp4 | Mov | Mkv | Webm | Avi | Wmv => Kind::Video,
			Mp3 | M4a | Wav | Flac | Ogg | Opus | Aiff | Wma | M4r => Kind::Audio,
			Txt | Md | Html | Csv | Tsv | Json | Jsonl | Yaml | Toml | Xml | Srt | Vtt => Kind::Text,
			Zip | SevenZ | Tar | Tgz | Txz | Tbz2 | Tzst => Kind::Archive,
		}
	}

	pub fn is_archive(self) -> bool {
		self.kind() == Kind::Archive
	}

	pub fn is_audio(self) -> bool {
		self.kind() == Kind::Audio
	}

	/// Data formats that hold rows or trees and convert into each other.
	pub fn is_data(self) -> bool {
		matches!(
			self,
			Format::Csv | Format::Tsv | Format::Json | Format::Jsonl | Format::Yaml | Format::Toml | Format::Xml
		)
	}
}

pub const ALL: &[Format] = &[
	Format::Jpg,
	Format::Png,
	Format::Webp,
	Format::Avif,
	Format::Gif,
	Format::Tiff,
	Format::Bmp,
	Format::Tga,
	Format::Qoi,
	Format::Ico,
	Format::Icns,
	Format::Svg,
	Format::Mp4,
	Format::Mov,
	Format::Mkv,
	Format::Webm,
	Format::Avi,
	Format::Wmv,
	Format::Mp3,
	Format::M4a,
	Format::Wav,
	Format::Flac,
	Format::Ogg,
	Format::Opus,
	Format::Aiff,
	Format::Wma,
	Format::M4r,
	Format::Txt,
	Format::Md,
	Format::Html,
	Format::Csv,
	Format::Tsv,
	Format::Json,
	Format::Jsonl,
	Format::Yaml,
	Format::Toml,
	Format::Xml,
	Format::Srt,
	Format::Vtt,
	Format::Zip,
	Format::SevenZ,
	Format::Tar,
	Format::Tgz,
	Format::Txz,
	Format::Tbz2,
	Format::Tzst,
];

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ids_round_trip() {
		for f in ALL {
			assert_eq!(Format::from_id(f.id()), Some(*f), "{f:?}");
			assert!(f.same_as(f.id()) || f.same_as(f.ext()), "{f:?}");
		}
	}
}
