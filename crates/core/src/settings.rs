use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ops::{Op, Wheel};

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
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
}

impl Default for Settings {
	fn default() -> Self {
		Settings {
			keys: Keys::default(),
			no_drag: NoDrag::default(),
			start_with_windows: true,
			learn_order: true,
			sounds: false,
			welcomed: false,
			usage: BTreeMap::new(),
			recent_folders: Vec::new(),
			defaults: Defaults::default(),
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
	fn recent_folders_stay_short_and_unique() {
		let mut s = Settings::default();
		for d in ["a", "b", "c", "d", "e", "b"] {
			s.remember_folder(Path::new(d));
		}
		let names: Vec<_> = s.recent_folders.iter().map(|p| p.display().to_string()).collect();
		assert_eq!(names, ["b", "e", "d", "c"]);
	}
}
