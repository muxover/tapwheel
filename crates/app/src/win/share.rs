use std::path::{Path, PathBuf};

use windows::ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager};
use windows::Foundation::TypedEventHandler;
use windows::Storage::{IStorageItem, StorageFile, StorageFolder};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::{
	FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, FileOpenDialog, IDataTransferManagerInterop, IFileOpenDialog, IShellItem,
	SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
};
use windows::core::{AgileReference, HSTRING, Interface};
use windows_sys::Win32::Foundation::{HWND as RawHwnd, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::{
	BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetForegroundWindow,
	GetMessageW, GetWindowThreadProcessId, HWND_TOPMOST, KillTimer, LWA_ALPHA, MSG, PostMessageW, PostQuitMessage,
	RegisterClassW, SW_SHOW, SWP_SHOWWINDOW, SetForegroundWindow, SetLayeredWindowAttributes, SetTimer, SetWindowPos,
	ShowWindow, TranslateMessage, WM_DESTROY, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
	WS_POPUP,
};

use super::util::{cursor, wide};

/// How long the share sheet's window stays around if nothing is chosen: the sheet closes with it.
const LINGER_MS: u32 = 5 * 60 * 1000;
/// After an app is chosen, a little longer so it can read the files.
const CHOSEN_MS: u32 = 30 * 1000;
const T_DONE: usize = 1;

/// Why the last share didn't open, for the card; read once by the UI thread.
static FAILED: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

pub fn take_failure() -> Option<String> {
	FAILED.lock().ok()?.take()
}

unsafe extern "system" fn anchor_proc(hwnd: RawHwnd, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	match msg {
		WM_TIMER => {
			unsafe { KillTimer(hwnd, T_DONE) };
			unsafe { DestroyWindow(hwnd) };
			0
		}
		WM_DESTROY => {
			unsafe { PostQuitMessage(0) };
			0
		}
		_ => unsafe { DefWindowProcW(hwnd, msg, w, l) },
	}
}

/// A 1 × 1, all but see-through window at the pointer for the share sheet to hang off: Windows only
/// shows the sheet for a visible window in front, and TapWheel's own is hidden.
fn anchor() -> Option<RawHwnd> {
	let pt = cursor();
	unsafe {
		let inst = GetModuleHandleW(std::ptr::null());
		let class = wide("TapWheelShareAnchor");
		let wc = WNDCLASSW {
			lpfnWndProc: Some(anchor_proc),
			hInstance: inst,
			lpszClassName: class.as_ptr(),
			..std::mem::zeroed()
		};
		// fails harmlessly after the first time
		RegisterClassW(&wc);
		let hwnd = CreateWindowExW(
			WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TOPMOST,
			class.as_ptr(),
			class.as_ptr(),
			WS_POPUP,
			pt.x,
			pt.y,
			1,
			1,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			inst,
			std::ptr::null(),
		);
		if hwnd.is_null() {
			return None;
		}
		SetLayeredWindowAttributes(hwnd, 0, 1, LWA_ALPHA);
		SetWindowPos(hwnd, HWND_TOPMOST, pt.x, pt.y, 1, 1, SWP_SHOWWINDOW);
		ShowWindow(hwnd, SW_SHOW);
		// Windows only lets the app the user is working in bring a window to the front; joining
		// that app's input for a moment is the documented way to hand the front to this one
		let front = GetForegroundWindow();
		let theirs = GetWindowThreadProcessId(front, std::ptr::null_mut());
		let ours = GetCurrentThreadId();
		let joined = theirs != 0 && theirs != ours && super::util::attach_input(ours, theirs, true);
		BringWindowToTop(hwnd);
		SetForegroundWindow(hwnd);
		SetFocus(hwnd);
		if joined {
			super::util::attach_input(ours, theirs, false);
		}
		Some(hwnd)
	}
}

fn item(p: &Path) -> windows::core::Result<IStorageItem> {
	let path = HSTRING::from(p.as_os_str());
	if p.is_dir() {
		StorageFolder::GetFolderFromPathAsync(&path)?.join()?.cast()
	} else {
		StorageFile::GetFileFromPathAsync(&path)?.join()?.cast()
	}
}

/// The share sheet at the pointer, on its own thread so looking the files up never stalls the app;
/// a failure is logged and `main` is told to show it on a card.
pub fn share(files: &[PathBuf], main: isize) {
	let files = files.to_vec();
	std::thread::spawn(move || {
		if let Err(e) = share_here(&files) {
			let why = format!("{} ({:#010x})", e.message(), e.code().0);
			super::log::write(&format!("share: {why}"));
			if let Ok(mut f) = FAILED.lock() {
				*f = Some(why);
			}
			unsafe { PostMessageW(main as _, super::WM_APP_JOBS, 0, 0) };
		}
	});
}

fn share_here(files: &[PathBuf]) -> windows::core::Result<()> {
	unsafe {
		windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_APARTMENTTHREADED)
			.ok()?;
	}
	fn step(what: &'static str) -> impl Fn(windows::core::Error) -> windows::core::Error {
		move |e| windows::core::Error::new(e.code(), format!("{what}: {}", e.message()))
	}
	let items: Vec<Option<IStorageItem>> = files
		.iter()
		.map(|p| item(p).map(Some))
		.collect::<Result<_, _>>()
		.map_err(step("reading the files"))?;
	let title = match files {
		[one] => one
			.file_name()
			.map(|n| n.to_string_lossy().into_owned())
			.unwrap_or_default(),
		many => format!("{} items", many.len()),
	};
	let raw =
		anchor().ok_or_else(|| windows::core::Error::new(windows::Win32::Foundation::E_FAIL, "making its window"))?;
	let window = HWND(raw as *mut _);
	let interop =
		windows::core::factory::<DataTransferManager, IDataTransferManagerInterop>().map_err(step("starting"))?;
	let dtm: DataTransferManager = unsafe { interop.GetForWindow(window) }.map_err(step("starting"))?;
	let list: windows_collections::IIterable<IStorageItem> = items.into();
	let list = AgileReference::new(&list)?;
	dtm.DataRequested(&TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(
		move |_, args| {
			if let Some(args) = args.as_ref() {
				let data = args.Request()?.Data()?;
				data.Properties()?.SetTitle(&HSTRING::from(title.as_str()))?;
				data.SetStorageItemsReadOnly(&list.resolve()?)?;
			}
			Ok(())
		},
	))?;
	let anchor_raw = raw as usize;
	dtm.TargetApplicationChosen(&TypedEventHandler::new(move |_, _| {
		unsafe { SetTimer(anchor_raw as _, T_DONE, CHOSEN_MS, None) };
		Ok(())
	}))?;
	unsafe {
		SetTimer(raw, T_DONE, LINGER_MS, None);
		interop
			.ShowShareUIForWindow(window)
			.map_err(step("showing the sheet"))?;
		// the sheet talks to this thread; keep it answering until the window goes
		let mut msg: MSG = std::mem::zeroed();
		while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
			TranslateMessage(&msg);
			DispatchMessageW(&msg);
		}
	}
	Ok(())
}

/// The folder picker, starting in `start` when given.
pub fn pick_folder(owner: isize, start: Option<&Path>) -> Option<PathBuf> {
	unsafe {
		let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER).ok()?;
		let opts = dialog.GetOptions().ok()?;
		dialog.SetOptions(opts | FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM).ok()?;
		dialog.SetTitle(&HSTRING::from("Move to")).ok()?;
		dialog.SetOkButtonLabel(&HSTRING::from("Move here")).ok()?;
		if let Some(dir) = start
			&& let Ok(folder) = SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(dir.as_os_str()), None)
		{
			let _ = dialog.SetFolder(&folder);
		}
		dialog.Show(Some(HWND(owner as *mut _))).ok()?;
		let result = dialog.GetResult().ok()?;
		let name = result.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
		let path = name.to_string().ok();
		windows::Win32::System::Com::CoTaskMemFree(Some(name.0 as *const _));
		path.map(PathBuf::from)
	}
}
