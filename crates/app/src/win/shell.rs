use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::{HANDLE, HWND};
use windows_sys::Win32::System::DataExchange::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Ole::{CF_HDROP, CF_UNICODETEXT};
use windows_sys::Win32::System::Registry::HKEY_CURRENT_USER;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::util::*;

fn global(bytes: &[u8]) -> HANDLE {
	unsafe {
		let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
		if h.is_null() {
			return std::ptr::null_mut();
		}
		let p = GlobalLock(h) as *mut u8;
		std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
		GlobalUnlock(h);
		h as HANDLE
	}
}

fn with_clipboard(owner: HWND, f: impl FnOnce()) -> bool {
	unsafe {
		// the clipboard can be briefly held by another app
		for _ in 0..10 {
			if OpenClipboard(owner) != 0 {
				EmptyClipboard();
				f();
				CloseClipboard();
				return true;
			}
			std::thread::sleep(std::time::Duration::from_millis(20));
		}
	}
	false
}

pub fn copy_text(owner: HWND, text: &str) -> bool {
	let data: Vec<u8> = wide(text).iter().flat_map(|c| c.to_le_bytes()).collect();
	with_clipboard(owner, || unsafe {
		SetClipboardData(CF_UNICODETEXT as u32, global(&data));
	})
}

/// The clipboard's text, if it holds any.
pub fn clipboard_text(owner: HWND) -> Option<String> {
	unsafe {
		if OpenClipboard(owner) == 0 {
			return None;
		}
		let h = GetClipboardData(CF_UNICODETEXT as u32);
		let text = if h.is_null() {
			None
		} else {
			let p = GlobalLock(h as _) as *const u16;
			if p.is_null() {
				None
			} else {
				let mut n = 0;
				while *p.add(n) != 0 {
					n += 1;
				}
				let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, n));
				GlobalUnlock(h as _);
				Some(s)
			}
		};
		CloseClipboard();
		text
	}
}

/// Moves items into a folder through the shell, so Ctrl+Z in Explorer can undo it.
pub fn move_to(items: &[PathBuf], dest: &Path) -> bool {
	let mut from: Vec<u16> = Vec::new();
	for p in items {
		from.extend(wide(p.as_os_str()));
	}
	from.push(0);
	let mut to = wide(dest.as_os_str());
	to.push(0);
	let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
	op.wFunc = FO_MOVE;
	op.pFrom = from.as_ptr();
	op.pTo = to.as_ptr();
	op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMMKDIR) as u16;
	unsafe { SHFileOperationW(&mut op) == 0 && op.fAnyOperationsAborted == 0 }
}

/// Puts files on the clipboard the way Explorer's Copy does, ready to paste.
pub fn copy_files(owner: HWND, files: &[PathBuf]) -> bool {
	let header = std::mem::size_of::<DROPFILES>();
	let mut list: Vec<u16> = Vec::new();
	for f in files {
		list.extend(wide(f.as_os_str()));
	}
	list.push(0);
	let df = DROPFILES {
		pFiles: header as u32,
		pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
		fNC: 0,
		fWide: 1,
	};
	let mut bytes = vec![0u8; header];
	unsafe { std::ptr::copy_nonoverlapping(&df as *const DROPFILES as *const u8, bytes.as_mut_ptr(), header) };
	bytes.extend(list.iter().flat_map(|c| c.to_le_bytes()));
	let effect = 1u32.to_le_bytes();
	with_clipboard(owner, || unsafe {
		SetClipboardData(CF_HDROP as u32, global(&bytes));
		let fmt = RegisterClipboardFormatW(wide("Preferred DropEffect").as_ptr());
		SetClipboardData(fmt, global(&effect));
	})
}

fn shell_open(file: &str, params: Option<&str>, dir: Option<&Path>) -> bool {
	let file = wide(file);
	let params = params.map(wide);
	let dir = dir.map(|d| wide(d.as_os_str()));
	let r = unsafe {
		ShellExecuteW(
			std::ptr::null_mut(),
			wide("open").as_ptr(),
			file.as_ptr(),
			params.as_ref().map_or(std::ptr::null(), |p| p.as_ptr()),
			dir.as_ref().map_or(std::ptr::null(), |d| d.as_ptr()),
			SW_SHOWNORMAL,
		)
	};
	r as isize > 32
}

pub fn open(path: &Path) -> bool {
	shell_open(&path.to_string_lossy(), None, None)
}

pub fn open_url(url: &str) -> bool {
	shell_open(url, None, None)
}

/// Windows Terminal when it's there, PowerShell otherwise.
pub fn terminal(dir: &Path) -> bool {
	let d = dir.to_string_lossy();
	shell_open("wt.exe", Some(&format!("-d \"{d}\"")), Some(dir))
		|| shell_open("powershell.exe", Some("-NoExit"), Some(dir))
}

/// Opens Explorer with these items selected (they must share a folder).
pub fn reveal(items: &[PathBuf]) -> bool {
	let Some(first) = items.first() else {
		return false;
	};
	let Some(folder) = first.parent() else {
		return open(first);
	};
	unsafe {
		let pf = ILCreateFromPathW(wide(folder.as_os_str()).as_ptr());
		if pf.is_null() {
			return shell_open("explorer.exe", Some(&format!("/select,\"{}\"", first.display())), None);
		}
		let kids: Vec<_> = items
			.iter()
			.filter(|p| p.parent() == Some(folder))
			.map(|p| ILCreateFromPathW(wide(p.as_os_str()).as_ptr()))
			.filter(|p| !p.is_null())
			.collect();
		let ptrs: Vec<*const _> = kids.iter().map(|p| *p as *const _).collect();
		let hr = SHOpenFolderAndSelectItems(pf, ptrs.len() as u32, ptrs.as_ptr(), 0);
		for k in kids {
			ILFree(k);
		}
		ILFree(pf);
		hr >= 0
	}
}

/// Sends files to the Recycle Bin, so Undo can itself be undone.
pub fn recycle(items: &[PathBuf]) -> bool {
	let existing: Vec<&PathBuf> = items.iter().filter(|p| p.exists()).collect();
	if existing.is_empty() {
		return true;
	}
	let mut from: Vec<u16> = Vec::new();
	for p in existing {
		from.extend(wide(p.as_os_str()));
	}
	from.push(0);
	let mut op: SHFILEOPSTRUCTW = unsafe { std::mem::zeroed() };
	op.wFunc = FO_DELETE;
	op.pFrom = from.as_ptr();
	op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_SILENT | FOF_NOERRORUI) as u16;
	unsafe { SHFileOperationW(&mut op) == 0 }
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn set_autostart(on: bool, packaged: bool) {
	if packaged {
		std::thread::spawn(move || {
			let _ = startup_task(on);
		});
		return;
	}
	if on {
		let exe = std::env::current_exe().unwrap_or_default();
		reg_set_string(
			HKEY_CURRENT_USER,
			RUN_KEY,
			Some("TapWheel"),
			&format!("\"{}\" --background", exe.display()),
		);
	} else {
		reg_delete_value(HKEY_CURRENT_USER, RUN_KEY, "TapWheel");
	}
}

fn startup_task(on: bool) -> windows::core::Result<()> {
	use windows::ApplicationModel::StartupTask;
	let task = StartupTask::GetAsync(&windows::core::HSTRING::from("TapWheelStartup"))?.join()?;
	if on {
		task.RequestEnableAsync()?.join()?;
	} else {
		task.Disable()?;
	}
	Ok(())
}

const VERB_KEYS: [&str; 2] = [
	r"Software\Classes\*\shell\TapWheel",
	r"Software\Classes\Directory\shell\TapWheel",
];

/// The classic right-click entry for the portable build. The Store package can't write
/// these keys for Explorer to see, so it relies on the drag and the sample files instead.
pub fn set_context_menu(on: bool) {
	let exe = std::env::current_exe().unwrap_or_default();
	for key in VERB_KEYS {
		if on {
			reg_set_string(HKEY_CURRENT_USER, key, None, "TapWheel");
			reg_set_string(
				HKEY_CURRENT_USER,
				key,
				Some("Icon"),
				&format!("\"{}\",0", exe.display()),
			);
			reg_set_string(
				HKEY_CURRENT_USER,
				&format!(r"{key}\command"),
				None,
				&format!("\"{}\" --pick \"%1\"", exe.display()),
			);
		} else {
			reg_delete_tree(HKEY_CURRENT_USER, key);
		}
	}
}

pub fn appdata(var: &str) -> PathBuf {
	std::env::var_os(var)
		.map(PathBuf::from)
		.unwrap_or_else(std::env::temp_dir)
		.join("TapWheel")
}

/// A folder of small files to practise on, made fresh each time.
pub fn samples(logo_png: &[u8], heic: &[u8]) -> std::io::Result<PathBuf> {
	let dir = appdata("LOCALAPPDATA").join("Samples");
	std::fs::create_dir_all(&dir)?;
	std::fs::write(dir.join("TapWheel logo.png"), logo_png)?;
	std::fs::write(dir.join("Phone photo.heic"), heic)?;
	std::fs::write(dir.join("Tone.wav"), tone_wav())?;
	std::fs::write(
		dir.join("Subtitles.srt"),
		"1\r\n00:00:01,000 --> 00:00:03,000\r\nDrag me, then tap Shift.\r\n\r\n2\r\n00:00:03,500 --> 00:00:06,000\r\nDrop me on VTT.\r\n",
	)?;
	Ok(dir)
}

/// Two seconds of a soft 440 Hz tone, 16-bit mono.
fn tone_wav() -> Vec<u8> {
	let rate = 44_100u32;
	let n = rate * 2;
	let mut out = Vec::with_capacity(44 + n as usize * 2);
	out.extend_from_slice(b"RIFF");
	out.extend_from_slice(&(36 + n * 2).to_le_bytes());
	out.extend_from_slice(b"WAVEfmt ");
	out.extend_from_slice(&16u32.to_le_bytes());
	out.extend_from_slice(&1u16.to_le_bytes());
	out.extend_from_slice(&1u16.to_le_bytes());
	out.extend_from_slice(&rate.to_le_bytes());
	out.extend_from_slice(&(rate * 2).to_le_bytes());
	out.extend_from_slice(&2u16.to_le_bytes());
	out.extend_from_slice(&16u16.to_le_bytes());
	out.extend_from_slice(b"data");
	out.extend_from_slice(&(n * 2).to_le_bytes());
	for i in 0..n {
		let t = i as f32 / rate as f32;
		let fade = (t * 8.0).min(1.0).min((2.0 - t) * 8.0);
		let v = (t * 440.0 * std::f32::consts::TAU).sin() * 0.25 * fade;
		out.extend_from_slice(&((v * i16::MAX as f32) as i16).to_le_bytes());
	}
	out
}
