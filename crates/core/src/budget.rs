//! How much memory a job may plan on, worked out from what this PC has free when the job starts,
//! so a job that won't fit says so before it starts instead of running out halfway. The fixed
//! limits elsewhere stay as ceilings on top of this.

use crate::Error;

const MB: u64 = 1024 * 1024;

/// One job plans on half of what's free, leaving the rest to Windows, the other apps and any
/// job running beside it.
const SHARE: u64 = 2;

/// Physical memory free now, as the system counts it.
pub fn free_memory() -> Option<u64> {
	#[cfg(windows)]
	{
		use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
		let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
		m.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
		(unsafe { GlobalMemoryStatusEx(&mut m) } != 0).then_some(m.ullAvailPhys)
	}
	#[cfg(not(windows))]
	{
		let info = std::fs::read_to_string("/proc/meminfo").ok()?;
		let kb: u64 = info
			.lines()
			.find_map(|l| l.strip_prefix("MemAvailable:"))?
			.trim()
			.trim_end_matches("kB")
			.trim()
			.parse()
			.ok()?;
		Some(kb * 1024)
	}
}

/// A size as people read it: "640 KB", "900 MB", "1.6 GB".
pub fn size(bytes: u64) -> String {
	if bytes < MB {
		format!("{} KB", bytes.div_ceil(1024))
	} else if bytes < 1024 * MB {
		format!("{} MB", bytes.div_ceil(MB))
	} else {
		format!("{:.1} GB", bytes as f64 / (1024 * MB) as f64)
	}
}

/// Refuses a job that needs `need` bytes of memory when `free` (what the system reports, if it
/// does) can't spare it.
pub fn fits(need: u64, free: Option<u64>) -> Result<(), Error> {
	match free {
		Some(free) if need > free / SHARE => Err(Error::Engine(format!(
			"this needs about {} of memory and this PC has {} free; close some apps and try again",
			size(need),
			size(free)
		))),
		_ => Ok(()),
	}
}

/// [`fits`] against what's free right now.
pub fn fits_now(need: u64) -> Result<(), Error> {
	fits(need, free_memory())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_job_plans_on_half_of_whats_free() {
		assert!(fits(400 * MB, Some(1024 * MB)).is_ok());
		let e = fits(700 * MB, Some(1024 * MB)).unwrap_err().to_string();
		assert!(e.contains("about 700 MB of memory and this PC has 1.0 GB free"), "{e}");
		// when the system won't say, only the fixed ceilings apply
		assert!(fits(u64::MAX, None).is_ok());
	}

	#[test]
	fn this_pc_reports_its_free_memory() {
		let free = free_memory().unwrap();
		assert!(free > 16 * MB, "{free}");
	}

	#[test]
	fn sizes_read_plainly() {
		assert_eq!(size(10 * 1024), "10 KB");
		assert_eq!(size(900 * MB), "900 MB");
		assert_eq!(size(1700 * MB), "1.7 GB");
	}
}
