//! Reads what this PC has, for `tapwheel_core::device`: cores, memory, the processor's family,
//! the graphics card, the temp drive, power and the screen. Each part that can't be read is left
//! unknown rather than guessed.

use std::os::windows::ffi::OsStrExt;

use tapwheel_core::device::{Device, Power};
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Graphics::Gdi::{
	DEVMODEW, DISPLAY_DEVICE_PRIMARY_DEVICE, DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, EnumDisplayDevicesW,
	EnumDisplaySettingsW,
};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{
	DEVICE_SEEK_PENALTY_DESCRIPTOR, IOCTL_STORAGE_QUERY_PROPERTY, PropertyStandardQuery, STORAGE_PROPERTY_QUERY,
	StorageDeviceSeekPenaltyProperty,
};
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows_sys::Win32::System::SystemInformation::{
	GlobalMemoryStatusEx, IMAGE_FILE_MACHINE_ARM64, IMAGE_FILE_MACHINE_UNKNOWN, MEMORYSTATUSEX,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};

/// Reads the PC now. Cheap enough to run at start and again when power changes.
pub fn read() -> Device {
	Device {
		cores: std::thread::available_parallelism().map_or(1, |n| n.get()),
		memory: total_memory(),
		arch: if native_arm() { "arm64" } else { "x64" }.into(),
		emulated: native_arm() && cfg!(target_arch = "x86_64"),
		gpu: gpu_name(),
		software_gpu: gpu_name().is_some_and(|n| n.starts_with("Microsoft Basic")),
		ssd: temp_drive_is_ssd(),
		power: power(),
		refresh_hz: refresh_hz(),
		animations: !super::app::reduced_motion(),
	}
}

/// Battery and Battery saver only, for the moments power changes.
pub fn power() -> Option<Power> {
	let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
	if unsafe { GetSystemPowerStatus(&mut s) } == 0 {
		return None;
	}
	// 128 is "no system battery"; 255 is "unknown"
	if s.BatteryFlag != 255 && s.BatteryFlag & 128 != 0 {
		return None;
	}
	Some(Power {
		on_battery: s.ACLineStatus == 0,
		percent: (s.BatteryLifePercent <= 100).then_some(s.BatteryLifePercent),
		saver: s.SystemStatusFlag & 1 != 0,
	})
}

fn total_memory() -> u64 {
	let mut m: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
	m.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
	if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 {
		0
	} else {
		m.ullTotalPhys
	}
}

fn native_arm() -> bool {
	let (mut process, mut native) = (IMAGE_FILE_MACHINE_UNKNOWN, IMAGE_FILE_MACHINE_UNKNOWN);
	let ok = unsafe { IsWow64Process2(GetCurrentProcess(), &mut process, &mut native) } != 0;
	ok && native == IMAGE_FILE_MACHINE_ARM64
}

/// The name of the graphics card that drives the main screen.
fn gpu_name() -> Option<String> {
	let mut i = 0;
	loop {
		let mut d: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
		d.cb = size_of::<DISPLAY_DEVICEW>() as u32;
		if unsafe { EnumDisplayDevicesW(std::ptr::null(), i, &mut d, 0) } == 0 {
			return None;
		}
		if d.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0 {
			let n = d
				.DeviceString
				.iter()
				.position(|&c| c == 0)
				.unwrap_or(d.DeviceString.len());
			let name = String::from_utf16_lossy(&d.DeviceString[..n]).trim().to_string();
			return (!name.is_empty()).then_some(name);
		}
		i += 1;
	}
}

fn refresh_hz() -> u32 {
	let mut m: DEVMODEW = unsafe { std::mem::zeroed() };
	m.dmSize = size_of::<DEVMODEW>() as u16;
	if unsafe { EnumDisplaySettingsW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut m) } == 0 {
		return 0;
	}
	m.dmDisplayFrequency
}

/// Whether the drive under the temp folder seeks like a hard drive. Asked of the volume itself,
/// which needs no rights beyond opening it for nothing.
fn temp_drive_is_ssd() -> Option<bool> {
	let temp = std::env::temp_dir();
	let drive = temp.to_str()?.get(..2)?;
	if !drive.ends_with(':') {
		return None;
	}
	let path: Vec<u16> = std::ffi::OsStr::new(&format!(r"\\.\{drive}"))
		.encode_wide()
		.chain(Some(0))
		.collect();
	unsafe {
		let h = CreateFileW(
			path.as_ptr(),
			0,
			FILE_SHARE_READ | FILE_SHARE_WRITE,
			std::ptr::null(),
			OPEN_EXISTING,
			0,
			std::ptr::null_mut(),
		);
		if h == INVALID_HANDLE_VALUE {
			return None;
		}
		let query = STORAGE_PROPERTY_QUERY {
			PropertyId: StorageDeviceSeekPenaltyProperty,
			QueryType: PropertyStandardQuery,
			AdditionalParameters: [0],
		};
		let mut out: DEVICE_SEEK_PENALTY_DESCRIPTOR = std::mem::zeroed();
		let mut got = 0;
		let ok = DeviceIoControl(
			h,
			IOCTL_STORAGE_QUERY_PROPERTY,
			&query as *const _ as *const _,
			size_of::<STORAGE_PROPERTY_QUERY>() as u32,
			&mut out as *mut _ as *mut _,
			size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
			&mut got,
			std::ptr::null_mut(),
		) != 0;
		CloseHandle(h);
		ok.then_some(!out.IncursSeekPenalty)
	}
}

/// Every screen as "1920x1080 at 150%", main one first, for a problem report.
pub fn displays() -> String {
	use windows_sys::Win32::Foundation::{LPARAM, RECT};
	use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};
	use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
	unsafe extern "system" fn each(mon: HMONITOR, _: HDC, r: *mut RECT, data: LPARAM) -> windows_sys::core::BOOL {
		let list = unsafe { &mut *(data as *mut Vec<(i32, i32, i32, u32)>) };
		let r = unsafe { *r };
		let (mut dx, mut dy) = (96u32, 96u32);
		unsafe { GetDpiForMonitor(mon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
		list.push((
			r.left.abs() + r.top.abs(),
			r.right - r.left,
			r.bottom - r.top,
			dx * 100 / 96,
		));
		1
	}
	let mut list: Vec<(i32, i32, i32, u32)> = Vec::new();
	unsafe {
		EnumDisplayMonitors(
			std::ptr::null_mut(),
			std::ptr::null(),
			Some(each),
			&mut list as *mut _ as LPARAM,
		)
	};
	// the main screen sits at 0,0
	list.sort_by_key(|m| m.0);
	let all: Vec<String> = list.iter().map(|(_, w, h, pct)| format!("{w}x{h} at {pct}%")).collect();
	if all.is_empty() {
		"not known".into()
	} else {
		all.join(", ")
	}
}
