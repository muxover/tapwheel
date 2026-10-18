use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::Error;

fn wide(p: &Path) -> Vec<u16> {
	p.as_os_str().encode_wide().chain(Some(0)).collect()
}

/// The program's first icon at each common size, largest first.
pub fn extract(path: &Path) -> Result<Vec<(u32, u32, Vec<u8>)>, Error> {
	let name = wide(path);
	let mut out = Vec::new();
	for size in [256i32, 128, 64, 48, 32, 24, 16] {
		let mut icon: HICON = std::ptr::null_mut();
		let mut id = 0u32;
		let got = unsafe { PrivateExtractIconsW(name.as_ptr(), 0, size, size, &mut icon, &mut id, 1, 0) };
		if got == 0 || got == u32::MAX || icon.is_null() {
			continue;
		}
		let px = unsafe { pixels(icon) };
		unsafe { DestroyIcon(icon) };
		if let Some(px) = px
			&& !out.iter().any(|p: &(u32, u32, Vec<u8>)| p.0 == px.0)
		{
			out.push(px);
		}
	}
	if out.is_empty() {
		return Err(Error::Engine("this program has no icon".into()));
	}
	Ok(out)
}

unsafe fn pixels(icon: HICON) -> Option<(u32, u32, Vec<u8>)> {
	unsafe {
		let mut info: ICONINFO = std::mem::zeroed();
		if GetIconInfo(icon, &mut info) == 0 {
			return None;
		}
		let mut bm: BITMAP = std::mem::zeroed();
		GetObjectW(
			info.hbmColor as _,
			std::mem::size_of::<BITMAP>() as i32,
			&mut bm as *mut _ as *mut _,
		);
		let (w, h) = (bm.bmWidth, bm.bmHeight);
		let mut bmi: BITMAPINFO = std::mem::zeroed();
		bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
		bmi.bmiHeader.biWidth = w;
		bmi.bmiHeader.biHeight = -h;
		bmi.bmiHeader.biPlanes = 1;
		bmi.bmiHeader.biBitCount = 32;
		bmi.bmiHeader.biCompression = BI_RGB;
		let mut bgra = vec![0u8; (w * h * 4) as usize];
		let dc = GetDC(std::ptr::null_mut());
		let lines = GetDIBits(
			dc,
			info.hbmColor,
			0,
			h as u32,
			bgra.as_mut_ptr() as *mut _,
			&mut bmi,
			DIB_RGB_COLORS,
		);
		// icons without their own alpha use the mask instead
		let mut mask = vec![0u8; (w * h * 4) as usize];
		let has_alpha = bgra.as_chunks::<4>().0.iter().any(|p| p[3] != 0);
		if !has_alpha {
			GetDIBits(
				dc,
				info.hbmMask,
				0,
				h as u32,
				mask.as_mut_ptr() as *mut _,
				&mut bmi,
				DIB_RGB_COLORS,
			);
		}
		ReleaseDC(std::ptr::null_mut(), dc);
		DeleteObject(info.hbmColor as _);
		DeleteObject(info.hbmMask as _);
		if lines == 0 {
			return None;
		}
		let rgba = bgra
			.as_chunks::<4>()
			.0
			.iter()
			.zip(mask.as_chunks::<4>().0.iter())
			.flat_map(|(p, m)| {
				let a = if has_alpha {
					p[3]
				} else if m[0] == 0 {
					255
				} else {
					0
				};
				[p[2], p[1], p[0], a]
			})
			.collect();
		Some((w as u32, h as u32, rgba))
	}
}
