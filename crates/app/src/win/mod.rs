mod actions;
mod app;
mod drop;
mod editor;
mod float;
mod hooks;
mod jobs;
mod log;
mod share;
mod shell;
mod system;
mod util;
mod web;

use windows_sys::Win32::UI::WindowsAndMessaging::WM_APP;

pub const WM_APP_TAP: u32 = WM_APP + 1;
pub const WM_APP_DRAG_END: u32 = WM_APP + 2;
pub const WM_APP_ESCAPE: u32 = WM_APP + 3;
pub const WM_APP_JOBS: u32 = WM_APP + 4;
pub const WM_APP_TRAY: u32 = WM_APP + 5;
/// Turn the wheel's page: 0 forwards, 1 back.
pub const WM_APP_PAGE: u32 = WM_APP + 6;
/// Messages for or from a page are waiting.
pub const WM_APP_WEB: u32 = WM_APP + 7;

pub fn run() -> i32 {
	log::catch_panics();
	app::run()
}
