//! What this PC can carry, read once when TapWheel starts, and the choices that follow from it:
//! how many batches run at once, whether heavy work waits for the charger, and whether the wheel
//! keeps its full motion. Reading the PC is the app's job; the rules live here so they're tested.

use serde::Serialize;

const GB: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Power {
	pub on_battery: bool,
	/// Charge left, 0 to 100, when Windows knows it.
	pub percent: Option<u8>,
	pub saver: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Device {
	pub cores: usize,
	pub memory: u64,
	/// `x64` or `arm64`.
	pub arch: String,
	/// An x64 build running on an Arm PC through emulation.
	pub emulated: bool,
	/// The graphics card's name, when Windows reports one.
	pub gpu: Option<String>,
	/// Only Windows' software renderer was found: no graphics driver to lean on.
	pub software_gpu: bool,
	/// Whether the drive TapWheel works on (the temp folder's) is an SSD, when that can be told.
	pub ssd: Option<bool>,
	/// None on a desktop with no battery.
	pub power: Option<Power>,
	pub refresh_hz: u32,
	/// Windows' "Animation effects" setting.
	pub animations: bool,
}

impl Device {
	/// How many batches run side by side. Each one can take a core or two and a share of memory,
	/// so small PCs run one at a time and roomy ones three. On a hard drive, two batches reading
	/// whole files would fight over the disk, so it's one.
	pub fn slots(&self) -> usize {
		if self.memory <= 8 * GB || self.cores <= 2 || self.ssd == Some(false) {
			1
		} else if self.memory <= 16 * GB || self.cores <= 4 {
			2
		} else {
			3
		}
	}

	/// Heavy work (video encodes, background removal) waits while Battery saver is on, since it
	/// would drain the battery Windows is trying to keep.
	pub fn heavy_waits(&self) -> bool {
		self.power.as_ref().is_some_and(|p| p.saver)
	}

	/// One line for the log, so a report from someone's PC says what it was running on.
	pub fn log_line(&self) -> String {
		let power = match &self.power {
			None => "no battery".to_string(),
			Some(p) => format!(
				"{}{}{}",
				if p.on_battery { "on battery" } else { "plugged in" },
				p.percent.map(|v| format!(" {v}%")).unwrap_or_default(),
				if p.saver { ", Battery saver on" } else { "" }
			),
		};
		format!(
			"this PC: {} cores, {} memory, {}{}, graphics {}, temp drive {}, {power}, {} Hz, animations {}; {} at once",
			self.cores,
			crate::budget::size(self.memory),
			self.arch,
			if self.emulated { " (emulated)" } else { "" },
			self.gpu.as_deref().unwrap_or("unknown"),
			match self.ssd {
				Some(true) => "SSD",
				Some(false) => "hard drive",
				None => "unknown",
			},
			self.refresh_hz,
			if self.animations { "on" } else { "off" },
			self.slots(),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn pc(cores: usize, gb: u64) -> Device {
		Device {
			cores,
			memory: gb * GB,
			arch: "x64".into(),
			ssd: Some(true),
			refresh_hz: 60,
			animations: true,
			..Device::default()
		}
	}

	#[test]
	fn small_pcs_run_one_batch_and_roomy_ones_three() {
		assert_eq!(pc(2, 16).slots(), 1, "two cores");
		assert_eq!(pc(8, 8).slots(), 1, "8 GB");
		assert_eq!(pc(4, 16).slots(), 2);
		assert_eq!(pc(8, 12).slots(), 2);
		assert_eq!(pc(8, 32).slots(), 3, "Jax's i7-1165G7 with 32 GB");
		let hdd = Device {
			ssd: Some(false),
			..pc(8, 32)
		};
		assert_eq!(hdd.slots(), 1, "a hard drive never gets two whole-file readers");
		let unknown = Device { ssd: None, ..pc(8, 32) };
		assert_eq!(unknown.slots(), 3);
	}

	#[test]
	fn heavy_work_waits_only_for_battery_saver() {
		let mut d = pc(8, 16);
		assert!(!d.heavy_waits(), "desktop");
		d.power = Some(Power {
			on_battery: true,
			percent: Some(40),
			saver: false,
		});
		assert!(!d.heavy_waits(), "on battery without saver");
		d.power = Some(Power {
			on_battery: true,
			percent: Some(15),
			saver: true,
		});
		assert!(d.heavy_waits());
	}

	#[test]
	fn the_log_line_says_what_the_pc_is() {
		let d = Device {
			gpu: Some("Intel(R) Iris(R) Xe Graphics".into()),
			power: Some(Power {
				on_battery: true,
				percent: Some(62),
				saver: true,
			}),
			..pc(8, 32)
		};
		assert_eq!(
			d.log_line(),
			"this PC: 8 cores, 32.0 GB memory, x64, graphics Intel(R) Iris(R) Xe Graphics, temp drive SSD, \
			 on battery 62%, Battery saver on, 60 Hz, animations on; 3 at once"
		);
	}
}
