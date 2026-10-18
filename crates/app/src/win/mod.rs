mod app;
mod drop;
mod hooks;
mod jobs;
mod shell;
mod system;
mod util;

use windows_sys::Win32::UI::WindowsAndMessaging::WM_APP;

pub const WM_APP_TAP: u32 = WM_APP + 1;
pub const WM_APP_DRAG_END: u32 = WM_APP + 2;
pub const WM_APP_ESCAPE: u32 = WM_APP + 3;
pub const WM_APP_JOBS: u32 = WM_APP + 4;
pub const WM_APP_TRAY: u32 = WM_APP + 5;

pub fn run() -> i32 {
	app::run()
}
