use std::ffi::{OsStr, OsString, c_void};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::{HWND, POINT, RECT, SIZE};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
	s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> OsString {
	let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
	OsString::from_wide(&buf[..end])
}

/// Copies `text` into a fixed UTF-16 buffer, cutting it to fit.
pub fn fill(dst: &mut [u16], text: &str) {
	let src: Vec<u16> = text.encode_utf16().take(dst.len() - 1).collect();
	dst[..src.len()].copy_from_slice(&src);
	dst[src.len()] = 0;
}

pub fn exe_dir() -> PathBuf {
	std::env::current_exe()
		.ok()
		.and_then(|p| p.parent().map(Path::to_path_buf))
		.unwrap_or_default()
}

pub fn reg_dword(root: HKEY, key: &str, value: &str) -> Option<u32> {
	let mut data = 0u32;
	let mut size = 4u32;
	let rc = unsafe {
		RegGetValueW(
			root,
			wide(key).as_ptr(),
			wide(value).as_ptr(),
			RRF_RT_REG_DWORD,
			std::ptr::null_mut(),
			&mut data as *mut u32 as *mut c_void,
			&mut size,
		)
	};
	(rc == 0).then_some(data)
}

pub fn reg_binary(root: HKEY, key: &str, value: &str) -> Option<Vec<u8>> {
	let mut buf = vec![0u8; 256];
	let mut size = buf.len() as u32;
	let rc = unsafe {
		RegGetValueW(
			root,
			wide(key).as_ptr(),
			wide(value).as_ptr(),
			RRF_RT_REG_BINARY,
			std::ptr::null_mut(),
			buf.as_mut_ptr() as *mut c_void,
			&mut size,
		)
	};
	(rc == 0).then(|| {
		buf.truncate(size as usize);
		buf
	})
}

pub fn reg_string(root: HKEY, key: &str, value: &str) -> Option<String> {
	let mut buf = vec![0u16; 512];
	let mut size = (buf.len() * 2) as u32;
	let rc = unsafe {
		RegGetValueW(
			root,
			wide(key).as_ptr(),
			wide(value).as_ptr(),
			RRF_RT_REG_SZ,
			std::ptr::null_mut(),
			buf.as_mut_ptr() as *mut c_void,
			&mut size,
		)
	};
	(rc == 0).then(|| from_wide(&buf).to_string_lossy().into_owned())
}

pub fn reg_set_string(root: HKEY, key: &str, value: Option<&str>, data: &str) -> bool {
	let data = wide(data);
	let name = value.map(wide);
	let rc = unsafe {
		RegSetKeyValueW(
			root,
			wide(key).as_ptr(),
			name.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
			REG_SZ,
			data.as_ptr() as *const c_void,
			(data.len() * 2) as u32,
		)
	};
	rc == 0
}

pub fn reg_delete_value(root: HKEY, key: &str, value: &str) {
	unsafe {
		RegDeleteKeyValueW(root, wide(key).as_ptr(), wide(value).as_ptr());
	}
}

pub fn reg_delete_tree(root: HKEY, key: &str) {
	unsafe {
		RegDeleteTreeW(root, wide(key).as_ptr());
	}
}

pub fn cursor() -> POINT {
	let mut pt = POINT { x: 0, y: 0 };
	unsafe { GetCursorPos(&mut pt) };
	pt
}

/// Work area of the monitor under `pt`, and its scale (1.0 at 96 dpi).
pub fn monitor_at(pt: POINT) -> (RECT, f32) {
	unsafe {
		let mon = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
		let mut info: MONITORINFO = std::mem::zeroed();
		info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
		GetMonitorInfoW(mon, &mut info);
		let (mut dx, mut dy) = (96u32, 96u32);
		windows_sys::Win32::UI::HiDpi::GetDpiForMonitor(
			mon,
			windows_sys::Win32::UI::HiDpi::MDT_EFFECTIVE_DPI,
			&mut dx,
			&mut dy,
		);
		(info.rcWork, dx as f32 / 96.0)
	}
}

fn bitmap_info(w: i32, h: i32) -> BITMAPINFO {
	let mut bi: BITMAPINFO = unsafe { std::mem::zeroed() };
	bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
	bi.bmiHeader.biWidth = w;
	bi.bmiHeader.biHeight = -h;
	bi.bmiHeader.biPlanes = 1;
	bi.bmiHeader.biBitCount = 32;
	bi.bmiHeader.biCompression = BI_RGB;
	bi
}

/// Puts a premultiplied frame on a layered window at (x, y) in screen pixels.
pub fn push_layered(hwnd: HWND, pm: &Pixmap, x: i32, y: i32) {
	push_layered_alpha(hwnd, pm, x, y, 255);
}

/// Same, with the whole frame faded by `alpha`.
pub fn push_layered_alpha(hwnd: HWND, pm: &Pixmap, x: i32, y: i32, alpha: u8) {
	let (w, h) = (pm.width() as i32, pm.height() as i32);
	let bgra = tapwheel_ui::draw::to_bgra(pm);
	unsafe {
		let screen = GetDC(std::ptr::null_mut());
		let mem = CreateCompatibleDC(screen);
		let bi = bitmap_info(w, h);
		let mut bits: *mut c_void = std::ptr::null_mut();
		let bmp = CreateDIBSection(mem, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
		if !bmp.is_null() && !bits.is_null() {
			std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits as *mut u8, bgra.len());
			let old = SelectObject(mem, bmp as HGDIOBJ);
			let dst = POINT { x, y };
			let size = SIZE { cx: w, cy: h };
			let src = POINT { x: 0, y: 0 };
			let blend = BLENDFUNCTION {
				BlendOp: AC_SRC_OVER as u8,
				BlendFlags: 0,
				SourceConstantAlpha: alpha,
				AlphaFormat: AC_SRC_ALPHA as u8,
			};
			UpdateLayeredWindow(hwnd, screen, &dst, &size, mem, &src, 0, &blend, ULW_ALPHA);
			SelectObject(mem, old);
			DeleteObject(bmp as HGDIOBJ);
		}
		DeleteDC(mem);
		ReleaseDC(std::ptr::null_mut(), screen);
	}
}

pub fn get_x(l: isize) -> i32 {
	(l & 0xffff) as i16 as i32
}

pub fn get_y(l: isize) -> i32 {
	((l >> 16) & 0xffff) as i16 as i32
}

/// Joins or leaves another thread's input queue, for handing the foreground to a new window.
pub fn attach_input(ours: u32, theirs: u32, on: bool) -> bool {
	unsafe { windows_sys::Win32::System::Threading::AttachThreadInput(ours, theirs, on as i32) != 0 }
}
