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
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
	CreateWindowExW, DefWindowProcW, HWND_TOPMOST, LWA_ALPHA, RegisterClassW, SWP_NOACTIVATE, SWP_SHOWWINDOW,
	SetLayeredWindowAttributes, SetWindowPos, WNDCLASSW, WS_EX_LAYERED, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use super::util::{cursor, wide};

/// The window the share sheet hangs off. Windows only shows the sheet for a visible window, and
/// TapWheel's own is hidden, so this is a 1 × 1 one at the pointer, all but see-through, made
/// once and moved there for each share.
fn anchor() -> Option<HWND> {
	use std::sync::atomic::{AtomicIsize, Ordering};
	static ANCHOR: AtomicIsize = AtomicIsize::new(0);
	let pt = cursor();
	unsafe {
		let mut h = ANCHOR.load(Ordering::Relaxed);
		if h == 0 {
			let inst = GetModuleHandleW(std::ptr::null());
			let class = wide("TapWheelShareAnchor");
			let wc = WNDCLASSW {
				lpfnWndProc: Some(DefWindowProcW),
				hInstance: inst,
				lpszClassName: class.as_ptr(),
				..std::mem::zeroed()
			};
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
			h = hwnd as isize;
			ANCHOR.store(h, Ordering::Relaxed);
		}
		SetWindowPos(h as _, HWND_TOPMOST, pt.x, pt.y, 1, 1, SWP_SHOWWINDOW | SWP_NOACTIVATE);
		Some(HWND(h as *mut _))
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

/// Opens the Windows share sheet for these files, at the pointer.
pub fn share(files: &[PathBuf]) -> windows::core::Result<()> {
	let items: Vec<Option<IStorageItem>> = files.iter().map(|p| item(p).map(Some)).collect::<Result<_, _>>()?;
	let title = match files {
		[one] => one
			.file_name()
			.map(|n| n.to_string_lossy().into_owned())
			.unwrap_or_default(),
		many => format!("{} items", many.len()),
	};
	let interop = windows::core::factory::<DataTransferManager, IDataTransferManagerInterop>()?;
	let window = anchor().ok_or_else(|| windows::core::Error::from(windows::Win32::Foundation::E_FAIL))?;
	let dtm: DataTransferManager = unsafe { interop.GetForWindow(window)? };
	// one handler at a time: the last share's would hand over the wrong files
	static LAST: std::sync::Mutex<Option<i64>> = std::sync::Mutex::new(None);
	// the handler may run on another thread, so it holds the list through an agile reference
	let list: windows_collections::IIterable<IStorageItem> = items.into();
	let list = AgileReference::new(&list)?;
	let handler = TypedEventHandler::<DataTransferManager, DataRequestedEventArgs>::new(move |_, args| {
		if let Some(args) = args.as_ref() {
			let data = args.Request()?.Data()?;
			data.Properties()?.SetTitle(&HSTRING::from(title.as_str()))?;
			data.SetStorageItemsReadOnly(&list.resolve()?)?;
		}
		Ok(())
	});
	if let Some(old) = LAST.lock().unwrap().take() {
		let _ = dtm.RemoveDataRequested(old);
	}
	*LAST.lock().unwrap() = Some(dtm.DataRequested(&handler)?);
	unsafe { interop.ShowShareUIForWindow(window) }
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
