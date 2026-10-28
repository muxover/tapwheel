//! The files selected in the Explorer window, or on the desktop, that has the keyboard, so the
//! wheel can open on them without a drag.

use std::path::PathBuf;

use windows::Win32::System::Com::{CLSCTX_LOCAL_SERVER, CoCreateInstance, CoTaskMemFree, IDispatch, IServiceProvider};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::{
	IFolderView, IShellBrowser, IShellItemArray, IShellWindows, SID_STopLevelBrowser, SIGDN_FILESYSPATH,
	SVGIO_SELECTION, SWC_DESKTOP, SWFO_NEEDDISPATCH, ShellWindows,
};
use windows::core::Interface;
use windows_sys::Win32::UI::WindowsAndMessaging::{
	GA_ROOT, GUITHREADINFO, GetAncestor, GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetParent,
	GetWindowThreadProcessId,
};

/// The selected files, or `None` when the keyboard isn't on the items of Explorer or the desktop
/// (an address bar, a rename box, a search box, a file dialog or another app), or nothing on
/// disk is selected. Items with no file behind them, such as This PC, are left out.
pub fn selected() -> Option<Vec<PathBuf>> {
	let view = focused_view()?;
	let files = files_in(view)
		.inspect_err(|e| super::log::write(&format!("reading Explorer's selection failed: {e}")))
		.ok()?;
	(!files.is_empty()).then_some(files)
}

fn class(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
	let mut buf = [0u16; 64];
	let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) } as usize;
	String::from_utf16_lossy(&buf[..n])
}

/// Where the keyboard is: Explorer's or the desktop's item view, as its `SHELLDLL_DefView` window.
fn focused_view() -> Option<windows_sys::Win32::Foundation::HWND> {
	unsafe {
		let fg = GetForegroundWindow();
		if fg.is_null()
			|| !matches!(
				class(GetAncestor(fg, GA_ROOT)).as_str(),
				"CabinetWClass" | "Progman" | "WorkerW"
			) {
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
				return Some(w);
			}
			w = GetParent(w);
		}
		None
	}
}

/// Finds the shell view that owns `view` among Explorer's windows and the desktop, and reads its
/// selection. Explorer's tabs share one top window, so the view window is what tells them apart.
fn files_in(view: windows_sys::Win32::Foundation::HWND) -> windows::core::Result<Vec<PathBuf>> {
	unsafe {
		let windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;
		let mut candidates: Vec<IDispatch> = Vec::new();
		for i in 0..windows.Count()? {
			if let Ok(d) = windows.Item(&VARIANT::from(i)) {
				candidates.push(d);
			}
		}
		let mut desktop_hwnd = 0;
		// CSIDL_DESKTOP is 0
		if let Ok(d) = windows.FindWindowSW(
			&VARIANT::from(0i32),
			&VARIANT::default(),
			SWC_DESKTOP,
			&mut desktop_hwnd,
			SWFO_NEEDDISPATCH,
		) {
			candidates.push(d);
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
