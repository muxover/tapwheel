#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(any(windows, test))]
mod gesture;
#[cfg(windows)]
mod win;

#[cfg(windows)]
#[global_allocator]
static ALLOC: win::Logged = win::Logged;

#[cfg(windows)]
fn main() {
	std::process::exit(win::run());
}

#[cfg(not(windows))]
fn main() {
	eprintln!("TapWheel runs on Windows 10 and 11.");
	std::process::exit(1);
}
