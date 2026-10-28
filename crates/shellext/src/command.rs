use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
	CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_NOTIMPL, E_POINTER, HMODULE, LPARAM, S_FALSE, WPARAM,
};
use windows::Win32::System::Com::{CoTaskMemAlloc, IBindCtx, IClassFactory, IClassFactory_Impl};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::{
	GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW,
	GetModuleHandleExW,
};
use windows::Win32::UI::Shell::{
	ECF_DEFAULT, ECS_ENABLED, IEnumExplorerCommand, IExplorerCommand, IExplorerCommand_Impl, IShellItemArray,
	SIGDN_FILESYSPATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
	ASFW_ANY, AllowSetForegroundWindow, FindWindowW, SendMessageW, WM_COPYDATA,
};
use windows::core::{BOOL, GUID, HRESULT, Interface, PCWSTR, PWSTR, Ref, Result, implement, w};

/// The class Windows asks for when it builds the menu. The package manifest names the same one.
pub const CLSID: GUID = GUID::from_u128(0x7148b49a_3df7_4720_a9d0_4580d276708f);

/// How long a TapWheel that was just started gets to open its window.
const START_WAIT: Duration = Duration::from_secs(5);

#[implement(IExplorerCommand)]
struct Command;

impl IExplorerCommand_Impl for Command_Impl {
	fn GetTitle(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
		co_string("TapWheel")
	}

	fn GetIcon(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
		let exe = app_exe().ok_or(windows::core::Error::from(E_NOTIMPL))?;
		co_string(&format!("{},0", exe.display()))
	}

	fn GetToolTip(&self, _items: Ref<IShellItemArray>) -> Result<PWSTR> {
		Err(E_NOTIMPL.into())
	}

	fn GetCanonicalName(&self) -> Result<GUID> {
		Ok(CLSID)
	}

	fn GetState(&self, _items: Ref<IShellItemArray>, _slow_ok: BOOL) -> Result<u32> {
		Ok(ECS_ENABLED.0 as u32)
	}

	fn Invoke(&self, items: Ref<IShellItemArray>, _ctx: Ref<IBindCtx>) -> Result<()> {
		let files = paths(items.ok()?)?;
		if files.is_empty() {
			return Ok(());
		}
		let request = crate::pick_request(&files);
		if send(&request) {
			return Ok(());
		}
		let exe = app_exe().ok_or(windows::core::Error::from(E_POINTER))?;
		std::process::Command::new(&exe)
			.arg("--background")
			.spawn()
			.map_err(|e| windows::core::Error::new(E_POINTER, e.to_string()))?;
		let start = Instant::now();
		while start.elapsed() < START_WAIT {
			std::thread::sleep(Duration::from_millis(50));
			if send(&request) {
				return Ok(());
			}
		}
		Err(windows::core::Error::new(E_POINTER, "TapWheel didn't start in time"))
	}

	fn GetFlags(&self) -> Result<u32> {
		Ok(ECF_DEFAULT.0 as u32)
	}

	fn EnumSubCommands(&self) -> Result<IEnumExplorerCommand> {
		Err(E_NOTIMPL.into())
	}
}

/// The selected items that are files or folders on disk. Others, like This PC, are left out.
fn paths(items: &IShellItemArray) -> Result<Vec<PathBuf>> {
	let mut files = Vec::new();
	unsafe {
		for i in 0..items.GetCount()? {
			let item = items.GetItemAt(i)?;
			if let Ok(name) = item.GetDisplayName(SIGDN_FILESYSPATH) {
				let path = name.to_string();
				windows::Win32::System::Com::CoTaskMemFree(Some(name.0 as *const c_void));
				if let Ok(path) = path {
					files.push(PathBuf::from(path));
				}
			}
		}
	}
	Ok(files)
}

/// Hands the request to TapWheel's main window. False when TapWheel isn't running.
fn send(request: &str) -> bool {
	unsafe {
		let Ok(hwnd) = FindWindowW(w!("TapWheel.Main"), PCWSTR::null()) else {
			return false;
		};
		let data: Vec<u16> = request.encode_utf16().collect();
		let cds = COPYDATASTRUCT {
			dwData: 1,
			cbData: (data.len() * 2) as u32,
			lpData: data.as_ptr() as *mut c_void,
		};
		// the wheel takes the keyboard when it opens
		let _ = AllowSetForegroundWindow(ASFW_ANY);
		SendMessageW(
			hwnd,
			WM_COPYDATA,
			Some(WPARAM(0)),
			Some(LPARAM(&cds as *const _ as isize)),
		)
		.0 != 0
	}
}

/// `tapwheel.exe` in the package, next to this DLL.
fn app_exe() -> Option<PathBuf> {
	Some(dll_dir()?.join("tapwheel.exe"))
}

fn dll_dir() -> Option<PathBuf> {
	unsafe {
		let mut module = HMODULE::default();
		GetModuleHandleExW(
			GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
			PCWSTR(dll_dir as *const () as *const u16),
			&mut module,
		)
		.ok()?;
		let mut buf = vec![0u16; 1024];
		let n = GetModuleFileNameW(Some(module), &mut buf) as usize;
		if n == 0 || n == buf.len() {
			return None;
		}
		Path::new(&String::from_utf16_lossy(&buf[..n]))
			.parent()
			.map(Path::to_path_buf)
	}
}

/// A string Explorer frees with `CoTaskMemFree`.
fn co_string(s: &str) -> Result<PWSTR> {
	let wide: Vec<u16> = s.encode_utf16().chain(Some(0)).collect();
	unsafe {
		let p = CoTaskMemAlloc(wide.len() * 2) as *mut u16;
		if p.is_null() {
			return Err(E_POINTER.into());
		}
		std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
		Ok(PWSTR(p))
	}
}

#[implement(IClassFactory)]
struct Factory;

impl IClassFactory_Impl for Factory_Impl {
	fn CreateInstance(
		&self,
		outer: Ref<windows::core::IUnknown>,
		iid: *const GUID,
		object: *mut *mut c_void,
	) -> Result<()> {
		if object.is_null() || iid.is_null() {
			return Err(E_POINTER.into());
		}
		unsafe { *object = std::ptr::null_mut() };
		if outer.is_some() {
			return Err(CLASS_E_NOAGGREGATION.into());
		}
		let command: IExplorerCommand = Command.into();
		unsafe { command.query(iid, object).ok() }
	}

	fn LockServer(&self, _lock: BOOL) -> Result<()> {
		Ok(())
	}
}

/// # Safety
/// Called by COM with valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetClassObject(
	clsid: *const GUID,
	iid: *const GUID,
	out: *mut *mut c_void,
) -> HRESULT {
	if clsid.is_null() || iid.is_null() || out.is_null() {
		return E_POINTER;
	}
	unsafe {
		*out = std::ptr::null_mut();
		if *clsid != CLSID {
			return CLASS_E_CLASSNOTAVAILABLE;
		}
		let factory: IClassFactory = Factory.into();
		factory.query(iid, out)
	}
}

/// The surrogate process keeps the DLL for its whole life; it's small.
#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
	S_FALSE
}

#[cfg(test)]
mod tests {
	use super::*;
	use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
	use windows::Win32::UI::Shell::{IShellItem, SHCreateItemFromParsingName, SHCreateShellItemArrayFromShellItem};

	fn command() -> IExplorerCommand {
		unsafe {
			let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
			let mut factory: *mut c_void = std::ptr::null_mut();
			DllGetClassObject(&CLSID, &IClassFactory::IID, &mut factory)
				.ok()
				.unwrap();
			let factory = IClassFactory::from_raw(factory);
			factory
				.CreateInstance::<_, IExplorerCommand>(None::<&windows::core::IUnknown>)
				.unwrap()
		}
	}

	#[test]
	fn only_tapwheels_class_is_served() {
		let other = GUID::from_u128(0x1);
		let mut out: *mut c_void = std::ptr::null_mut();
		let hr = unsafe { DllGetClassObject(&other, &IClassFactory::IID, &mut out) };
		assert_eq!(hr, CLASS_E_CLASSNOTAVAILABLE);
		assert!(out.is_null());
	}

	#[test]
	fn the_menu_entry_reads_tapwheel_and_is_enabled_for_a_file() {
		let cmd = command();
		let file = std::env::temp_dir().join(format!("tw-shell-{}.txt", std::process::id()));
		std::fs::write(&file, "hello").unwrap();
		let wide: Vec<u16> = file
			.as_os_str()
			.to_string_lossy()
			.encode_utf16()
			.chain(Some(0))
			.collect();
		unsafe {
			let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).unwrap();
			let items = SHCreateShellItemArrayFromShellItem(&item).unwrap();
			let title = cmd.GetTitle(&items).unwrap();
			assert_eq!(title.to_string().unwrap(), "TapWheel");
			windows::Win32::System::Com::CoTaskMemFree(Some(title.0 as *const c_void));
			assert_eq!(cmd.GetState(&items, true).unwrap(), ECS_ENABLED.0 as u32);
			assert_eq!(cmd.GetCanonicalName().unwrap(), CLSID);
			assert_eq!(cmd.GetFlags().unwrap(), ECF_DEFAULT.0 as u32);
			assert!(cmd.EnumSubCommands().is_err());
			assert_eq!(paths(&items).unwrap(), vec![file.clone()]);
		}
		std::fs::remove_file(file).unwrap();
	}
}
