use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ops::{Format, Op, Wheel};

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
	pub keys: Keys,
	pub start_with_windows: bool,
	/// Slices you pick most move to the top of the wheel.
	pub learn_order: bool,
	pub welcomed: bool,
	pub usage: BTreeMap<String, u32>,
}

impl Default for Settings {
	fn default() -> Self {
		Settings {
			keys: Keys::default(),
			start_with_windows: true,
			learn_order: true,
			welcomed: false,
			usage: BTreeMap::new(),
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
		*self.usage.entry(op_id(op)).or_default() += 1;
	}

	/// Most-used first; ties keep the wheel's own order.
	pub fn order(&self, ops: &mut [Op]) {
		if self.learn_order {
			ops.sort_by_key(|op| std::cmp::Reverse(self.usage.get(&op_id(*op)).copied().unwrap_or(0)));
		}
	}
}

pub fn op_id(op: Op) -> String {
	match op {
		Op::Convert(f) => format!("to-{}", f.ext()),
		other => format!("{other:?}").to_lowercase(),
	}
}

pub fn op_from_id(id: &str) -> Option<Op> {
	const FORMATS: &[Format] = &[
		Format::Jpg,
		Format::Png,
		Format::Webp,
		Format::Avif,
		Format::Tiff,
		Format::Bmp,
		Format::Gif,
		Format::Ico,
		Format::Mp3,
		Format::M4a,
		Format::Wav,
		Format::Flac,
		Format::Ogg,
		Format::Opus,
		Format::Mp4,
		Format::Mov,
		Format::Mkv,
		Format::Webm,
		Format::Avi,
		Format::Srt,
		Format::Vtt,
		Format::Txt,
		Format::Zip,
		Format::SevenZ,
		Format::Tar,
		Format::Tgz,
	];
	const OPS: &[Op] = &[
		Op::CompressImage,
		Op::HalfSize,
		Op::FitHd,
		Op::Rotate,
		Op::StripMetadata,
		Op::CompressVideo,
		Op::Video720,
		Op::Mute,
		Op::ExtractAudio,
		Op::CompressAudio,
		Op::Normalize,
		Op::Extract,
		Op::Zip,
		Op::CopyPath,
		Op::CopyFile,
		Op::Terminal,
		Op::Reveal,
	];
	if let Some(ext) = id.strip_prefix("to-") {
		return FORMATS.iter().find(|f| f.ext() == ext).map(|f| Op::Convert(*f));
	}
	OPS.iter().find(|o| op_id(**o) == id).copied()
}

#[cfg(test)]
mod tests {
	use super::*;

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
	fn ids_round_trip() {
		for op in [
			Op::Convert(Format::Tgz),
			Op::Convert(Format::SevenZ),
			Op::StripMetadata,
			Op::Reveal,
		] {
			assert_eq!(op_from_id(&op_id(op)), Some(op));
		}
	}
}
