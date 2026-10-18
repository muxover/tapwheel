use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU32, AtomicU64, Ordering};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::{WM_APP_DRAG_END, WM_APP_ESCAPE, WM_APP_PAGE, WM_APP_TAP};

// Hook callbacks only touch these and post messages; the app state is never borrowed from here.
static TARGET: AtomicIsize = AtomicIsize::new(0);
static DOWN: AtomicBool = AtomicBool::new(false);
static DRAGGING: AtomicBool = AtomicBool::new(false);
static DOWN_X: AtomicI32 = AtomicI32::new(0);
static DOWN_Y: AtomicI32 = AtomicI32::new(0);
static HELD: AtomicU32 = AtomicU32::new(NONE);
static HELD_AT: AtomicU64 = AtomicU64::new(0);
static DIRTY: AtomicBool = AtomicBool::new(false);
/// Set while a wheel is on screen, so taps switch wheels even without a drag.
pub static WHEEL_OPEN: AtomicBool = AtomicBool::new(false);
pub static PAUSED: AtomicBool = AtomicBool::new(false);

const NONE: u32 = u32::MAX;
/// Tags the input TapWheel sends itself, so the hooks skip only that and still hear the
/// on-screen keyboard, voice control and other tools that type for people.
const OWN: usize = 0x7457_6865;
const TAP_MS: u64 = 400;

pub const KEY_SHIFT: usize = 0;
pub const KEY_CTRL: usize = 1;
pub const KEY_ALT: usize = 2;

fn modifier(vk: u32) -> Option<u32> {
	match vk as u16 {
		VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Some(KEY_SHIFT as u32),
		VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Some(KEY_CTRL as u32),
		VK_MENU | VK_LMENU | VK_RMENU => Some(KEY_ALT as u32),
		_ => None,
	}
}

fn post(msg: u32, w: usize) {
	let hwnd = TARGET.load(Ordering::Relaxed) as HWND;
	if !hwnd.is_null() {
		unsafe { PostMessageW(hwnd, msg, w, 0) };
	}
}

pub fn dragging() -> bool {
	DRAGGING.load(Ordering::Relaxed)
}

unsafe extern "system" fn keyboard(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
	if code == HC_ACTION as i32 {
		let k = unsafe { &*(l as *const KBDLLHOOKSTRUCT) };
		let msg = w as u32;
		if k.dwExtraInfo != OWN {
			let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
			let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
			match modifier(k.vkCode) {
				Some(m) if down => {
					if HELD.load(Ordering::Relaxed) != m {
						HELD.store(m, Ordering::Relaxed);
						HELD_AT.store(unsafe { GetTickCount64() }, Ordering::Relaxed);
						DIRTY.store(false, Ordering::Relaxed);
					}
				}
				Some(m) if up => {
					let quick = unsafe { GetTickCount64() }.saturating_sub(HELD_AT.load(Ordering::Relaxed)) <= TAP_MS;
					let clean = HELD.load(Ordering::Relaxed) == m && !DIRTY.load(Ordering::Relaxed);
					HELD.store(NONE, Ordering::Relaxed);
					let live = DRAGGING.load(Ordering::Relaxed) || WHEEL_OPEN.load(Ordering::Relaxed);
					if quick && clean && live && !PAUSED.load(Ordering::Relaxed) {
						post(WM_APP_TAP, m as usize);
					}
				}
				_ if down => {
					DIRTY.store(true, Ordering::Relaxed);
					if WHEEL_OPEN.load(Ordering::Relaxed) {
						if k.vkCode == VK_ESCAPE as u32 {
							post(WM_APP_ESCAPE, 0);
						} else if k.vkCode == VK_TAB as u32 {
							// Tab turns the wheel's page while a drag holds the mouse
							let back = HELD.load(Ordering::Relaxed) == KEY_SHIFT as u32;
							post(WM_APP_PAGE, back as usize);
						}
					}
				}
				_ => {}
			}
		}
	}
	unsafe { CallNextHookEx(std::ptr::null_mut(), code, w, l) }
}

unsafe extern "system" fn mouse(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
	if code == HC_ACTION as i32 {
		let m = unsafe { &*(l as *const MSLLHOOKSTRUCT) };
		if m.dwExtraInfo != OWN {
			match w as u32 {
				WM_LBUTTONDOWN => {
					DOWN.store(true, Ordering::Relaxed);
					DRAGGING.store(false, Ordering::Relaxed);
					DOWN_X.store(m.pt.x, Ordering::Relaxed);
					DOWN_Y.store(m.pt.y, Ordering::Relaxed);
				}
				WM_MOUSEMOVE => {
					if DOWN.load(Ordering::Relaxed) && !DRAGGING.load(Ordering::Relaxed) {
						let (cx, cy) = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
						let dx = (m.pt.x - DOWN_X.load(Ordering::Relaxed)).abs();
						let dy = (m.pt.y - DOWN_Y.load(Ordering::Relaxed)).abs();
						if dx > cx || dy > cy {
							DRAGGING.store(true, Ordering::Relaxed);
						}
					}
				}
				WM_LBUTTONUP => {
					let was = DRAGGING.swap(false, Ordering::Relaxed);
					DOWN.store(false, Ordering::Relaxed);
					if was {
						post(WM_APP_DRAG_END, 0);
					}
				}
				WM_RBUTTONDOWN | WM_MBUTTONDOWN => DIRTY.store(true, Ordering::Relaxed),
				WM_MOUSEWHEEL if WHEEL_OPEN.load(Ordering::Relaxed) => {
					let delta = (m.mouseData >> 16) as i16;
					post(WM_APP_PAGE, (delta > 0) as usize);
				}
				_ => {}
			}
		}
	}
	unsafe { CallNextHookEx(std::ptr::null_mut(), code, w, l) }
}

pub struct Hooks {
	kb: HHOOK,
	ms: HHOOK,
}

impl Hooks {
	pub fn install(target: HWND) -> Hooks {
		TARGET.store(target as isize, Ordering::Relaxed);
		unsafe {
			let module = GetModuleHandleW(std::ptr::null());
			Hooks {
				kb: SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), module, 0),
				ms: SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), module, 0),
			}
		}
	}

	pub fn ok(&self) -> bool {
		!self.kb.is_null() && !self.ms.is_null()
	}
}

impl Drop for Hooks {
	fn drop(&mut self) {
		unsafe {
			if !self.kb.is_null() {
				UnhookWindowsHookEx(self.kb);
			}
			if !self.ms.is_null() {
				UnhookWindowsHookEx(self.ms);
			}
		}
	}
}

/// A zero-distance mouse move, so a drag in progress notices the wheel under the pointer.
pub fn nudge() {
	let input = INPUT {
		r#type: INPUT_MOUSE,
		Anonymous: INPUT_0 {
			mi: MOUSEINPUT {
				dx: 0,
				dy: 0,
				mouseData: 0,
				dwFlags: MOUSEEVENTF_MOVE,
				time: 0,
				dwExtraInfo: OWN,
			},
		},
	};
	unsafe { SendInput(1, &input, std::mem::size_of::<INPUT>() as i32) };
}
