//! The files selected in the Explorer window, or on the desktop, that has the keyboard, so the
//! wheel can open on them without a drag, and where the focused one is on screen.

use std::cell::RefCell;
use std::path::PathBuf;

use windows::Win32::System::Com::{
	CLSCTX_INPROC_SERVER, CLSCTX_LOCAL_SERVER, CoCreateInstance, CoTaskMemFree, IDispatch, IServiceProvider,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
use windows::Win32::UI::Shell::{
	IFolderView, IShellBrowser, IShellItemArray, IShellWindows, IWebBrowserApp, SID_STopLevelBrowser,
	SIGDN_FILESYSPATH, SVGIO_SELECTION, SWC_DESKTOP, SWFO_NEEDDISPATCH, ShellWindows,
};
use windows::core::Interface;
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
	GA_ROOT, GUITHREADINFO, GetAncestor, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetParent,
	GetWindowThreadProcessId,
};

thread_local! {
	// Made once on the UI thread: creating them is the slow part, asking them is quick.
	static SHELL: RefCell<Option<IShellWindows>> = const { RefCell::new(None) };
	static UIA: RefCell<Option<IUIAutomation>> = const { RefCell::new(None) };
}

/// Connects to Explorer and UI Automation ahead of time, so the first double tap is as quick as
/// the rest. Called on the UI thread when a way in without a drag is turned on.
pub fn warm() {
	SHELL.with_borrow_mut(|slot| {
		if slot.is_none() {
			*slot = unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER) }
				.inspect_err(|e| super::log::write(&format!("connecting to Explorer failed: {e}")))
				.ok();
		}
	});
	UIA.with_borrow_mut(|slot| {
		if slot.is_none() {
			*slot = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
				.inspect_err(|e| super::log::write(&format!("starting UI Automation failed: {e}")))
				.ok();
		}
	});
}

/// The selected files, or `None` when the keyboard isn't on the items of Explorer or the desktop
/// (an address bar, a rename box, a search box, a file dialog or another app), or nothing on
/// disk is selected. Items with no file behind them, such as This PC, are left out.
pub fn selected() -> Option<Vec<PathBuf>> {
	let Some((root, view)) = focused_view() else {
		// a line in the log says why nothing opened, for a report from a PC we can't see
		super::log::write(&format!("no wheel without a drag: the keyboard is on {}", focus_name()));
		return None;
	};
	let files = files_in(root, view)
		.or_else(|_| {
			// Explorer restarted since the list was made: make it again, once
			SHELL.with_borrow_mut(|s| *s = None);
			files_in(root, view)
		})
		.inspect_err(|e| super::log::write(&format!("reading Explorer's selection failed: {e}")))
		.ok()?;
	if files.is_empty() {
		super::log::write("no wheel without a drag: no file on disk is selected");
	}
	(!files.is_empty()).then_some(files)
}

/// The class of the window with the keyboard and of its top window, for the log.
fn focus_name() -> String {
	unsafe {
		let fg = GetForegroundWindow();
		if fg.is_null() {
			return "no window".into();
		}
		let thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
		let mut info = GUITHREADINFO {
			cbSize: size_of::<GUITHREADINFO>() as u32,
			..std::mem::zeroed()
		};
		let focus = if GetGUIThreadInfo(thread, &mut info) != 0 && !info.hwndFocus.is_null() {
			class(info.hwndFocus)
		} else {
			"nothing".into()
		};
		format!("{focus} in {}", class(GetAncestor(fg, GA_ROOT)))
	}
}

/// Where the focused file sits on screen, when Explorer or the desktop has the keyboard, so the
/// wheel opens beside the file rather than under the pointer.
pub fn focused_item_rect() -> Option<RECT> {
	let fg = unsafe { GetForegroundWindow() };
	if fg.is_null() || !is_shell_root(unsafe { GetAncestor(fg, GA_ROOT) }) {
		return None;
	}
	UIA.with_borrow_mut(|slot| {
		if slot.is_none() {
			*slot = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.ok();
		}
		let r = unsafe {
			slot.as_ref()?
				.GetFocusedElement()
				.ok()?
				.CurrentBoundingRectangle()
				.ok()?
		};
		(r.right > r.left && r.bottom > r.top).then_some(RECT {
			left: r.left,
			top: r.top,
			right: r.right,
			bottom: r.bottom,
		})
	})
}

fn class(hwnd: HWND) -> String {
	let mut buf = [0u16; 64];
	let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
	String::from_utf16_lossy(&buf[..n])
}

fn is_shell_root(root: HWND) -> bool {
	matches!(class(root).as_str(), "CabinetWClass" | "Progman" | "WorkerW")
}

/// Where the keyboard is: Explorer's or the desktop's item view, with its top window.
fn focused_view() -> Option<(HWND, HWND)> {
	unsafe {
		let fg = GetForegroundWindow();
		if fg.is_null() {
			return None;
		}
		let root = GetAncestor(fg, GA_ROOT);
		if !is_shell_root(root) {
			return None;
		}
		let thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
		let mut info = GUITHREADINFO {
			cbSize: size_of::<GUITHREADINFO>() as u32,
			..std::mem::zeroed()
		};
		if GetGUIThreadInfo(thread, &mut info) == 0 || info.hwndFocus.is_null() {
			return None;
		}
		// the item list itself; a rename box or the search box has the keyboard for typing
		if !matches!(class(info.hwndFocus).as_str(), "DirectUIHWND" | "SysListView32") {
			return None;
		}
		let mut w = info.hwndFocus;
		while !w.is_null() {
			if class(w) == "SHELLDLL_DefView" {
				return Some((root, w));
			}
			w = GetParent(w);
		}
		None
	}
}

/// Reads the selection of the shell view `view` inside the top window `root`. Only windows with
/// that top window are asked, since every question to Explorer crosses into its process; its
/// tabs share one top window, so the view window tells them apart.
fn files_in(root: HWND, view: HWND) -> windows::core::Result<Vec<PathBuf>> {
	let windows = SHELL.with_borrow_mut(|slot| -> windows::core::Result<IShellWindows> {
		if slot.is_none() {
			*slot = Some(unsafe { CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)? });
		}
		Ok(slot.clone().expect("set just above"))
	})?;
	unsafe {
		let mut candidates: Vec<IDispatch> = Vec::new();
		if class(root) == "CabinetWClass" {
			for i in 0..windows.Count()? {
				let Ok(d) = windows.Item(&VARIANT::from(i)) else {
					continue;
				};
				let same_window = d
					.cast::<IWebBrowserApp>()
					.and_then(|b| b.HWND())
					.is_ok_and(|h| h.0 as HWND == root);
				if same_window {
					candidates.push(d);
				}
			}
		} else {
			let mut desktop_hwnd = 0;
			// CSIDL_DESKTOP is 0
			candidates.push(windows.FindWindowSW(
				&VARIANT::from(0i32),
				&VARIANT::default(),
				SWC_DESKTOP,
				&mut desktop_hwnd,
				SWFO_NEEDDISPATCH,
			)?);
		}
		for d in candidates {
			let Ok(sp) = d.cast::<IServiceProvider>() else { continue };
			let Ok(browser) = sp.QueryService::<IShellBrowser>(&SID_STopLevelBrowser) else {
				continue;
			};
			let Ok(shell_view) = browser.QueryActiveShellView() else {
				continue;
			};
			if shell_view.GetWindow().map(|h| h.0) != Ok(view) {
				continue;
			}
			let folder: IFolderView = shell_view.cast()?;
			let items: IShellItemArray = folder.Items(SVGIO_SELECTION)?;
			let mut files = Vec::new();
			for i in 0..items.GetCount()? {
				let item = items.GetItemAt(i)?;
				// items with no path on disk, like This PC or a phone, are left out
				if let Ok(name) = item.GetDisplayName(SIGDN_FILESYSPATH) {
					let path = name.to_string();
					CoTaskMemFree(Some(name.0 as *const _));
					if let Ok(path) = path {
						files.push(PathBuf::from(path));
					}
				}
			}
			return Ok(files);
		}
		Ok(Vec::new())
	}
}
