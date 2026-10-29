mod access;
mod actions;
mod app;
mod crash;
mod device;
mod drop;
mod editor;
mod float;
mod hooks;
mod jobs;
mod log;
mod selection;
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
/// A trigger key went down during a drag; `WM_APP_TAP` or `WM_APP_NOTAP` follows.
pub const WM_APP_PRESS: u32 = WM_APP + 8;
pub const WM_APP_NOTAP: u32 = WM_APP + 9;
/// Two quick taps of a key with no drag: open its wheel on Explorer's selection.
pub const WM_APP_DOUBLE: u32 = WM_APP + 10;
/// A message from another TapWheel process that arrived while the app was busy, tried again.
pub const WM_APP_MAIL: u32 = WM_APP + 11;

pub use crash::Logged;

pub fn run() -> i32 {
	log::catch_panics();
	crash::install();
	app::run()
}
