use std::path::PathBuf;

use tapwheel_ui::{Fonts, Rgba, Theme};
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETHIGHCONTRAST, SystemParametersInfoW};

use super::util::{reg_dword, reg_string};

pub fn build_number() -> u32 {
	reg_string(
		HKEY_LOCAL_MACHINE,
		r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
		"CurrentBuildNumber",
	)
	.and_then(|s| s.trim().parse().ok())
	.unwrap_or(19045)
}

pub fn is_win11() -> bool {
	build_number() >= 22000
}

fn high_contrast() -> bool {
	unsafe {
		let mut hc: HIGHCONTRASTW = std::mem::zeroed();
		hc.cbSize = std::mem::size_of::<HIGHCONTRASTW>() as u32;
		SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, &mut hc as *mut _ as *mut _, 0) != 0
			&& hc.dwFlags & HCF_HIGHCONTRASTON != 0
	}
}

/// Reads light/dark, the accent colour and high contrast the way Settings sets them.
pub fn theme() -> Theme {
	let light = reg_dword(
		HKEY_CURRENT_USER,
		r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
		"AppsUseLightTheme",
	)
	.unwrap_or(1);
	// stored as 0xAABBGGRR
	let accent = reg_dword(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\DWM", "AccentColor")
		.map(|v| Rgba::from_u8((v & 0xff) as u8, ((v >> 8) & 0xff) as u8, ((v >> 16) & 0xff) as u8))
		.unwrap_or(Rgba::hex(0x0078d4));
	let hc = high_contrast();
	Theme {
		dark: if hc { !high_contrast_is_light() } else { light == 0 },
		win11: is_win11(),
		accent,
		high_contrast: hc,
	}
}

fn high_contrast_is_light() -> bool {
	use windows_sys::Win32::Graphics::Gdi::GetSysColor;
	use windows_sys::Win32::UI::WindowsAndMessaging::COLOR_WINDOW;
	let c = unsafe { GetSysColor(COLOR_WINDOW) };
	let (r, g, b) = (c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff);
	r + g + b > 384
}

fn fonts_dir() -> PathBuf {
	let mut buf = [0u16; 260];
	let n = unsafe {
		windows_sys::Win32::System::SystemInformation::GetWindowsDirectoryW(buf.as_mut_ptr(), buf.len() as u32)
	};
	let win = String::from_utf16_lossy(&buf[..n as usize]);
	PathBuf::from(if win.is_empty() { r"C:\Windows".to_string() } else { win }).join("Fonts")
}

/// Segoe UI Variable on Windows 11, Segoe UI on Windows 10, Arial if neither is there.
pub fn fonts() -> Fonts {
	let dir = fonts_dir();
	let read = |name: &str| std::fs::read(dir.join(name)).ok();
	let pairs: [(&str, &str); 3] = [
		("SegUIVar.ttf", "SegUIVar.ttf"),
		("segoeui.ttf", "seguisb.ttf"),
		("arial.ttf", "arialbd.ttf"),
	];
	let start = if is_win11() { 0 } else { 1 };
	for (regular, semibold) in &pairs[start..] {
		if let (Some(r), Some(s)) = (read(regular), read(semibold)) {
			if let Some(f) = Fonts::new(r, s) {
				return f;
			}
		}
	}
	panic!("no usable system font in {}", dir.display());
}

pub fn packaged() -> bool {
	let mut len = 0u32;
	let rc = unsafe {
		windows_sys::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName(&mut len, std::ptr::null_mut())
	};
	// APPMODEL_ERROR_NO_PACKAGE
	rc != 15700
}

/// The Store package's startup task launches the app with no arguments; it should stay in the tray.
pub fn started_at_sign_in() -> bool {
	use windows::ApplicationModel::Activation::ActivationKind;
	windows::ApplicationModel::AppInstance::GetActivatedEventArgs()
		.and_then(|a| a.Kind())
		.is_ok_and(|k| k == ActivationKind::StartupTask)
}
