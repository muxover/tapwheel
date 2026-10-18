use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::format::Format;
use crate::kind::Kind;
use crate::ops::Op;
use crate::settings::Defaults;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Turn {
	Cw,
	Ccw,
	Half,
	FlipH,
	FlipV,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
	Grid,
	Row,
	Column,
	Gif,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
	Az,
	Za,
	Number,
	Length,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Case {
	Upper,
	Lower,
	Title,
	Sentence,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapAt {
	First,
	Middle,
	/// A time in seconds.
	At(f64),
	/// One frame every this many seconds, into a folder.
	Every(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashKind {
	Sha256,
	Sha1,
	Md5,
}

/// One part of a timeline, in seconds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
	pub start: f64,
	pub end: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tags {
	pub title: Option<String>,
	pub artist: Option<String>,
	pub album: Option<String>,
	pub album_artist: Option<String>,
	pub year: Option<String>,
	pub genre: Option<String>,
	pub track: Option<String>,
	pub comment: Option<String>,
	/// A picture to embed as the cover.
	pub cover: Option<PathBuf>,
	/// Drop every tag that isn't set above.
	pub clear: bool,
}

/// Exactly what to do, with every setting filled in. Wheel drops and editor windows both end here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "do", rename_all = "snake_case")]
pub enum Recipe {
	Convert {
		to: Format,
	},
	// pictures
	CompressImage {
		quality: u8,
		max_edge: Option<u32>,
		target_kb: Option<u32>,
	},
	ResizeImage {
		width: Option<u32>,
		height: Option<u32>,
		percent: Option<f32>,
	},
	RotateImage {
		turn: Turn,
	},
	CropImage {
		x: u32,
		y: u32,
		w: u32,
		h: u32,
	},
	StripMetadata {
		location_only: bool,
	},
	RemoveBackground,
	Trace,
	Combine {
		layout: Layout,
		gap: u32,
		background: String,
	},
	/// A picture an editor window drew, saved in the source's format.
	Rendered {
		file: PathBuf,
		suffix: String,
		to: Option<Format>,
	},
	// video and sound
	Trim {
		keep: Vec<Span>,
		/// Each span becomes its own file instead of being joined.
		separate: bool,
		/// Cut on exact frames (re-encode) instead of the nearest keyframe (instant).
		exact: bool,
	},
	CropVideo {
		x: u32,
		y: u32,
		w: u32,
		h: u32,
	},
	CompressVideo {
		crf: u8,
		max_height: Option<u32>,
		target_mb: Option<f64>,
	},
	ResizeVideo {
		height: u32,
		vertical: bool,
	},
	Speed {
		factor: f32,
	},
	MakeGif {
		span: Option<Span>,
		fps: u32,
		width: u32,
	},
	Snapshot {
		at: SnapAt,
	},
	Mute,
	ExtractAudio,
	RotateVideo {
		turn: Turn,
	},
	Reverse,
	Join,
	Fade {
		fade_in: f32,
		fade_out: f32,
	},
	Normalize,
	Volume {
		db: f32,
	},
	Channels {
		mono: bool,
	},
	CompressAudio {
		kbps: u32,
	},
	Ringtone,
	RemoveSilence {
		threshold_db: f32,
		min_gap: f32,
	},
	Tags {
		tags: Tags,
	},
	// icons
	IconSet {
		sizes: Vec<u32>,
	},
	Favicon,
	WindowsIcon,
	MacIcon,
	ExtractIcons,
	// text and code
	ReadAloud {
		voice: Option<String>,
		rate: f32,
	},
	ToUtf8,
	LineEndings {
		crlf: bool,
	},
	CleanUp,
	SortLines {
		by: SortBy,
	},
	Dedupe,
	ChangeCase {
		to: Case,
	},
	Pretty,
	Minify,
	Merge,
	SplitText {
		lines: usize,
	},
	Indent {
		/// 0 means tabs.
		spaces: u8,
	},
	// archives
	Pack {
		format: Format,
	},
	Extract,
	Protect {
		password: String,
		format: Format,
	},
	Test,
	/// A copy beside the original, named the way Windows names copies.
	Duplicate,
	SplitArchive {
		part_mb: u64,
	},
}

impl Recipe {
	/// Recipes that write into a new folder instead of one file.
	pub fn makes_folder(&self) -> bool {
		matches!(
			self,
			Recipe::Extract
				| Recipe::IconSet { .. }
				| Recipe::Favicon
				| Recipe::ExtractIcons
				| Recipe::SplitText { .. }
				| Recipe::SplitArchive { .. }
				| Recipe::Snapshot { at: SnapAt::Every(_) }
		) || matches!(self, Recipe::Trim { separate: true, keep, .. } if keep.len() > 1)
	}

	/// One job for the whole selection instead of one per file.
	pub fn takes_all(&self) -> bool {
		matches!(
			self,
			Recipe::Pack { .. } | Recipe::Join | Recipe::Merge | Recipe::Combine { .. } | Recipe::Protect { .. }
		)
	}
}

/// What a plain drop (or a ring choice) on a slice means, using the saved settings.
/// `None` means the slice needs its window first.
pub fn recipe_for(op: Op, preset: Option<&str>, kind: Option<Kind>, d: &Defaults) -> Option<Recipe> {
	let preset = preset.or_else(|| op.presets(kind).first().map(|p| p.id));
	if preset == Some("custom") {
		return None;
	}
	let pr = preset.unwrap_or("");
	let num = |s: &str| s.trim_start_matches('+').parse::<f32>().ok();
	let turn = || match pr {
		"ccw" => Turn::Ccw,
		"180" => Turn::Half,
		"flip_h" => Turn::FlipH,
		"flip_v" => Turn::FlipV,
		_ => Turn::Cw,
	};
	let video = kind == Some(Kind::Video);
	let r = match op {
		Op::Convert(to) => Recipe::Convert { to },
		Op::Compress if video => match pr {
			"smaller" => Recipe::CompressVideo {
				crf: d.video_crf_small,
				max_height: Some(720),
				target_mb: None,
			},
			"25mb" => Recipe::CompressVideo {
				crf: d.video_crf,
				max_height: None,
				target_mb: Some(24.0),
			},
			"10mb" => Recipe::CompressVideo {
				crf: d.video_crf,
				max_height: None,
				target_mb: Some(9.5),
			},
			_ => Recipe::CompressVideo {
				crf: d.video_crf_balanced,
				max_height: None,
				target_mb: None,
			},
		},
		Op::Compress if kind == Some(Kind::Audio) => Recipe::CompressAudio {
			kbps: pr.trim_end_matches('k').parse().unwrap_or(128),
		},
		Op::Compress => match pr {
			"smaller" => Recipe::CompressImage {
				quality: d.image_quality_small,
				max_edge: Some(2560),
				target_kb: None,
			},
			"best" => Recipe::CompressImage {
				quality: d.image_quality_best,
				max_edge: None,
				target_kb: None,
			},
			_ => Recipe::CompressImage {
				quality: d.image_quality_balanced,
				max_edge: None,
				target_kb: None,
			},
		},
		Op::Resize if video => Recipe::ResizeVideo {
			height: match pr {
				"1080" => 1080,
				"480" => 480,
				_ => 720,
			},
			vertical: pr == "vertical",
		},
		Op::Resize => match pr {
			"half" => Recipe::ResizeImage {
				width: None,
				height: None,
				percent: Some(50.0),
			},
			edge => {
				let e = edge.parse().ok();
				Recipe::ResizeImage {
					width: e,
					height: e,
					percent: None,
				}
			}
		},
		Op::Rotate if video => Recipe::RotateVideo { turn: turn() },
		Op::Rotate => Recipe::RotateImage { turn: turn() },
		Op::Metadata if kind == Some(Kind::Audio) => Recipe::Tags {
			tags: Tags {
				clear: true,
				..Tags::default()
			},
		},
		Op::Metadata => Recipe::StripMetadata {
			location_only: pr == "location",
		},
		Op::RemoveBackground => Recipe::RemoveBackground,
		Op::MakeIcon | Op::WindowsIcon => Recipe::WindowsIcon,
		Op::Trace => Recipe::Trace,
		Op::Combine => Recipe::Combine {
			layout: match pr {
				"row" => Layout::Row,
				"column" => Layout::Column,
				"gif" => Layout::Gif,
				_ => Layout::Grid,
			},
			gap: d.combine_gap,
			background: d.combine_background.clone(),
		},
		Op::Speed => Recipe::Speed {
			factor: num(pr).unwrap_or(2.0),
		},
		Op::Snapshot => Recipe::Snapshot {
			at: match pr {
				"first" => SnapAt::First,
				"every" => SnapAt::Every(10.0),
				_ => SnapAt::Middle,
			},
		},
		Op::Mute => Recipe::Mute,
		Op::ExtractAudio => Recipe::ExtractAudio,
		Op::Reverse => Recipe::Reverse,
		Op::Join => Recipe::Join,
		Op::Fade => Recipe::Fade {
			fade_in: if pr == "out" { 0.0 } else { d.fade_seconds },
			fade_out: if pr == "in" { 0.0 } else { d.fade_seconds },
		},
		Op::Normalize => Recipe::Normalize,
		Op::Volume => Recipe::Volume {
			db: num(pr).unwrap_or(3.0),
		},
		Op::Channels => Recipe::Channels { mono: pr != "stereo" },
		Op::Ringtone => Recipe::Ringtone,
		Op::RemoveSilence => Recipe::RemoveSilence {
			threshold_db: d.silence_db,
			min_gap: d.silence_gap,
		},
		Op::IconSet => Recipe::IconSet {
			sizes: match pr.parse::<u32>() {
				Ok(n) => vec![n],
				Err(_) => vec![16, 24, 32, 48, 64, 128, 256, 512, 1024],
			},
		},
		Op::Favicon => Recipe::Favicon,
		Op::MacIcon => Recipe::MacIcon,
		Op::ExtractIcons => Recipe::ExtractIcons,
		Op::ReadAloud => Recipe::ReadAloud {
			voice: d.voice.clone(),
			rate: d.voice_rate,
		},
		Op::ToUtf8 => Recipe::ToUtf8,
		Op::LineEndings => Recipe::LineEndings { crlf: pr != "lf" },
		Op::CleanUp => Recipe::CleanUp,
		Op::SortLines => Recipe::SortLines {
			by: match pr {
				"za" => SortBy::Za,
				"number" => SortBy::Number,
				"length" => SortBy::Length,
				_ => SortBy::Az,
			},
		},
		Op::Dedupe => Recipe::Dedupe,
		Op::ChangeCase => Recipe::ChangeCase {
			to: match pr {
				"upper" => Case::Upper,
				"lower" => Case::Lower,
				"sentence" => Case::Sentence,
				_ => Case::Title,
			},
		},
		Op::Pretty => Recipe::Pretty,
		Op::Minify => Recipe::Minify,
		Op::Merge => Recipe::Merge,
		Op::Split if kind == Some(Kind::Archive) => Recipe::SplitArchive {
			part_mb: pr.parse().unwrap_or(100),
		},
		Op::Split => Recipe::SplitText {
			lines: pr.parse().unwrap_or(1000),
		},
		Op::Indent => Recipe::Indent {
			spaces: pr.parse().unwrap_or(0),
		},
		Op::Extract => Recipe::Extract,
		Op::Test => Recipe::Test,
		Op::Duplicate => Recipe::Duplicate,
		Op::Zip => Recipe::Pack { format: Format::Zip },
		Op::MakeGif => Recipe::MakeGif {
			span: None,
			fps: d.gif_fps,
			width: d.gif_width,
		},
		// these always need their window or are actions
		Op::Crop
		| Op::AddBackground
		| Op::Adjust
		| Op::Annotate
		| Op::Redact
		| Op::Trim
		| Op::CodeImage
		| Op::Protect
		| Op::Contents
		| Op::Copy
		| Op::CopyPath
		| Op::Rename
		| Op::Info
		| Op::Properties
		| Op::OpenWith
		| Op::MoveTo
		| Op::Share
		| Op::Float
		| Op::Hash
		| Op::Terminal
		| Op::CopyMarkdown
		| Op::Stats => return None,
	};
	Some(r)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ring_choices_become_settings() {
		let d = Defaults::default();
		assert_eq!(
			recipe_for(Op::Compress, Some("10mb"), Some(Kind::Video), &d),
			Some(Recipe::CompressVideo {
				crf: d.video_crf,
				max_height: None,
				target_mb: Some(9.5)
			})
		);
		assert_eq!(
			recipe_for(Op::Speed, Some("0.5"), Some(Kind::Audio), &d),
			Some(Recipe::Speed { factor: 0.5 })
		);
		assert_eq!(
			recipe_for(Op::Volume, Some("-6"), Some(Kind::Audio), &d),
			Some(Recipe::Volume { db: -6.0 })
		);
		assert_eq!(recipe_for(Op::Compress, Some("custom"), Some(Kind::Image), &d), None);
	}

	#[test]
	fn plain_drop_uses_the_first_choice() {
		let d = Defaults::default();
		assert_eq!(
			recipe_for(Op::Rotate, None, Some(Kind::Image), &d),
			Some(Recipe::RotateImage { turn: Turn::Cw })
		);
		assert_eq!(recipe_for(Op::Crop, None, Some(Kind::Image), &d), None);
		assert_eq!(recipe_for(Op::Metadata, None, Some(Kind::Audio), &d), None);
	}

	#[test]
	fn editor_messages_parse() {
		let json = r#"{"do":"trim","keep":[{"start":1.5,"end":4.0}],"separate":false,"exact":true}"#;
		let r: Recipe = serde_json::from_str(json).unwrap();
		assert!(matches!(r, Recipe::Trim { exact: true, .. }));
	}
}
