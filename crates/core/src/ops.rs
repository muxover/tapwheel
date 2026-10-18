use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::format::Format;
use crate::kind::{Kind, common_ext, common_kind, ext_of, kind_of};

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

/// One slice on a wheel. The same tool can mean different work for different kinds of file,
/// so `Trim` on a video and `Trim` on a song share an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
	Convert(Format),
	// pictures
	Crop,
	AddBackground,
	RemoveBackground,
	Adjust,
	Annotate,
	Redact,
	Resize,
	Compress,
	Rotate,
	Metadata,
	MakeIcon,
	Trace,
	Combine,
	// video and sound
	Trim,
	Speed,
	MakeGif,
	Snapshot,
	Mute,
	ExtractAudio,
	Join,
	Reverse,
	Fade,
	Normalize,
	Volume,
	Channels,
	Ringtone,
	RemoveSilence,
	// icons
	IconSet,
	Favicon,
	WindowsIcon,
	MacIcon,
	ExtractIcons,
	// text and code
	ReadAloud,
	ToUtf8,
	LineEndings,
	CleanUp,
	SortLines,
	Dedupe,
	ChangeCase,
	Stats,
	Pretty,
	Minify,
	Merge,
	Split,
	CodeImage,
	Indent,
	CopyMarkdown,
	// archives and folders
	Extract,
	Protect,
	Test,
	Contents,
	Zip,
	// actions
	Copy,
	CopyPath,
	Rename,
	Duplicate,
	Info,
	Properties,
	OpenWith,
	MoveTo,
	Share,
	Float,
	Hash,
	Terminal,
}

/// How a slice runs once something is dropped on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
	/// Runs straight away in the background with the saved settings.
	Run,
	/// Opens a window to set it up first.
	Editor,
	/// Done by the app itself, immediately.
	Action,
}

/// A choice on the outer ring around a slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset {
	pub id: &'static str,
	pub label: &'static str,
}

const fn p(id: &'static str, label: &'static str) -> Preset {
	Preset { id, label }
}

/// The ring choice that opens the tool's window instead of running it.
pub const CUSTOM: Preset = p("custom", "Custom");

impl Op {
	pub fn label(self) -> &'static str {
		match self {
			Op::Convert(f) => f.label(),
			Op::Crop => "Crop",
			Op::AddBackground => "Add BG",
			Op::RemoveBackground => "Remove BG",
			Op::Adjust => "Adjust",
			Op::Annotate => "Annotate",
			Op::Redact => "Redact",
			Op::Resize => "Resize",
			Op::Compress => "Compress",
			Op::Rotate => "Rotate",
			Op::Metadata => "Metadata",
			Op::MakeIcon => "Make icon",
			Op::Trace => "To SVG",
			Op::Combine => "Combine",
			Op::Trim => "Trim",
			Op::Speed => "Speed",
			Op::MakeGif => "Make GIF",
			Op::Snapshot => "Snapshot",
			Op::Mute => "Mute",
			Op::ExtractAudio => "Get audio",
			Op::Join => "Join",
			Op::Reverse => "Reverse",
			Op::Fade => "Fade",
			Op::Normalize => "Normalize",
			Op::Volume => "Volume",
			Op::Channels => "Mono",
			Op::Ringtone => "Ringtone",
			Op::RemoveSilence => "No silence",
			Op::IconSet => "PNG set",
			Op::Favicon => "Favicon",
			Op::WindowsIcon => "Windows",
			Op::MacIcon => "macOS",
			Op::ExtractIcons => "Get icons",
			Op::ReadAloud => "Read aloud",
			Op::ToUtf8 => "UTF-8",
			Op::LineEndings => "Line ends",
			Op::CleanUp => "Clean up",
			Op::SortLines => "Sort",
			Op::Dedupe => "Dedupe",
			Op::ChangeCase => "Case",
			Op::Stats => "Count",
			Op::Pretty => "Format",
			Op::Minify => "Minify",
			Op::Merge => "Merge",
			Op::Split => "Split",
			Op::CodeImage => "Code shot",
			Op::Indent => "Indent",
			Op::CopyMarkdown => "Copy as MD",
			Op::Extract => "Extract",
			Op::Protect => "Password",
			Op::Test => "Test",
			Op::Contents => "Contents",
			Op::Zip => "ZIP",
			Op::Copy => "Copy",
			Op::CopyPath => "Copy path",
			Op::Rename => "Rename",
			Op::Duplicate => "Duplicate",
			Op::Info => "File info",
			Op::Properties => "Properties",
			Op::OpenWith => "Open with",
			Op::MoveTo => "Move to",
			Op::Share => "Share",
			Op::Float => "Float",
			Op::Hash => "Checksum",
			Op::Terminal => "Terminal",
		}
	}

	/// What the slice does, for the wheel's centre and for screen readers.
	pub fn describe(self, kind: Option<Kind>) -> String {
		let k = kind.unwrap_or(Kind::Other);
		match self {
			Op::Convert(f) if f.is_archive() => format!("Pack into {}", f.label()),
			Op::Convert(Format::M4r) => "Make an iPhone ringtone".into(),
			Op::Convert(Format::Png) if k == Kind::Code => "Save as a picture".into(),
			Op::Convert(Format::Html) if k == Kind::Code => "Save as a highlighted page".into(),
			Op::Convert(f) => format!("Convert to {}", f.label()),
			Op::Crop => "Cut to a size or shape".into(),
			Op::AddBackground => "Put it on a backdrop".into(),
			Op::RemoveBackground => "Cut out the subject".into(),
			Op::Adjust => "Light, colour and contrast".into(),
			Op::Annotate => "Draw, point and write".into(),
			Op::Redact => "Hide private parts".into(),
			Op::Resize if k == Kind::Video => "Change the resolution".into(),
			Op::Resize => "Change the size".into(),
			Op::Compress => "Make it smaller".into(),
			Op::Rotate => "Turn or flip".into(),
			Op::Metadata if k == Kind::Audio => "Title, artist and cover".into(),
			Op::Metadata => "See or remove hidden data".into(),
			Op::MakeIcon => "Windows icon with every size".into(),
			Op::Trace => "Trace into a vector".into(),
			Op::Combine => "Lay pictures out together".into(),
			Op::Trim => "Keep or cut a part".into(),
			Op::Speed => "Faster or slower".into(),
			Op::MakeGif => "Turn a part into a GIF".into(),
			Op::Snapshot => "Save a frame as a picture".into(),
			Op::Mute => "Remove the sound".into(),
			Op::ExtractAudio => "Save the sound alone".into(),
			Op::Join => "Join into one file".into(),
			Op::Reverse => "Play it backwards".into(),
			Op::Fade => "Fade in or out".into(),
			Op::Normalize => "Even out the loudness".into(),
			Op::Volume => "Louder or quieter".into(),
			Op::Channels => "Mono or stereo".into(),
			Op::Ringtone => "Make an iPhone ringtone".into(),
			Op::RemoveSilence => "Cut quiet gaps".into(),
			Op::IconSet => "PNGs from 16 to 1024".into(),
			Op::Favicon => "Every website icon".into(),
			Op::WindowsIcon => "ICO with every size".into(),
			Op::MacIcon => "ICNS for macOS".into(),
			Op::ExtractIcons if k == Kind::Program => "Save the program's icon".into(),
			Op::ExtractIcons => "Every size as PNG".into(),
			Op::ReadAloud => "Save as speech".into(),
			Op::ToUtf8 => "Fix the text encoding".into(),
			Op::LineEndings => "Windows or Unix line ends".into(),
			Op::CleanUp => "Trim spaces and blank lines".into(),
			Op::SortLines => "Sort the lines".into(),
			Op::Dedupe => "Remove repeated lines".into(),
			Op::ChangeCase => "UPPER, lower or Title".into(),
			Op::Stats => "Count lines and words".into(),
			Op::Pretty => "Tidy the layout".into(),
			Op::Minify => "Squeeze out the spaces".into(),
			Op::Merge => "Join into one file".into(),
			Op::Split => "Cut into smaller files".into(),
			Op::CodeImage => "A picture of your code".into(),
			Op::Indent => "Tabs or spaces".into(),
			Op::CopyMarkdown => "Copy as a code block".into(),
			Op::Extract => "Unpack here".into(),
			Op::Protect => "Pack with a password".into(),
			Op::Test => "Check it isn't damaged".into(),
			Op::Contents => "See what's inside".into(),
			Op::Zip => "Pack into one ZIP".into(),
			Op::Copy => "Copy to paste elsewhere".into(),
			Op::CopyPath => "Copy the full path".into(),
			Op::Rename => "New names, one or many".into(),
			Op::Duplicate => "A copy right next to it".into(),
			Op::Info => "Size, real type and details".into(),
			Op::Properties => "The Windows properties window".into(),
			Op::OpenWith => "Pick an app to open it".into(),
			Op::MoveTo => "Move to a folder".into(),
			Op::Share => "Send with Windows share".into(),
			Op::Float => "Keep it handy for later".into(),
			Op::Hash => "Copy a checksum".into(),
			Op::Terminal => "Open a terminal here".into(),
		}
	}

	/// The Fluent icon drawn on the slice.
	pub fn icon(self, kind: Option<Kind>) -> &'static str {
		match self {
			Op::Convert(_) => "",
			Op::Crop => "crop",
			Op::AddBackground => "image_add",
			Op::RemoveBackground => "image_sparkle",
			Op::Adjust => "options",
			Op::Annotate => "pen",
			Op::Redact => "eye_off",
			Op::Resize => "resize",
			Op::Compress => "arrow_minimize",
			Op::Rotate => "arrow_rotate_clockwise",
			Op::Metadata if kind == Some(Kind::Audio) => "tag",
			Op::Metadata => "info",
			Op::MakeIcon => "app_generic",
			Op::Trace => "shape_union",
			Op::Combine => "grid",
			Op::Trim => "cut",
			Op::Speed => "top_speed",
			Op::MakeGif => "gif",
			Op::Snapshot => "camera",
			Op::Mute => "speaker_mute",
			Op::ExtractAudio => "music_note_2",
			Op::Join => "arrow_join",
			Op::Reverse => "arrow_hook_up_left",
			Op::Fade => "data_area",
			Op::Normalize => "data_bar_vertical",
			Op::Volume => "speaker_2",
			Op::Channels => "headphones",
			Op::Ringtone => "phone",
			Op::RemoveSilence => "timer_off",
			Op::IconSet => "image_multiple",
			Op::Favicon => "globe",
			Op::WindowsIcon => "window",
			Op::MacIcon => "laptop",
			Op::ExtractIcons => "apps",
			Op::ReadAloud => "read_aloud",
			Op::ToUtf8 => "text_font",
			Op::LineEndings => "text_wrap",
			Op::CleanUp => "eraser",
			Op::SortLines => "text_sort_ascending",
			Op::Dedupe => "text_bullet_list_square",
			Op::ChangeCase => "text_case_title",
			Op::Stats => "number_symbol",
			Op::Pretty => "text_align_left",
			Op::Minify => "text_collapse",
			Op::Merge => "merge",
			Op::Split => "split_vertical",
			Op::CodeImage => "code_block",
			Op::Indent => "text_indent_increase",
			Op::CopyMarkdown => "markdown",
			Op::Extract => "archive_arrow_back",
			Op::Protect => "lock_closed",
			Op::Test => "shield_checkmark",
			Op::Contents => "list",
			Op::Zip => "folder_zip",
			Op::Copy => "copy",
			Op::CopyPath => "link",
			Op::Rename => "rename",
			Op::Duplicate => "document_copy",
			Op::Info => "document_search",
			Op::Properties => "wrench",
			Op::OpenWith => "open",
			Op::MoveTo => "folder_arrow_right",
			Op::Share => "share",
			Op::Float => "pin",
			Op::Hash => "fingerprint",
			Op::Terminal => "window_console",
		}
	}

	pub fn mode(self) -> Mode {
		match self {
			Op::Crop
			| Op::AddBackground
			| Op::Adjust
			| Op::Annotate
			| Op::Redact
			| Op::Trim
			| Op::MakeGif
			| Op::Combine
			| Op::CodeImage
			| Op::Protect
			| Op::Contents
			| Op::Rename
			| Op::Info => Mode::Editor,
			Op::Copy
			| Op::CopyPath
			| Op::Properties
			| Op::OpenWith
			| Op::MoveTo
			| Op::Share
			| Op::Float
			| Op::Hash
			| Op::Terminal
			| Op::CopyMarkdown
			| Op::Stats => Mode::Action,
			_ => Mode::Run,
		}
	}

	/// Choices on the ring around the slice. The first one is what a plain drop on the slice does.
	pub fn presets(self, kind: Option<Kind>) -> &'static [Preset] {
		const IMG_COMPRESS: &[Preset] = &[
			p("balanced", "Balanced"),
			p("smaller", "Smaller"),
			p("best", "Best"),
			CUSTOM,
		];
		const VID_COMPRESS: &[Preset] = &[
			p("balanced", "Balanced"),
			p("smaller", "Smaller"),
			p("25mb", "25 MB"),
			p("10mb", "10 MB"),
			CUSTOM,
		];
		const AUD_COMPRESS: &[Preset] = &[p("128k", "128k"), p("96k", "96k"), p("64k", "64k"), p("192k", "192k")];
		const IMG_RESIZE: &[Preset] = &[
			p("half", "50%"),
			p("1920", "1920"),
			p("1280", "1280"),
			p("1080", "1080"),
			CUSTOM,
		];
		const VID_RESIZE: &[Preset] = &[
			p("720", "720p"),
			p("1080", "1080p"),
			p("480", "480p"),
			p("vertical", "9:16"),
		];
		const ROTATE: &[Preset] = &[
			p("cw", "Right"),
			p("ccw", "Left"),
			p("180", "180°"),
			p("flip_h", "Mirror"),
			p("flip_v", "Flip"),
		];
		const SPEED: &[Preset] = &[p("2", "2×"), p("1.5", "1.5×"), p("4", "4×"), p("0.5", "0.5×")];
		const FADE: &[Preset] = &[p("both", "Both"), p("in", "In"), p("out", "Out")];
		const VOLUME: &[Preset] = &[p("+6", "+6 dB"), p("+3", "+3 dB"), p("-3", "−3 dB"), p("-6", "−6 dB")];
		const CHANNELS: &[Preset] = &[p("mono", "Mono"), p("stereo", "Stereo")];
		const SNAPSHOT: &[Preset] = &[
			p("middle", "Middle"),
			p("first", "First"),
			p("every", "Every 10 s"),
			CUSTOM,
		];
		const IMG_META: &[Preset] = &[p("all", "Remove all"), p("location", "Location"), CUSTOM];
		const AUD_META: &[Preset] = &[CUSTOM, p("all", "Remove all")];
		const SORT: &[Preset] = &[
			p("az", "A to Z"),
			p("za", "Z to A"),
			p("number", "Numbers"),
			p("length", "Length"),
		];
		const CASE: &[Preset] = &[
			p("title", "Title"),
			p("upper", "UPPER"),
			p("lower", "lower"),
			p("sentence", "Sentence"),
		];
		const ENDINGS: &[Preset] = &[p("crlf", "Windows"), p("lf", "Unix")];
		const INDENT: &[Preset] = &[p("tabs", "Tabs"), p("2", "2 spaces"), p("4", "4 spaces")];
		const SPLIT_TEXT: &[Preset] = &[
			p("1000", "1,000 lines"),
			p("100", "100 lines"),
			p("10000", "10,000 lines"),
		];
		const SPLIT_ARCHIVE: &[Preset] = &[p("25", "25 MB"), p("100", "100 MB"), p("1000", "1 GB")];
		const COPY_PATH: &[Preset] = &[
			p("path", "Path"),
			p("name", "Name"),
			p("stem", "Base name"),
			p("ext", "Extension"),
			p("quoted", "Quoted"),
			p("uri", "File URL"),
		];
		const HASH: &[Preset] = &[
			p("sha256", "SHA-256"),
			p("md5", "MD5"),
			p("sha1", "SHA-1"),
			p("check", "Compare"),
		];
		const COMBINE: &[Preset] = &[
			p("grid", "Grid"),
			p("row", "Side by side"),
			p("column", "Stacked"),
			p("gif", "GIF"),
			CUSTOM,
		];
		const ICON_SIZES: &[Preset] = &[
			p("all", "All sizes"),
			p("256", "256"),
			p("512", "512"),
			p("1024", "1024"),
		];
		match (self, kind) {
			(Op::Compress, Some(Kind::Video)) => VID_COMPRESS,
			(Op::Compress, Some(Kind::Audio)) => AUD_COMPRESS,
			(Op::Compress, _) => IMG_COMPRESS,
			(Op::Resize, Some(Kind::Video)) => VID_RESIZE,
			(Op::Resize, _) => IMG_RESIZE,
			(Op::Rotate, _) => ROTATE,
			(Op::Speed, _) => SPEED,
			(Op::Fade, _) => FADE,
			(Op::Volume, _) => VOLUME,
			(Op::Channels, _) => CHANNELS,
			(Op::Snapshot, _) => SNAPSHOT,
			(Op::Metadata, Some(Kind::Audio)) => AUD_META,
			(Op::Metadata, _) => IMG_META,
			(Op::SortLines, _) => SORT,
			(Op::ChangeCase, _) => CASE,
			(Op::LineEndings, _) => ENDINGS,
			(Op::Indent, _) => INDENT,
			(Op::Split, Some(Kind::Archive)) => SPLIT_ARCHIVE,
			(Op::Split, _) => SPLIT_TEXT,
			(Op::Hash, _) => HASH,
			(Op::CopyPath, _) => COPY_PATH,
			(Op::Combine, _) => COMBINE,
			(Op::IconSet, _) => ICON_SIZES,
			_ => &[],
		}
	}

	/// Stable name for settings, usage counts and editor messages.
	pub fn id(self) -> String {
		match self {
			Op::Convert(f) => format!("to-{}", f.id()),
			o => {
				let s = serde_json::to_value(o).ok();
				s.and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
			}
		}
	}

	pub fn from_id(id: &str) -> Option<Op> {
		if let Some(f) = id.strip_prefix("to-") {
			return Format::from_id(f).map(Op::Convert);
		}
		serde_json::from_value(serde_json::Value::String(id.into())).ok()
	}
}

/// Formats a kind of file can become, best first.
fn targets(kind: Kind, exts: &[String]) -> Vec<Format> {
	use Format::*;
	let all = |e: &str| exts.iter().all(|x| x == e);
	let any_of = |list: &[&str]| exts.iter().all(|x| list.contains(&x.as_str()));
	match kind {
		Kind::Image if all("gif") => vec![Mp4, Webm, Png, Jpg, Webp],
		Kind::Image => vec![Png, Jpg, Webp, Avif, Gif, Tiff, Bmp, Ico, Tga, Qoi],
		Kind::Icon if any_of(&["svg", "svgz"]) => vec![Png, Ico, Icns, Webp, Jpg],
		Kind::Icon => vec![Png, Ico, Icns, Webp, Jpg],
		Kind::Video => vec![Mp4, Mov, Mkv, Webm, Gif, Mp3, Avi, Wmv, M4a, Wav],
		Kind::Audio => vec![Mp3, M4a, Wav, Flac, Ogg, Opus, Aiff, Wma, M4r],
		Kind::Text
			if any_of(&[
				"csv", "tsv", "psv", "json", "jsonl", "ndjson", "geojson", "yaml", "yml", "toml", "xml", "plist",
			]) =>
		{
			vec![Json, Csv, Yaml, Xml, Toml, Tsv, Jsonl]
		}
		Kind::Text if any_of(&["ini", "cfg", "conf", "properties", "env"]) => vec![Json, Yaml, Toml],
		Kind::Text if any_of(&["md", "markdown", "mdown", "mkd"]) => vec![Html, Txt],
		Kind::Text if any_of(&["html", "htm", "xhtml"]) => vec![Md, Txt],
		Kind::Text if any_of(&["srt", "vtt", "ass", "ssa", "sub"]) => vec![Srt, Vtt, Txt],
		Kind::Text if any_of(&["rtf"]) => vec![Txt, Md, Html],
		Kind::Text => vec![Md, Html, Txt],
		Kind::Code if any_of(&["json"]) => vec![Yaml, Toml, Xml, Csv],
		Kind::Code => vec![Png, Html, Md, Txt],
		Kind::Archive => vec![Zip, SevenZ, Tgz, Txz, Tar, Tbz2, Tzst],
		Kind::Folder | Kind::Program | Kind::Other => vec![Zip, SevenZ, Tgz, Txz, Tar],
	}
}

/// Text files whose lines belong to a structure: data, config, markup and subtitles.
const STRUCTURED_TEXT: &[&str] = &[
	"json",
	"json5",
	"geojson",
	"plist",
	"yaml",
	"yml",
	"toml",
	"xml",
	"html",
	"htm",
	"xhtml",
	"csv",
	"tsv",
	"psv",
	"ini",
	"cfg",
	"conf",
	"properties",
	"env",
	"srt",
	"vtt",
	"ass",
	"ssa",
	"sub",
	"ics",
	"vcf",
	"bib",
];

fn tools(kind: Option<Kind>, count: usize, exts: &[String]) -> Vec<Op> {
	use Op::*;
	let many = count > 1;
	let any_of = |list: &[&str]| exts.iter().all(|x| list.contains(&x.as_str()));
	let mut t = match kind {
		Some(Kind::Image) => {
			let mut t = vec![
				Crop,
				AddBackground,
				RemoveBackground,
				Compress,
				Resize,
				Adjust,
				Annotate,
				Redact,
				Rotate,
				Metadata,
				MakeIcon,
				Trace,
			];
			if many {
				t.insert(0, Combine);
			}
			t
		}
		Some(Kind::Video) => {
			let mut t = vec![
				Trim,
				Compress,
				MakeGif,
				Resize,
				Crop,
				Speed,
				Snapshot,
				Mute,
				ExtractAudio,
				Rotate,
				Reverse,
			];
			if many {
				t.insert(0, Join);
			}
			t
		}
		Some(Kind::Audio) => {
			let mut t = vec![
				Trim,
				Fade,
				Normalize,
				Compress,
				Volume,
				Speed,
				Metadata,
				Ringtone,
				Channels,
				RemoveSilence,
				Reverse,
			];
			if many {
				t.insert(0, Join);
			}
			t
		}
		Some(Kind::Icon) if any_of(&["svg", "svgz"]) => vec![WindowsIcon, Favicon, IconSet, MacIcon, Minify],
		Some(Kind::Icon) => vec![ExtractIcons, IconSet, Favicon, WindowsIcon, MacIcon],
		Some(Kind::Program) => vec![ExtractIcons],
		Some(Kind::Text) => {
			// Data, config and subtitle files have a structure that sorting, changing case or
			// splitting lines would break, and reading them aloud makes no sense; records one per
			// line can still be split and deduplicated.
			let records = any_of(&["jsonl", "ndjson", "log"]);
			let structured = !records && any_of(STRUCTURED_TEXT);
			let mut t = if structured || records { vec![] } else { vec![ReadAloud] };
			if any_of(crate::code::PRETTY_EXTS) {
				t.push(Pretty);
			}
			if any_of(crate::code::MINIFY_EXTS) {
				t.push(Minify);
			}
			t.push(ToUtf8);
			// trimming line ends would drop a TSV row's empty last column
			if !any_of(&["tsv"]) {
				t.push(CleanUp);
			}
			if records {
				t.extend([Dedupe, Split]);
			} else if !structured {
				t.extend([SortLines, Dedupe, ChangeCase, Split]);
			}
			t.extend([LineEndings, Stats]);
			if many {
				t.insert(0, Merge);
			}
			t
		}
		Some(Kind::Code) => {
			let mut t = vec![CodeImage, CopyMarkdown];
			if any_of(crate::code::PRETTY_EXTS) {
				t.push(Pretty);
			}
			if any_of(crate::code::MINIFY_EXTS) {
				t.push(Minify);
			}
			t.extend([Indent, LineEndings, CleanUp, ToUtf8]);
			if many {
				t.push(Merge);
			}
			t
		}
		Some(Kind::Archive) => vec![Extract, Contents, Test, Protect, Split],
		Some(Kind::Folder) => vec![Protect],
		_ => Vec::new(),
	};
	if kind != Some(Kind::Archive) {
		t.push(Zip);
	}
	t
}

/// The slices a wheel shows for these files, in order. The wheel pages through more than eight.
pub fn choices(wheel: Wheel, paths: &[impl AsRef<Path>]) -> Vec<Op> {
	if paths.is_empty() {
		return Vec::new();
	}
	let kind = common_kind(paths);
	let exts: Vec<String> = paths.iter().map(|p| ext_of(p.as_ref())).collect();
	match wheel {
		Wheel::Convert => {
			let not_same = |f: &Format| !exts.iter().all(|e| f.same_as(e));
			let fs = match kind {
				Some(k) => targets(k, &exts),
				None => targets(Kind::Other, &exts),
			};
			fs.into_iter().filter(not_same).map(Op::Convert).collect()
		}
		Wheel::Tools => tools(kind, paths.len(), &exts),
		Wheel::Actions => {
			let mut a = vec![
				Op::Copy,
				Op::CopyPath,
				Op::Rename,
				Op::MoveTo,
				Op::Duplicate,
				Op::Share,
				Op::Info,
				Op::Float,
			];
			if paths.iter().all(|p| !p.as_ref().is_dir()) {
				a.push(Op::Hash);
			}
			if paths.len() == 1 {
				a.push(Op::Properties);
				if !paths[0].as_ref().is_dir() {
					a.push(Op::OpenWith);
				}
			} else {
				// file info describes one item
				a.retain(|o| *o != Op::Info);
			}
			if paths
				.iter()
				.any(|p| matches!(kind_of(p.as_ref()), Kind::Folder | Kind::Code))
			{
				a.push(Op::Terminal);
			}
			a
		}
	}
}

/// Kind used to pick icons and wording for a whole selection.
pub fn selection_kind(paths: &[impl AsRef<Path>]) -> Option<Kind> {
	match paths {
		[one] => Some(kind_of(one.as_ref())),
		_ => common_kind(paths),
	}
}

/// A short word for the selection's format, like "HEIC" or "3 PNGs", for the wheel's centre.
pub fn selection_badge(paths: &[impl AsRef<Path>]) -> String {
	match common_ext(paths) {
		Some(e) if !e.is_empty() => e.to_uppercase(),
		_ => String::new(),
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
	}

	#[test]
	fn gif_can_become_a_video() {
		let ops = choices(Wheel::Convert, &p(&["funny.gif"]));
		assert_eq!(ops[0], Op::Convert(Format::Mp4));
		assert!(!ops.contains(&Op::Convert(Format::Gif)));
	}

	#[test]
	fn data_files_convert_into_each_other() {
		let ops = choices(Wheel::Convert, &p(&["rows.csv"]));
		assert!(ops.contains(&Op::Convert(Format::Json)));
		assert!(!ops.contains(&Op::Convert(Format::Csv)));
		let ops = choices(Wheel::Convert, &p(&["README.md"]));
		assert_eq!(ops[0], Op::Convert(Format::Html));
	}

	#[test]
	fn mixed_files_can_only_be_packed() {
		let ops = choices(Wheel::Convert, &p(&["a.png", "b.mp3"]));
		assert!(ops.iter().all(|o| matches!(o, Op::Convert(f) if f.is_archive())));
	}

	#[test]
	fn several_files_can_be_joined() {
		assert_eq!(choices(Wheel::Tools, &p(&["a.mp4", "b.mp4"]))[0], Op::Join);
		assert_eq!(choices(Wheel::Tools, &p(&["a.png", "b.png"]))[0], Op::Combine);
		assert!(!choices(Wheel::Tools, &p(&["a.mp4"])).contains(&Op::Join));
	}

	#[test]
	fn every_kind_has_tools_and_zip_ends_them() {
		for f in ["a.png", "a.mp4", "a.mp3", "a.ico", "a.txt", "a.rs", "a.exe"] {
			let t = choices(Wheel::Tools, &p(&[f]));
			assert!(t.len() >= 2, "{f}");
			assert_eq!(t.last(), Some(&Op::Zip), "{f}");
		}
		assert!(!choices(Wheel::Tools, &p(&["a.zip"])).contains(&Op::Zip));
	}

	#[test]
	fn terminal_only_for_folders_and_code() {
		assert!(!choices(Wheel::Actions, &p(&["x.png"])).contains(&Op::Terminal));
		assert!(choices(Wheel::Actions, &p(&["main.rs"])).contains(&Op::Terminal));
	}

	#[test]
	fn structured_text_only_gets_tools_that_keep_it_whole() {
		let tools = |f: &str| choices(Wheel::Tools, &[PathBuf::from(f)]);
		for f in ["a.json", "a.yaml", "a.csv", "a.tsv", "a.srt", "a.ini", "a.xml"] {
			let t = tools(f);
			for bad in [Op::SortLines, Op::ChangeCase, Op::ReadAloud, Op::Split] {
				assert!(!t.contains(&bad), "{f} offers {bad:?}");
			}
		}
		assert!(
			!tools("a.tsv").contains(&Op::CleanUp),
			"trimming drops empty last columns"
		);
		assert!(tools("a.jsonl").contains(&Op::Split) && tools("a.jsonl").contains(&Op::Dedupe));
		assert!(tools("notes.txt").contains(&Op::SortLines) && tools("notes.txt").contains(&Op::Stats));
		// MIDI is notes, not sound: nothing here can play it
		assert_ne!(crate::kind::kind_of(std::path::Path::new("song.mid")), Kind::Audio);
	}

	#[test]
	fn ids_round_trip() {
		for op in [Op::Crop, Op::RemoveBackground, Op::Convert(Format::Tgz), Op::CopyPath] {
			assert_eq!(Op::from_id(&op.id()), Some(op), "{op:?}");
		}
		assert_eq!(Op::RemoveBackground.id(), "remove_background");
	}

	#[test]
	fn presets_exist_for_ring_tools() {
		assert!(!Op::Compress.presets(Some(Kind::Video)).is_empty());
		assert!(Op::Crop.presets(Some(Kind::Image)).is_empty());
	}
}
