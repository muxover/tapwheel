//! The Windows share window (Nearby Sharing, mail, apps) and the folder picker.

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

fn item(p: &Path) -> windows::core::Result<IStorageItem> {
	let path = HSTRING::from(p.as_os_str());
	if p.is_dir() {
		StorageFolder::GetFolderFromPathAsync(&path)?.join()?.cast()
	} else {
		StorageFile::GetFileFromPathAsync(&path)?.join()?.cast()
	}
}

/// Opens the share window for these files, next to `hwnd`.
pub fn share(hwnd: isize, files: &[PathBuf]) -> windows::core::Result<()> {
	let items: Vec<Option<IStorageItem>> = files.iter().map(|p| item(p).map(Some)).collect::<Result<_, _>>()?;
	let title = match files {
		[one] => one
			.file_name()
			.map(|n| n.to_string_lossy().into_owned())
			.unwrap_or_default(),
		many => format!("{} items", many.len()),
	};
	let interop = windows::core::factory::<DataTransferManager, IDataTransferManagerInterop>()?;
	let window = HWND(hwnd as *mut _);
	let dtm: DataTransferManager = unsafe { interop.GetForWindow(window)? };
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
	dtm.DataRequested(&handler)?;
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
