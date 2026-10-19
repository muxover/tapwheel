//! What gets written when the app dies in a way the panic hook never sees: a fault such as an
//! access violation, or memory running out. Both write one line to the log; a fault also leaves a
//! minidump in `%LOCALAPPDATA%\TapWheel\crash`. Nothing here may allocate, because the heap may be
//! the thing that failed.

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_WRITE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
	CREATE_ALWAYS, CreateFileW, FILE_APPEND_DATA, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, OPEN_ALWAYS, WriteFile,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
	EXCEPTION_POINTERS, MINIDUMP_EXCEPTION_INFORMATION, MiniDumpWithThreadInfo, MiniDumpWriteDump,
	SetUnhandledExceptionFilter,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId};

/// NUL-terminated wide paths, made at start so a crash has nothing to build.
struct Paths {
	log: Vec<u16>,
	dump: Vec<u16>,
}

static PATHS: OnceLock<Paths> = OnceLock::new();

fn wide(p: &Path) -> Vec<u16> {
	p.as_os_str().encode_wide().chain([0]).collect()
}

fn set_paths(log: &Path, dump: &Path) {
	let _ = PATHS.set(Paths {
		log: wide(log),
		dump: wide(dump),
	});
}

pub fn install() {
	let dir = super::shell::appdata("LOCALAPPDATA").join("crash");
	let _ = std::fs::create_dir_all(&dir);
	set_paths(
		&super::log::path(),
		&dir.join(format!("tapwheel-{}.dmp", std::process::id())),
	);
	unsafe { SetUnhandledExceptionFilter(Some(on_fault)) };
}

/// A log line built on the stack.
struct Line {
	buf: [u8; 192],
	len: usize,
}

impl Write for Line {
	fn write_str(&mut self, s: &str) -> std::fmt::Result {
		let n = s.len().min(self.buf.len() - self.len);
		self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
		self.len += n;
		Ok(())
	}
}

fn line(what: std::fmt::Arguments) -> Line {
	let secs = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|d| d.as_secs())
		.unwrap_or(0);
	let mut l = Line { buf: [0; 192], len: 0 };
	let _ = writeln!(l, "{secs} v{} {what}", env!("CARGO_PKG_VERSION"));
	l
}

fn append(l: &Line) {
	let Some(p) = PATHS.get() else { return };
	unsafe {
		let h = CreateFileW(
			p.log.as_ptr(),
			FILE_APPEND_DATA,
			FILE_SHARE_READ,
			std::ptr::null(),
			OPEN_ALWAYS,
			FILE_ATTRIBUTE_NORMAL,
			std::ptr::null_mut(),
		);
		if h == INVALID_HANDLE_VALUE {
			return;
		}
		let mut written = 0;
		WriteFile(h, l.buf.as_ptr(), l.len as u32, &mut written, std::ptr::null_mut());
		CloseHandle(h);
	}
}

fn dump(info: *const EXCEPTION_POINTERS) -> bool {
	let Some(p) = PATHS.get() else { return false };
	unsafe {
		let h = CreateFileW(
			p.dump.as_ptr(),
			GENERIC_WRITE,
			0,
			std::ptr::null(),
			CREATE_ALWAYS,
			FILE_ATTRIBUTE_NORMAL,
			std::ptr::null_mut(),
		);
		if h == INVALID_HANDLE_VALUE {
			return false;
		}
		let ex = MINIDUMP_EXCEPTION_INFORMATION {
			ThreadId: GetCurrentThreadId(),
			ExceptionPointers: info as *mut _,
			ClientPointers: 0,
		};
		let ok = MiniDumpWriteDump(
			GetCurrentProcess(),
			GetCurrentProcessId(),
			h,
			MiniDumpWithThreadInfo,
			if info.is_null() { std::ptr::null() } else { &ex },
			std::ptr::null(),
			std::ptr::null(),
		);
		CloseHandle(h);
		ok != 0
	}
}

/// What the exception codes seen in practice mean, for the log line.
fn meaning(code: u32) -> &'static str {
	match code {
		0xc000_0005 => " (access violation)",
		0xc000_0017 => " (out of memory)",
		0xc000_001d => " (illegal instruction)",
		0xc000_0094 => " (integer divided by zero)",
		0xc000_00fd => " (stack overflow)",
		0xc000_0374 => " (heap corruption)",
		0xc000_0409 => " (fail fast)",
		// Chromium's own out-of-memory code, raised inside the app by WebView2
		0xe000_0008 => " (out of memory in WebView2)",
		0xe06d_7363 => " (C++ exception)",
		0x8000_0003 => " (breakpoint)",
		_ => "",
	}
}

unsafe extern "system" fn on_fault(info: *const EXCEPTION_POINTERS) -> i32 {
	let (code, at) = unsafe { info.as_ref().and_then(|i| i.ExceptionRecord.as_ref()) }
		.map_or((0, 0), |r| (r.ExceptionCode as u32, r.ExceptionAddress as usize));
	let dumped = dump(info);
	append(&line(format_args!(
		"crash: exception {code:#010x}{} at {at:#x}{}",
		meaning(code),
		if dumped { ", minidump in the crash folder" } else { "" }
	)));
	// keep searching, so Windows still reports the crash the usual way
	0
}

fn out_of_memory(size: usize) {
	append(&line(format_args!("out of memory: Windows couldn't give {size} bytes")));
}

/// The system allocator, plus a log line when it fails. Rust ends the process on a failed
/// allocation without running the panic hook, so this is the only trace such a crash leaves.
pub struct Logged;

unsafe impl GlobalAlloc for Logged {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let p = unsafe { System.alloc(layout) };
		if p.is_null() {
			out_of_memory(layout.size());
		}
		p
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		let p = unsafe { System.alloc_zeroed(layout) };
		if p.is_null() {
			out_of_memory(layout.size());
		}
		p
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		unsafe { System.dealloc(ptr, layout) }
	}

	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		let p = unsafe { System.realloc(ptr, layout, new_size) };
		if p.is_null() {
			out_of_memory(new_size);
		}
		p
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use windows_sys::Win32::System::Diagnostics::Debug::{CONTEXT, EXCEPTION_RECORD};

	#[test]
	fn a_fault_and_a_failed_allocation_leave_a_trace() {
		let dir = std::env::temp_dir().join(format!("tw-crash-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let (log, dmp) = (dir.join("tapwheel.log"), dir.join("tapwheel.dmp"));
		set_paths(&log, &dmp);
		assert_eq!(unsafe { on_fault(std::ptr::null()) }, 0);
		let mut record = EXCEPTION_RECORD {
			ExceptionCode: 0xe000_0008_u32 as i32,
			..Default::default()
		};
		let mut context = CONTEXT::default();
		let pointers = EXCEPTION_POINTERS {
			ExceptionRecord: &mut record,
			ContextRecord: &mut context,
		};
		assert_eq!(unsafe { on_fault(&pointers) }, 0);
		let huge = Layout::from_size_align(1 << 60, 8).unwrap();
		assert!(unsafe { Logged.alloc(huge) }.is_null());
		let text = std::fs::read_to_string(&log).unwrap();
		assert!(text.contains("crash: exception 0x00000000 at"), "{text}");
		assert!(
			text.contains("crash: exception 0xe0000008 (out of memory in WebView2) at 0x0"),
			"{text}"
		);
		assert!(
			text.contains(&format!("out of memory: Windows couldn't give {} bytes", 1usize << 60)),
			"{text}"
		);
		assert!(std::fs::metadata(&dmp).is_ok_and(|m| m.len() > 0));
		let _ = std::fs::remove_dir_all(dir);
	}
}
