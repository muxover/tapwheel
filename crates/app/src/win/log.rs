//! A short local log of what went wrong, so a problem on someone's PC can be found without
//! guessing. It stays on the PC, holds errors only (never keys or clicks), and keeps itself
//! under 256 KB by starting over with the last half.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const LIMIT: u64 = 256 * 1024;

static LOCK: Mutex<()> = Mutex::new(());

pub fn path() -> PathBuf {
	super::shell::appdata("LOCALAPPDATA").join("tapwheel.log")
}

/// Adds one line with the time it happened.
pub fn write(what: &str) {
	let _guard = LOCK.lock();
	let file = path();
	if let Some(dir) = file.parent() {
		let _ = std::fs::create_dir_all(dir);
	}
	if std::fs::metadata(&file).is_ok_and(|m| m.len() > LIMIT)
		&& let Ok(old) = std::fs::read(&file)
	{
		let keep = &old[old.len() / 2..];
		let start = keep.iter().position(|&b| b == b'\n').map_or(0, |i| i + 1);
		let _ = std::fs::write(&file, &keep[start..]);
	}
	let secs = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_secs())
		.unwrap_or(0);
	if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
		let line = what.replace(['\r', '\n'], " ");
		let _ = writeln!(f, "{secs} v{} {line}", env!("CARGO_PKG_VERSION"));
	}
}

/// Writes panics to the log before the default report.
pub fn catch_panics() {
	let default = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		write(&format!("panic: {info}"));
		default(info);
	}));
}
