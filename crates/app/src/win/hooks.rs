//! The low-level keyboard and mouse hooks, on a thread that does nothing else. Windows calls a
//! hook on the thread that set it, so a busy UI thread used to delay every key and click on the
//! PC, and Windows quietly removes a hook that answers too slowly. Here a hook only feeds the
//! tracker and posts what it found to the UI thread.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::System::Threading::{
	GetCurrentThreadId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::{WM_APP_DOUBLE, WM_APP_DRAG_END, WM_APP_ESCAPE, WM_APP_NOTAP, WM_APP_PAGE, WM_APP_PRESS, WM_APP_TAP};
use crate::gesture::{Context, Event, KEY_ALT, KEY_CTRL, KEY_SHIFT, Key, Mouse, Tracker, hooks_missed};

static TARGET: AtomicIsize = AtomicIsize::new(0);
static DRAGGING: AtomicBool = AtomicBool::new(false);
/// When the hooks last saw any input, as Windows' tick count.
static SEEN: AtomicU32 = AtomicU32::new(0);
/// Set while a wheel is on screen, so taps switch wheels even without a drag.
pub static WHEEL_OPEN: AtomicBool = AtomicBool::new(false);
pub static PAUSED: AtomicBool = AtomicBool::new(false);
/// Set when the wheel has seen files on the current drag.
pub static FILES: AtomicBool = AtomicBool::new(false);
/// Set when a double tap opens the wheel on Explorer's selection.
pub static DOUBLE_TAP: AtomicBool = AtomicBool::new(false);

/// Tags the input TapWheel sends itself, so the hooks skip only that and still hear the
/// on-screen keyboard, voice control and other tools that type for people.
const OWN: usize = 0x7457_6865;
/// Asks the hook thread to find out where a drag starts, outside the hook.
const WM_HOOK_SOURCE: u32 = WM_APP + 40;
const CHECK_MS: u32 = 1000;

thread_local! {
	static TRACK: RefCell<Tracker> = RefCell::new(Tracker::default());
	static INSTALLED: RefCell<Option<(HHOOK, HHOOK)>> = const { RefCell::new(None) };
}

fn context() -> Context {
	Context {
		wheel_open: WHEEL_OPEN.load(Ordering::Relaxed),
		paused: PAUSED.load(Ordering::Relaxed),
		files_seen: FILES.load(Ordering::Relaxed),
		double_tap: DOUBLE_TAP.load(Ordering::Relaxed),
	}
}

fn post(e: Event) {
	let (msg, w) = match e {
		Event::Press(k) => (WM_APP_PRESS, k),
		Event::Tap(k) => (WM_APP_TAP, k),
		Event::DoubleTap(k) => (WM_APP_DOUBLE, k),
		Event::NoTap(k) => (WM_APP_NOTAP, k),
		Event::Escape => (WM_APP_ESCAPE, 0),
		Event::Page { back } => (WM_APP_PAGE, back as usize),
		Event::DragEnd => (WM_APP_DRAG_END, 0),
	};
	let hwnd = TARGET.load(Ordering::Relaxed) as HWND;
	if !hwnd.is_null() {
		unsafe { PostMessageW(hwnd, msg, w, 0) };
	}
}

pub fn dragging() -> bool {
	DRAGGING.load(Ordering::Relaxed)
}

fn key_of(vk: u32) -> Key {
	match vk as u16 {
		VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Key::Modifier(KEY_SHIFT),
		VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Key::Modifier(KEY_CTRL),
		VK_MENU | VK_LMENU | VK_RMENU => Key::Modifier(KEY_ALT),
		VK_ESCAPE => Key::Escape,
		VK_LWIN | VK_RWIN => Key::Windows,
		VK_TAB => Key::Tab,
		_ => Key::Other,
	}
}

unsafe extern "system" fn keyboard(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
	if code == HC_ACTION as i32 {
		let k = unsafe { &*(l as *const KBDLLHOOKSTRUCT) };
		SEEN.store(k.time, Ordering::Relaxed);
		if k.dwExtraInfo != OWN {
			let msg = w as u32;
			let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
			let (swallow, event) = TRACK.with_borrow_mut(|t| t.key(key_of(k.vkCode), down, k.time, context()));
			if let Some(e) = event {
				post(e);
			}
			if swallow {
				return 1;
			}
		}
	}
	unsafe { CallNextHookEx(std::ptr::null_mut(), code, w, l) }
}

unsafe extern "system" fn mouse(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
	if code == HC_ACTION as i32 {
		let m = unsafe { &*(l as *const MSLLHOOKSTRUCT) };
		SEEN.store(m.time, Ordering::Relaxed);
		let what = match w as u32 {
			WM_LBUTTONDOWN => Some(Mouse::LeftDown),
			WM_MOUSEMOVE => Some(Mouse::Move),
			WM_LBUTTONUP => Some(Mouse::LeftUp),
			WM_RBUTTONDOWN | WM_MBUTTONDOWN => Some(Mouse::OtherDown),
			WM_MOUSEWHEEL => Some(Mouse::Wheel((m.mouseData >> 16) as i16)),
			_ => None,
		};
		if m.dwExtraInfo != OWN
			&& let Some(what) = what
		{
			let (event, dragging) = TRACK.with_borrow_mut(|t| (t.mouse(what, m.pt.x, m.pt.y, context()), t.dragging()));
			DRAGGING.store(dragging, Ordering::Relaxed);
			if what == Mouse::LeftDown {
				FILES.store(false, Ordering::Relaxed);
				unsafe { PostThreadMessageW(GetCurrentThreadId(), WM_HOOK_SOURCE, m.pt.x as usize, m.pt.y as isize) };
			}
			if let Some(e) = event {
				post(e);
			}
		}
	}
	unsafe { CallNextHookEx(std::ptr::null_mut(), code, w, l) }
}

/// Another app came to the front (Start, Task view, Alt+Tab, a window that takes focus): an open
/// wheel would sit over it, so it fades out the way Esc closes it. TapWheel's own windows don't count.
unsafe extern "system" fn foreground(_: HWINEVENTHOOK, _: u32, _: HWND, _: i32, _: i32, _: u32, _: u32) {
	if WHEEL_OPEN.load(Ordering::Relaxed) {
		post(Event::Escape);
	}
}

/// Whether a drag starting at this point starts in a shell view: Explorer, the desktop, or a
/// file dialog, which all read Alt, Ctrl and Shift as link, copy and move.
fn shell_at(x: i32, y: i32) -> bool {
	unsafe {
		let under = WindowFromPoint(POINT { x, y });
		let mut w = under;
		let mut class = [0u16; 64];
		while !w.is_null() {
			let n = GetClassNameW(w, class.as_mut_ptr(), class.len() as i32) as usize;
			if String::from_utf16_lossy(&class[..n]) == "SHELLDLL_DefView" {
				return true;
			}
			w = GetParent(w);
		}
		let mut pid = 0;
		GetWindowThreadProcessId(GetAncestor(under, GA_ROOT), &mut pid);
		let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
		if process.is_null() {
			return false;
		}
		let mut name = [0u16; 512];
		let mut len = name.len() as u32;
		let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, name.as_mut_ptr(), &mut len) != 0;
		CloseHandle(process);
		ok && String::from_utf16_lossy(&name[..len as usize])
			.to_ascii_lowercase()
			.ends_with("\\explorer.exe")
	}
}

fn install() -> bool {
	unsafe {
		let module = GetModuleHandleW(std::ptr::null());
		let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard), module, 0);
		let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), module, 0);
		SEEN.store(GetTickCount(), Ordering::Relaxed);
		INSTALLED.with_borrow_mut(|h| *h = Some((kb, ms)));
		!kb.is_null() && !ms.is_null()
	}
}

fn uninstall() {
	if let Some((kb, ms)) = INSTALLED.with_borrow_mut(Option::take) {
		unsafe {
			if !kb.is_null() {
				UnhookWindowsHookEx(kb);
			}
			if !ms.is_null() {
				UnhookWindowsHookEx(ms);
			}
		}
	}
}

/// Puts the hooks back when input reached Windows without passing them.
fn check(reinstalled: &mut u32) {
	let mut info = LASTINPUTINFO {
		cbSize: size_of::<LASTINPUTINFO>() as u32,
		dwTime: 0,
	};
	if unsafe { GetLastInputInfo(&mut info) } == 0 || !hooks_missed(info.dwTime, SEEN.load(Ordering::Relaxed)) {
		return;
	}
	uninstall();
	let ok = install();
	*reinstalled += 1;
	// a log line the first time and then every tenth, in case something keeps tripping it
	if *reinstalled % 10 == 1 {
		super::log::write(&format!(
			"Windows removed the keyboard or mouse hook; set it again{} ({} times so far)",
			if ok { "" } else { ", which failed" },
			reinstalled
		));
	}
}

fn run(ready: std::sync::mpsc::Sender<bool>) {
	let (cx, cy) = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
	TRACK.with_borrow_mut(|t| *t = Tracker::new((cx, cy)));
	let ok = install();
	let fg = unsafe {
		SetWinEventHook(
			EVENT_SYSTEM_FOREGROUND,
			EVENT_SYSTEM_FOREGROUND,
			std::ptr::null_mut(),
			Some(foreground),
			0,
			0,
			WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
		)
	};
	let _ = ready.send(ok);
	let timer = unsafe { SetTimer(std::ptr::null_mut(), 0, CHECK_MS, None) };
	let mut reinstalled = 0;
	let mut msg: MSG = unsafe { std::mem::zeroed() };
	while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
		match msg.message {
			WM_TIMER if msg.wParam == timer => check(&mut reinstalled),
			WM_HOOK_SOURCE => {
				let shell = shell_at(msg.wParam as i32, msg.lParam as i32);
				TRACK.with_borrow_mut(|t| t.shell_drag = shell);
			}
			_ => unsafe {
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			},
		}
	}
	unsafe {
		KillTimer(std::ptr::null_mut(), timer);
		if !fg.is_null() {
			UnhookWinEvent(fg);
		}
	}
	uninstall();
}

pub struct Hooks {
	thread: u32,
	join: Option<std::thread::JoinHandle<()>>,
	ok: bool,
}

impl Hooks {
	pub fn install(target: HWND) -> Hooks {
		TARGET.store(target as isize, Ordering::Relaxed);
		let (tx, rx) = std::sync::mpsc::channel();
		let (ids, id) = std::sync::mpsc::channel();
		let spawned = std::thread::Builder::new().name("hooks".into()).spawn(move || {
			let _ = ids.send(unsafe { GetCurrentThreadId() });
			run(tx);
		});
		match spawned {
			Ok(join) => Hooks {
				thread: id.recv().unwrap_or(0),
				ok: rx.recv().unwrap_or(false),
				join: Some(join),
			},
			Err(e) => {
				super::log::write(&format!("the keyboard and mouse hooks couldn't start: {e}"));
				Hooks {
					thread: 0,
					join: None,
					ok: false,
				}
			}
		}
	}

	pub fn ok(&self) -> bool {
		self.ok
	}
}

impl Drop for Hooks {
	fn drop(&mut self) {
		if let Some(join) = self.join.take() {
			unsafe { PostThreadMessageW(self.thread, WM_QUIT, 0, 0) };
			let _ = join.join();
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

#[cfg(test)]
mod tests {
	use super::*;

	fn alt(up: bool) -> INPUT {
		INPUT {
			r#type: INPUT_KEYBOARD,
			Anonymous: INPUT_0 {
				ki: KEYBDINPUT {
					wVk: VK_MENU,
					wScan: 0,
					dwFlags: if up { KEYEVENTF_KEYUP } else { 0 },
					time: 0,
					dwExtraInfo: 0,
				},
			},
		}
	}

	#[test]
	fn taps_arrive_while_the_ui_thread_is_busy() {
		let target = unsafe {
			CreateWindowExW(
				0,
				super::super::util::wide("STATIC").as_ptr(),
				std::ptr::null(),
				0,
				0,
				0,
				0,
				0,
				HWND_MESSAGE,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
				std::ptr::null(),
			)
		};
		assert!(!target.is_null());
		let hooks = Hooks::install(target);
		assert!(hooks.ok());
		// an open wheel hears taps without a drag, which a test can't make safely
		WHEEL_OPEN.store(true, Ordering::Relaxed);
		let typist = std::thread::spawn(|| {
			std::thread::sleep(std::time::Duration::from_millis(300));
			for up in [false, true] {
				let input = alt(up);
				let sent = unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
				assert_eq!(sent, 1, "SendInput failed");
				std::thread::sleep(std::time::Duration::from_millis(40));
			}
		});
		// this thread is the UI thread here, and it doesn't look at its messages for 2 s
		std::thread::sleep(std::time::Duration::from_secs(2));
		typist.join().unwrap();
		let mut msg: MSG = unsafe { std::mem::zeroed() };
		let mut taps = Vec::new();
		while unsafe { PeekMessageW(&mut msg, target, 0, 0, PM_REMOVE) } != 0 {
			if msg.message == WM_APP_TAP {
				taps.push(msg.wParam);
			}
		}
		WHEEL_OPEN.store(false, Ordering::Relaxed);
		drop(hooks);
		unsafe { DestroyWindow(target) };
		assert_eq!(taps, vec![KEY_ALT]);
	}
}
