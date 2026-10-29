use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::kind::Kind;
use crate::ops::{Op, Wheel};

/// The most ring choices of your own one slice keeps for one kind of file, so the ring stays
/// readable.
pub const MAX_OWN_CHOICES: usize = 3;
/// The longest name a ring choice of your own can have, so it fits its chip.
pub const MAX_CHOICE_LABEL: usize = 16;

/// A tool's settings kept as a choice on its ring, like "Compress to 2 MB".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OwnChoice {
	/// The slice's id, like `compress`.
	pub op: String,
	/// The kind of file it was made for, like `video`.
	pub kind: Option<String>,
	pub label: String,
	pub recipe: crate::Recipe,
}

/// Reads the saved ring choices one by one, so a choice an older or newer TapWheel wrote
/// differently is dropped instead of taking the rest of the settings with it.
fn lenient_choices<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<OwnChoice>, D::Error> {
	let raw: Vec<serde_json::Value> = Vec::deserialize(d)?;
	Ok(raw.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
}

/// The settings file's format. Raised when a change needs an old file read differently.
pub const SETTINGS_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Key {
	Shift,
	Ctrl,
	Alt,
	Off,
}

impl Key {
	pub fn label(self) -> &'static str {
		match self {
			Key::Shift => "Shift",
			Key::Ctrl => "Ctrl",
			Key::Alt => "Alt",
			Key::Off => "Off",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Keys {
	pub convert: Key,
	pub tools: Key,
	pub actions: Key,
}

impl Default for Keys {
	fn default() -> Self {
		Keys {
			convert: Key::Shift,
			tools: Key::Ctrl,
			actions: Key::Alt,
		}
	}
}

impl Keys {
	pub fn wheel_for(&self, key: Key) -> Option<Wheel> {
		if key == Key::Off {
			return None;
		}
		Wheel::ALL.into_iter().find(|w| self.key_for(*w) == key)
	}

	pub fn key_for(&self, wheel: Wheel) -> Key {
		match wheel {
			Wheel::Convert => self.convert,
			Wheel::Tools => self.tools,
			Wheel::Actions => self.actions,
		}
	}

	/// Gives `wheel` the key; whichever wheel had it before takes the old key, so no key does two things.
	pub fn assign(&mut self, wheel: Wheel, key: Key) {
		let old = self.key_for(wheel);
		if key != Key::Off
			&& let Some(other) = self.wheel_for(key)
		{
			*self.slot(other) = old;
		}
		*self.slot(wheel) = key;
	}

	fn slot(&mut self, wheel: Wheel) -> &mut Key {
		match wheel {
			Wheel::Convert => &mut self.convert,
			Wheel::Tools => &mut self.tools,
			Wheel::Actions => &mut self.actions,
		}
	}
}

/// Tool settings: what a plain drop on each slice does. Editors start from these too.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Defaults {
	/// JPG, WebP and AVIF quality for conversions, 1 to 100.
	pub image_quality: u8,
	pub image_quality_balanced: u8,
	pub image_quality_small: u8,
	pub image_quality_best: u8,
	/// Remove location and camera details whenever a picture is converted.
	pub strip_on_convert: bool,
	pub video_crf: u8,
	pub video_crf_balanced: u8,
	pub video_crf_small: u8,
	/// Use the graphics card's encoder when there is one.
	pub hardware_video: bool,
	pub audio_kbps: u32,
	pub gif_fps: u32,
	pub gif_width: u32,
	pub fade_seconds: f32,
	pub silence_db: f32,
	pub silence_gap: f32,
	pub trim_exact: bool,
	pub combine_gap: u32,
	pub combine_background: String,
	pub voice: Option<String>,
	pub voice_rate: f32,
	pub code_theme: String,
	pub code_background: String,
	pub background_style: String,
}

impl Default for Defaults {
	fn default() -> Self {
		Defaults {
			image_quality: 90,
			image_quality_balanced: 78,
			image_quality_small: 62,
			image_quality_best: 88,
			strip_on_convert: false,
			video_crf: 23,
			video_crf_balanced: 27,
			video_crf_small: 31,
			hardware_video: false,
			audio_kbps: 192,
			gif_fps: 15,
			gif_width: 640,
			fade_seconds: 2.0,
			silence_db: -42.0,
			silence_gap: 0.6,
			trim_exact: true,
			combine_gap: 16,
			combine_background: "#ffffff".into(),
			voice: None,
			voice_rate: 1.0,
			code_theme: "dark".into(),
			code_background: "sunset".into(),
			background_style: "sunset".into(),
		}
	}
}

/// How the wheel opens on files selected in Explorer or on the desktop, without a drag.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoDrag {
	/// Two quick taps of a wheel's key open that wheel.
	#[default]
	DoubleTap,
	/// Ctrl+Shift+Space opens the Convert wheel. Windows gives the shortcut to TapWheel alone.
	Shortcut,
	Off,
}

/// How one wheel is arranged for one kind of file. Slices are kept by id, so a slice the app adds
/// later still shows up, after the ones placed by hand.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
	/// The order set by hand. Empty means the app's own order, with favourites first if that's on.
	pub order: Vec<String>,
	/// Always first, in this order, and never moved by favourites.
	pub pinned: Vec<String>,
	pub hidden: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WheelSize {
	Small,
	#[default]
	Normal,
	Large,
}

impl WheelSize {
	pub fn factor(self) -> f32 {
		match self {
			WheelSize::Small => 0.85,
			WheelSize::Normal => 1.0,
			WheelSize::Large => 1.15,
		}
	}
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
	pub version: u32,
	pub keys: Keys,
	pub no_drag: NoDrag,
	pub start_with_windows: bool,
	/// Slices you pick most move to the top of the wheel.
	pub learn_order: bool,
	/// Short sounds when a wheel opens and a job finishes.
	pub sounds: bool,
	pub welcomed: bool,
	pub usage: BTreeMap<String, u32>,
	/// Folders used with Move to, newest first.
	pub recent_folders: Vec<PathBuf>,
	pub defaults: Defaults,
	/// Wheels arranged by hand, by `layout_key`.
	pub wheels: BTreeMap<String, Layout>,
	pub wheel_size: WheelSize,
	/// The wheel's colour as `#rrggbb`; none follows the Windows accent.
	pub accent: Option<String>,
	/// Tool settings kept as ring choices, oldest first.
	#[serde(deserialize_with = "lenient_choices")]
	pub own_choices: Vec<OwnChoice>,
}

/// Where a wheel's arrangement for a kind of file is kept: `convert/image`. Mixed selections
/// share `mixed`.
pub fn layout_key(wheel: Wheel, kind: Option<Kind>) -> String {
	format!("{}/{}", wheel.id(), kind.map_or("mixed", Kind::id))
}

impl Default for Settings {
	fn default() -> Self {
		Settings {
			version: SETTINGS_VERSION,
			keys: Keys::default(),
			no_drag: NoDrag::default(),
			start_with_windows: true,
			learn_order: true,
			sounds: false,
			welcomed: false,
			usage: BTreeMap::new(),
			recent_folders: Vec::new(),
			defaults: Defaults::default(),
			wheels: BTreeMap::new(),
			wheel_size: WheelSize::default(),
			accent: None,
			own_choices: Vec::new(),
		}
	}
}

impl Settings {
	pub fn load(path: &Path) -> Settings {
		std::fs::read_to_string(path)
			.ok()
			.and_then(|s| toml::from_str(&s).ok())
			.unwrap_or_default()
	}

	pub fn save(&self, path: &Path) -> std::io::Result<()> {
		if let Some(dir) = path.parent() {
			std::fs::create_dir_all(dir)?;
		}
		let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
		let part = path.with_extension("toml.tw-part");
		std::fs::write(&part, text)?;
		std::fs::rename(&part, path)
	}

	pub fn record(&mut self, op: Op) {
		*self.usage.entry(op.id()).or_default() += 1;
	}

	/// Most-used first; ties keep the wheel's own order.
	pub fn order(&self, ops: &mut [Op]) {
		if self.learn_order {
			ops.sort_by_key(|op| std::cmp::Reverse(self.usage.get(&op.id()).copied().unwrap_or(0)));
		}
	}

	/// Keeps a tool's settings as a ring choice. Refused when the recipe belongs to one file, the
	/// name is empty, or the slice already has the most choices it keeps; a choice with the same
	/// name is replaced.
	pub fn keep_choice(
		&mut self,
		op: Op,
		kind: Option<Kind>,
		label: &str,
		recipe: crate::Recipe,
	) -> Result<(), &'static str> {
		let label: String = label.trim().chars().take(MAX_CHOICE_LABEL).collect();
		if label.is_empty() {
			return Err("Give the choice a name");
		}
		if !recipe.reusable() {
			return Err("This tool's settings belong to this file, so they can't be kept");
		}
		let (op, kind) = (op.id(), kind.map(|k| k.id().to_string()));
		self.own_choices
			.retain(|c| !(c.op == op && c.kind == kind && c.label.eq_ignore_ascii_case(&label)));
		if self.own_choices.iter().filter(|c| c.op == op && c.kind == kind).count() >= MAX_OWN_CHOICES {
			return Err("This slice already keeps three choices; remove one in Settings first");
		}
		self.own_choices.push(OwnChoice {
			op,
			kind,
			label,
			recipe,
		});
		Ok(())
	}

	/// The ring choices of your own for a slice and a kind of file, as (index into `own_choices`, choice).
	pub fn choices_for(&self, op: Op, kind: Option<Kind>) -> Vec<(usize, &OwnChoice)> {
		let (op, kind) = (op.id(), kind.map(|k| k.id().to_string()));
		self.own_choices
			.iter()
			.enumerate()
			.filter(|(_, c)| c.op == op && c.kind == kind)
			.collect()
	}

	/// The wheel colour picked in Settings, when it's a valid `#rrggbb`.
	pub fn accent_rgb(&self) -> Option<(u8, u8, u8)> {
		let hex = self.accent.as_deref()?.strip_prefix('#')?;
		if hex.len() != 6 {
			return None;
		}
		let v = u32::from_str_radix(hex, 16).ok()?;
		Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
	}

	/// Puts a wheel's slices in the order the user set: hidden ones left out, pinned ones first in
	/// their order, then the hand-set order or, without one, favourites first. Slices the layout
	/// doesn't name keep the app's own order after the ones it does.
	pub fn arrange(&self, wheel: Wheel, kind: Option<Kind>, ops: &mut Vec<Op>) {
		let Some(layout) = self.wheels.get(&layout_key(wheel, kind)) else {
			self.order(ops);
			return;
		};
		ops.retain(|op| !layout.hidden.contains(&op.id()));
		if layout.order.is_empty() {
			self.order(ops);
		} else {
			let place = |op: &Op| layout.order.iter().position(|id| *id == op.id()).unwrap_or(usize::MAX);
			ops.sort_by_key(place);
		}
		let pin = |op: &Op| layout.pinned.iter().position(|id| *id == op.id()).unwrap_or(usize::MAX);
		ops.sort_by_key(pin);
	}

	/// Remembers a Move to folder, keeping the four most recent.
	pub fn remember_folder(&mut self, dir: &Path) {
		self.recent_folders.retain(|d| d != dir);
		self.recent_folders.insert(0, dir.to_path_buf());
		self.recent_folders.truncate(4);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::format::Format;

	#[test]
	fn assigning_a_taken_key_swaps() {
		let mut k = Keys::default();
		k.assign(Wheel::Tools, Key::Shift);
		assert_eq!(k.tools, Key::Shift);
		assert_eq!(k.convert, Key::Ctrl);
		assert_eq!(k.actions, Key::Alt);
		k.assign(Wheel::Actions, Key::Off);
		assert_eq!(k.wheel_for(Key::Alt), None);
		assert_eq!(k.wheel_for(Key::Off), None);
	}

	#[test]
	fn usage_reorders_and_round_trips() {
		let mut s = Settings::default();
		let mut ops = vec![
			Op::Convert(Format::Png),
			Op::Convert(Format::Jpg),
			Op::Convert(Format::Webp),
		];
		s.record(Op::Convert(Format::Webp));
		s.record(Op::Convert(Format::Webp));
		s.record(Op::Convert(Format::Jpg));
		s.order(&mut ops);
		assert_eq!(
			ops,
			vec![
				Op::Convert(Format::Webp),
				Op::Convert(Format::Jpg),
				Op::Convert(Format::Png)
			]
		);

		let dir = std::env::temp_dir().join(format!("tw-set-{}", std::process::id()));
		let path = dir.join("settings.toml");
		s.save(&path).unwrap();
		assert_eq!(Settings::load(&path), s);
		std::fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn broken_file_falls_back_to_defaults() {
		let dir = std::env::temp_dir().join(format!("tw-set-bad-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("settings.toml");
		std::fs::write(&path, "keys = 12\n[[[").unwrap();
		assert_eq!(Settings::load(&path), Settings::default());
		std::fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn a_settings_file_from_before_no_drag_keeps_its_keys() {
		let dir = std::env::temp_dir().join(format!("tw-set-old-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("settings.toml");
		std::fs::write(
			&path,
			"start_with_windows = false\n\n[keys]\nconvert = \"alt\"\ntools = \"ctrl\"\nactions = \"shift\"\n",
		)
		.unwrap();
		let s = Settings::load(&path);
		assert_eq!(s.keys.convert, Key::Alt);
		assert!(!s.start_with_windows);
		assert_eq!(s.no_drag, NoDrag::DoubleTap);
		let mut s = s;
		s.no_drag = NoDrag::Shortcut;
		s.save(&path).unwrap();
		assert!(
			std::fs::read_to_string(&path)
				.unwrap()
				.contains("no_drag = \"shortcut\"")
		);
		assert_eq!(Settings::load(&path).no_drag, NoDrag::Shortcut);
		std::fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn the_wheel_colour_is_read_only_when_valid() {
		let mut s = Settings::default();
		assert_eq!(s.accent_rgb(), None);
		s.accent = Some("#60cdff".into());
		assert_eq!(s.accent_rgb(), Some((0x60, 0xcd, 0xff)));
		for bad in ["60cdff", "#60cdf", "#zzzzzz", ""] {
			s.accent = Some(bad.into());
			assert_eq!(s.accent_rgb(), None, "{bad}");
		}
	}

	fn small_video() -> crate::Recipe {
		crate::recipe_for(Op::Compress, Some("small"), Some(Kind::Video), &Defaults::default()).unwrap()
	}

	#[test]
	fn a_kept_choice_rides_its_slice_and_survives_a_restart() {
		let mut s = Settings::default();
		s.keep_choice(Op::Compress, Some(Kind::Video), "  To send  ", small_video())
			.unwrap();
		let mine = s.choices_for(Op::Compress, Some(Kind::Video));
		assert_eq!(mine.len(), 1);
		assert_eq!(mine[0].1.label, "To send");
		// pictures have their own compress ring
		assert!(s.choices_for(Op::Compress, Some(Kind::Image)).is_empty());
		let dir = std::env::temp_dir().join(format!("tw-set-own-{}", std::process::id()));
		let path = dir.join("settings.toml");
		s.save(&path).unwrap();
		assert_eq!(Settings::load(&path), s);
		// and through the Settings page, which sends JSON
		let json = serde_json::to_value(&s).unwrap();
		assert_eq!(serde_json::from_value::<Settings>(json).unwrap(), s);
		std::fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn choices_are_capped_named_and_never_tied_to_one_file() {
		let mut s = Settings::default();
		assert!(
			s.keep_choice(Op::Compress, Some(Kind::Video), "   ", small_video())
				.is_err()
		);
		let trim = crate::Recipe::Trim {
			keep: Vec::new(),
			separate: false,
			exact: true,
		};
		assert!(s.keep_choice(Op::Trim, Some(Kind::Video), "Intro", trim).is_err());
		for name in ["One", "Two", "Three"] {
			s.keep_choice(Op::Compress, Some(Kind::Video), name, small_video())
				.unwrap();
		}
		assert!(
			s.keep_choice(Op::Compress, Some(Kind::Video), "Four", small_video())
				.is_err()
		);
		// the same name replaces instead of counting again
		s.keep_choice(Op::Compress, Some(Kind::Video), "two", small_video())
			.unwrap();
		assert_eq!(s.choices_for(Op::Compress, Some(Kind::Video)).len(), 3);
		s.keep_choice(
			Op::Compress,
			Some(Kind::Video),
			"A very long choice name",
			small_video(),
		)
		.unwrap_err();
		let mut t = Settings::default();
		t.keep_choice(
			Op::Compress,
			Some(Kind::Video),
			"A very long choice name",
			small_video(),
		)
		.unwrap();
		assert_eq!(t.own_choices[0].label.chars().count(), MAX_CHOICE_LABEL);
	}

	#[test]
	fn a_choice_that_no_longer_reads_is_dropped_alone() {
		let text = r#"learn_order = false

[[own_choices]]
op = "compress"
kind = "video"
label = "Gone"
recipe = { do = "a_tool_that_left" }

[[own_choices]]
op = "compress"
kind = "video"
label = "Small"

[own_choices.recipe]
do = "compress_video"
crf = 30
max_height = 720
target_mb = 8
"#;
		let s: Settings = toml::from_str(text).unwrap();
		assert!(!s.learn_order, "the rest of the file still loads");
		assert!(s.own_choices.iter().all(|c| c.label != "Gone"));
	}

	fn ids(ops: &[Op]) -> Vec<String> {
		ops.iter().map(|o| o.id()).collect()
	}

	#[test]
	fn a_wheel_without_a_layout_keeps_favourites_first() {
		let mut s = Settings::default();
		s.record(Op::Convert(Format::Webp));
		let mut ops = vec![Op::Convert(Format::Png), Op::Convert(Format::Webp)];
		s.arrange(Wheel::Convert, Some(Kind::Image), &mut ops);
		assert_eq!(ids(&ops), ["to-webp", "to-png"]);
	}

	#[test]
	fn hidden_slices_go_and_pinned_ones_lead_in_their_order() {
		let mut s = Settings::default();
		s.wheels.insert(
			layout_key(Wheel::Convert, Some(Kind::Image)),
			Layout {
				order: Vec::new(),
				pinned: vec!["to-webp".into(), "to-jpg".into()],
				hidden: vec!["to-png".into()],
			},
		);
		// favourites would put PNG and GIF first; PNG is hidden and pins outrank favourites
		for _ in 0..5 {
			s.record(Op::Convert(Format::Png));
			s.record(Op::Convert(Format::Gif));
		}
		let mut ops = vec![
			Op::Convert(Format::Png),
			Op::Convert(Format::Jpg),
			Op::Convert(Format::Gif),
			Op::Convert(Format::Webp),
		];
		s.arrange(Wheel::Convert, Some(Kind::Image), &mut ops);
		assert_eq!(ids(&ops), ["to-webp", "to-jpg", "to-gif"]);
		// another kind of file keeps its own arrangement
		let mut video = vec![Op::Convert(Format::Png), Op::Convert(Format::Gif)];
		s.arrange(Wheel::Convert, Some(Kind::Video), &mut video);
		assert_eq!(ids(&video), ["to-png", "to-gif"]);
	}

	#[test]
	fn a_hand_set_order_wins_and_new_slices_follow_it() {
		let mut s = Settings::default();
		s.record(Op::Crop);
		s.wheels.insert(
			layout_key(Wheel::Tools, Some(Kind::Image)),
			Layout {
				order: vec!["resize".into(), "compress".into()],
				..Layout::default()
			},
		);
		let mut ops = vec![Op::Crop, Op::Compress, Op::Adjust, Op::Resize];
		s.arrange(Wheel::Tools, Some(Kind::Image), &mut ops);
		assert_eq!(ids(&ops), ["resize", "compress", "crop", "adjust"]);
	}

	#[test]
	fn pinned_slices_fill_the_first_page() {
		let catalog = crate::ops::catalog();
		let (_, _, all) = catalog
			.iter()
			.find(|(w, k, _)| *w == Wheel::Convert && *k == Kind::Image)
			.unwrap();
		assert!(all.len() > 8, "images convert to more than one page");
		let last_three: Vec<String> = all[all.len() - 3..].iter().map(|o| o.id()).collect();
		let mut s = Settings::default();
		s.wheels.insert(
			layout_key(Wheel::Convert, Some(Kind::Image)),
			Layout {
				pinned: last_three.clone(),
				..Layout::default()
			},
		);
		let mut ops = all.clone();
		s.arrange(Wheel::Convert, Some(Kind::Image), &mut ops);
		// the first page holds seven slices and More
		assert_eq!(ids(&ops[..3]), last_three);
		assert_eq!(ops.len(), all.len());
	}

	#[test]
	fn the_catalog_lists_every_wheel_for_every_kind() {
		let catalog = crate::ops::catalog();
		assert_eq!(catalog.len(), 21);
		assert!(catalog.iter().all(|(_, _, ops)| !ops.is_empty()));
		let images_tools = &catalog
			.iter()
			.find(|(w, k, _)| *w == Wheel::Tools && *k == Kind::Image)
			.unwrap()
			.2;
		assert!(images_tools.contains(&Op::Crop) && images_tools.contains(&Op::RemoveBackground));
	}

	#[test]
	fn a_0_1_1_settings_file_loads_with_its_choices() {
		// the shape v0.1.1 wrote
		let old = r##"start_with_windows = true
learn_order = false
sounds = false
welcomed = true
recent_folders = ["C:\\Users\\you\\Pictures"]

[keys]
convert = "ctrl"
tools = "shift"
actions = "alt"

[usage]
crop = 4
to-webp = 9

[defaults]
image_quality = 85
image_quality_balanced = 78
image_quality_small = 62
image_quality_best = 88
strip_on_convert = true
video_crf = 23
video_crf_balanced = 27
video_crf_small = 31
hardware_video = false
audio_kbps = 256
gif_fps = 15
gif_width = 640
fade_seconds = 2.0
silence_db = -42.0
silence_gap = 0.6
trim_exact = true
combine_gap = 16
combine_background = "#ffffff"
voice_rate = 1.0
code_theme = "dark"
code_background = "sunset"
background_style = "sunset"
"##;
		let dir = std::env::temp_dir().join(format!("tw-set-011-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let path = dir.join("settings.toml");
		std::fs::write(&path, old).unwrap();
		let s = Settings::load(&path);
		assert_eq!(s.keys.convert, Key::Ctrl);
		assert!(!s.learn_order);
		assert_eq!(s.usage.get("to-webp"), Some(&9));
		assert_eq!(s.defaults.image_quality, 85);
		assert_eq!(s.defaults.audio_kbps, 256);
		assert!(s.wheels.is_empty());
		assert_eq!(s.wheel_size, WheelSize::Normal);
		assert_eq!(s.accent, None);
		assert_eq!(s.version, SETTINGS_VERSION);
		std::fs::remove_dir_all(dir).unwrap();
	}

	#[test]
	fn recent_folders_stay_short_and_unique() {
		let mut s = Settings::default();
		for d in ["a", "b", "c", "d", "e", "b"] {
			s.remember_folder(Path::new(d));
		}
		let names: Vec<_> = s.recent_folders.iter().map(|p| p.display().to_string()).collect();
		assert_eq!(names, ["b", "e", "d", "c"]);
	}
}
