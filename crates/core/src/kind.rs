use std::path::Path;

/// The families TapWheel knows. Each one gets its own wheels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
	Image,
	Video,
	Audio,
	Icon,
	Text,
	Code,
	Archive,
	/// Programs and libraries, which carry icons worth pulling out.
	Program,
	Folder,
	Other,
}

impl Kind {
	/// Plural noun for "3 images" in titles and cards.
	pub fn plural(self) -> &'static str {
		match self {
			Kind::Image => "images",
			Kind::Video => "videos",
			Kind::Audio => "sounds",
			Kind::Icon => "icons",
			Kind::Text => "text files",
			Kind::Code => "code files",
			Kind::Archive => "archives",
			Kind::Program => "programs",
			Kind::Folder => "folders",
			Kind::Other => "items",
		}
	}
}

pub const IMAGE_EXTS: &[&str] = &[
	"jpg", "jpeg", "jfif", "jpe", "png", "apng", "webp", "avif", "heic", "heif", "hif", "tif", "tiff", "bmp", "dib",
	"gif", "jxl", "tga", "qoi", "dds", "psd", "pcx", "ppm", "pgm", "pbm", "pam", "exr", "hdr", "jp2", "j2k", "dng",
	"cr2", "cr3", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "raf", "pef", "srw", "3fr", "erf", "kdc", "mrw",
	"x3f",
];
pub const RAW_EXTS: &[&str] = &[
	"dng", "cr2", "cr3", "nef", "nrw", "arw", "srf", "sr2", "orf", "rw2", "raf", "pef", "srw", "3fr", "erf", "kdc",
	"mrw", "x3f",
];
pub const VIDEO_EXTS: &[&str] = &[
	"mp4", "m4v", "mov", "qt", "mkv", "webm", "avi", "wmv", "asf", "mpg", "mpeg", "m2v", "m2ts", "3gp", "3g2", "flv",
	"f4v", "ogv", "vob", "mxf", "dv", "rm", "rmvb", "divx",
];
pub const AUDIO_EXTS: &[&str] = &[
	"mp3", "m4a", "m4b", "m4r", "aac", "wav", "wave", "flac", "ogg", "oga", "opus", "aif", "aiff", "aifc", "wma",
	"alac", "amr", "ac3", "eac3", "dts", "caf", "mka", "mp2", "ape", "wv", "tta", "au", "snd", "voc", "w64", "spx",
];
pub const ICON_EXTS: &[&str] = &["ico", "icns", "cur", "svg", "svgz"];
pub const PROGRAM_EXTS: &[&str] = &["exe", "dll", "cpl", "scr", "ocx", "mui", "sys"];
pub const TEXT_EXTS: &[&str] = &[
	"txt",
	"text",
	"md",
	"markdown",
	"mdown",
	"mkd",
	"rst",
	"adoc",
	"org",
	"html",
	"htm",
	"xhtml",
	"rtf",
	"csv",
	"tsv",
	"psv",
	"json",
	"jsonl",
	"ndjson",
	"json5",
	"geojson",
	"yaml",
	"yml",
	"toml",
	"xml",
	"plist",
	"ini",
	"cfg",
	"conf",
	"properties",
	"env",
	"log",
	"srt",
	"vtt",
	"ass",
	"ssa",
	"sub",
	"nfo",
	"tex",
	"bib",
	"ics",
	"vcf",
];
pub const CODE_EXTS: &[&str] = &[
	"rs",
	"py",
	"pyw",
	"pyi",
	"js",
	"mjs",
	"cjs",
	"jsx",
	"ts",
	"mts",
	"cts",
	"tsx",
	"go",
	"c",
	"h",
	"cc",
	"cpp",
	"cxx",
	"hpp",
	"hh",
	"hxx",
	"cs",
	"csx",
	"vb",
	"fs",
	"java",
	"kt",
	"kts",
	"scala",
	"groovy",
	"gradle",
	"swift",
	"m",
	"mm",
	"rb",
	"php",
	"pl",
	"pm",
	"lua",
	"r",
	"jl",
	"dart",
	"zig",
	"nim",
	"ex",
	"exs",
	"erl",
	"hs",
	"clj",
	"elm",
	"ml",
	"sh",
	"bash",
	"zsh",
	"fish",
	"ps1",
	"psm1",
	"psd1",
	"bat",
	"cmd",
	"sql",
	"css",
	"scss",
	"sass",
	"less",
	"vue",
	"svelte",
	"astro",
	"graphql",
	"gql",
	"proto",
	"dockerfile",
	"makefile",
	"cmake",
	"tf",
	"hcl",
	"nix",
	"glsl",
	"hlsl",
	"wgsl",
	"asm",
	"s",
	"v",
	"sv",
	"vhd",
	"sol",
	"gd",
	"ahk",
	"au3",
	"reg",
];
pub const ARCHIVE_EXTS: &[&str] = &[
	"zip", "7z", "tar", "tgz", "gz", "tbz", "tbz2", "bz2", "txz", "xz", "tzst", "zst", "jar", "apk", "cbz", "epub",
	"nupkg", "vsix", "whl",
];

/// Lowercase extension, with double extensions like `.tar.gz` folded into one (`tgz`).
pub fn ext_of(path: &Path) -> String {
	let name = path
		.file_name()
		.map(|n| n.to_string_lossy().to_lowercase())
		.unwrap_or_default();
	for (double, single) in [
		(".tar.gz", "tgz"),
		(".tar.bz2", "tbz2"),
		(".tar.xz", "txz"),
		(".tar.zst", "tzst"),
	] {
		if name.ends_with(double) {
			return single.into();
		}
	}
	match name.as_str() {
		"dockerfile" | "containerfile" => return "dockerfile".into(),
		"makefile" | "gnumakefile" => return "makefile".into(),
		"cmakelists.txt" => return "cmake".into(),
		_ => {}
	}
	path.extension()
		.map(|e| e.to_string_lossy().to_lowercase())
		.unwrap_or_default()
}

pub fn kind_of(path: &Path) -> Kind {
	if path.is_dir() {
		return Kind::Folder;
	}
	let ext = ext_of(path);
	if matches!(ext.as_str(), "ts" | "mts") && is_transport_stream(path) {
		return Kind::Video;
	}
	kind_of_ext(&ext)
}

/// `.ts` and `.mts` are TypeScript and also camera or broadcast video. Video packets start
/// with 0x47 every 188 bytes, or every 192 bytes after a 4-byte timestamp on camera files.
fn is_transport_stream(path: &Path) -> bool {
	use std::io::Read;
	let mut head = [0u8; 389];
	match std::fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)) {
		Ok(()) => {
			(head[0] == 0x47 && head[188] == 0x47 && head[376] == 0x47)
				|| (head[4] == 0x47 && head[196] == 0x47 && head[388] == 0x47)
		}
		Err(_) => false,
	}
}

pub fn kind_of_ext(ext: &str) -> Kind {
	let ext = ext.trim_start_matches('.');
	let lists: [(&[&str], Kind); 8] = [
		(IMAGE_EXTS, Kind::Image),
		(VIDEO_EXTS, Kind::Video),
		(AUDIO_EXTS, Kind::Audio),
		(ICON_EXTS, Kind::Icon),
		(CODE_EXTS, Kind::Code),
		(TEXT_EXTS, Kind::Text),
		(ARCHIVE_EXTS, Kind::Archive),
		(PROGRAM_EXTS, Kind::Program),
	];
	lists
		.iter()
		.find(|(exts, _)| exts.contains(&ext))
		.map(|(_, k)| *k)
		.unwrap_or(Kind::Other)
}

pub fn is_raw(ext: &str) -> bool {
	RAW_EXTS.contains(&ext)
}

/// The kind shared by every path, or `None` for a mixed selection.
pub fn common_kind(paths: &[impl AsRef<Path>]) -> Option<Kind> {
	let mut kinds = paths.iter().map(|p| kind_of(p.as_ref()));
	let first = kinds.next()?;
	kinds.all(|k| k == first).then_some(first)
}

/// The extension shared by every path, if there is one.
pub fn common_ext(paths: &[impl AsRef<Path>]) -> Option<String> {
	let mut exts = paths.iter().map(|p| ext_of(p.as_ref()));
	let first = exts.next()?;
	exts.all(|e| e == first).then_some(first)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn kinds_by_extension() {
		assert_eq!(kind_of(Path::new("a/b/Song.FLAC")), Kind::Audio);
		assert_eq!(kind_of(Path::new("clip.mkv")), Kind::Video);
		assert_eq!(kind_of(Path::new("subs.vtt")), Kind::Text);
		assert_eq!(kind_of(Path::new("notes.txt")), Kind::Text);
		assert_eq!(kind_of(Path::new("main.rs")), Kind::Code);
		assert_eq!(kind_of(Path::new("logo.svg")), Kind::Icon);
		assert_eq!(kind_of(Path::new("app.ico")), Kind::Icon);
		assert_eq!(kind_of(Path::new("setup.exe")), Kind::Program);
		assert_eq!(kind_of(Path::new("IMG_1.CR3")), Kind::Image);
		assert_eq!(kind_of(Path::new("noext")), Kind::Other);
	}

	#[test]
	fn double_extensions_are_one() {
		assert_eq!(ext_of(Path::new("backup.tar.gz")), "tgz");
		assert_eq!(ext_of(Path::new("backup.tar.xz")), "txz");
		assert_eq!(kind_of(Path::new("backup.tar.zst")), Kind::Archive);
		assert_eq!(ext_of(Path::new("data.gz")), "gz");
		assert_eq!(kind_of(Path::new("Dockerfile")), Kind::Code);
	}

	#[test]
	fn no_extension_is_listed_twice() {
		let all = [
			IMAGE_EXTS,
			VIDEO_EXTS,
			AUDIO_EXTS,
			ICON_EXTS,
			PROGRAM_EXTS,
			TEXT_EXTS,
			CODE_EXTS,
			ARCHIVE_EXTS,
		]
		.concat();
		let mut seen = std::collections::HashSet::new();
		for e in all {
			assert!(seen.insert(e), "{e} is in two lists");
		}
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
