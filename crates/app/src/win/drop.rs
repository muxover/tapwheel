use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{POINTL, S_OK};
use windows_sys::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED_HGLOBAL};
use windows_sys::Win32::System::Ole::{CF_HDROP, DROPEFFECT_COPY, DROPEFFECT_NONE, ReleaseStgMedium};
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows_sys::core::{GUID, HRESULT};

use super::util::from_wide;

const E_NOINTERFACE: HRESULT = 0x8000_4002u32 as i32;
const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_c000_000000000046);
const IID_IDROPTARGET: GUID = GUID::from_u128(0x00000122_0000_0000_c000_000000000046);

/// What the wheel decides for a point: whether a drop there does anything.
pub trait Target {
	fn enter(&self, files: Option<Vec<PathBuf>>, x: i32, y: i32) -> bool;
	fn over(&self, x: i32, y: i32) -> bool;
	fn leave(&self);
	fn dropped(&self, files: Option<Vec<PathBuf>>, x: i32, y: i32) -> bool;
}

#[repr(C)]
struct Vtbl {
	query_interface: unsafe extern "system" fn(*mut DropTarget, *const GUID, *mut *mut c_void) -> HRESULT,
	add_ref: unsafe extern "system" fn(*mut DropTarget) -> u32,
	release: unsafe extern "system" fn(*mut DropTarget) -> u32,
	drag_enter: unsafe extern "system" fn(*mut DropTarget, *mut c_void, u32, POINTL, *mut u32) -> HRESULT,
	drag_over: unsafe extern "system" fn(*mut DropTarget, u32, POINTL, *mut u32) -> HRESULT,
	drag_leave: unsafe extern "system" fn(*mut DropTarget) -> HRESULT,
	on_drop: unsafe extern "system" fn(*mut DropTarget, *mut c_void, u32, POINTL, *mut u32) -> HRESULT,
}

#[repr(C)]
pub struct DropTarget {
	vtbl: *const Vtbl,
	refs: AtomicU32,
	target: Box<dyn Target>,
}

static VTBL: Vtbl = Vtbl {
	query_interface,
	add_ref,
	release,
	drag_enter,
	drag_over,
	drag_leave,
	on_drop,
};

impl DropTarget {
	/// A COM object with one reference, owned by the caller until `RevokeDragDrop`.
	pub fn create(target: Box<dyn Target>) -> *mut c_void {
		Box::into_raw(Box::new(DropTarget {
			vtbl: &VTBL,
			refs: AtomicU32::new(1),
			target,
		})) as *mut c_void
	}
}

unsafe extern "system" fn query_interface(this: *mut DropTarget, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
	unsafe {
		let iid = &*iid;
		let same =
			|a: &GUID, b: &GUID| a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4;
		if same(iid, &IID_IUNKNOWN) || same(iid, &IID_IDROPTARGET) {
			add_ref(this);
			*out = this as *mut c_void;
			S_OK
		} else {
			*out = std::ptr::null_mut();
			E_NOINTERFACE
		}
	}
}

unsafe extern "system" fn add_ref(this: *mut DropTarget) -> u32 {
	unsafe { (*this).refs.fetch_add(1, Ordering::AcqRel) + 1 }
}

unsafe extern "system" fn release(this: *mut DropTarget) -> u32 {
	unsafe {
		let left = (*this).refs.fetch_sub(1, Ordering::AcqRel) - 1;
		if left == 0 {
			drop(Box::from_raw(this));
		}
		left
	}
}

/// `IDataObject::GetData` is slot 3 of its vtable.
type GetData = unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT;

/// The dragged paths, or `None` when the drag carries no files (text, links, virtual items).
fn files_of(data: *mut c_void) -> Option<Vec<PathBuf>> {
	if data.is_null() {
		return None;
	}
	unsafe {
		let vtbl = *(data as *const *const usize);
		let get: GetData = std::mem::transmute(*vtbl.add(3));
		let fmt = FORMATETC {
			cfFormat: CF_HDROP,
			ptd: std::ptr::null_mut(),
			dwAspect: DVASPECT_CONTENT,
			lindex: -1,
			tymed: TYMED_HGLOBAL as u32,
		};
		let mut medium: STGMEDIUM = std::mem::zeroed();
		if get(data, &fmt, &mut medium) != S_OK {
			return None;
		}
		let hdrop = medium.u.hGlobal as HDROP;
		let count = DragQueryFileW(hdrop, u32::MAX, std::ptr::null_mut(), 0);
		let mut files = Vec::with_capacity(count as usize);
		for i in 0..count {
			let len = DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0);
			let mut buf = vec![0u16; len as usize + 1];
			DragQueryFileW(hdrop, i, buf.as_mut_ptr(), buf.len() as u32);
			files.push(PathBuf::from(from_wide(&buf)));
		}
		ReleaseStgMedium(&mut medium);
		(!files.is_empty()).then_some(files)
	}
}

// Always COPY or NONE, never MOVE: holding Shift means "move" in Explorer, and a MOVE
// answer would make the source delete the original after the drop.
fn effect(ok: bool) -> u32 {
	if ok { DROPEFFECT_COPY } else { DROPEFFECT_NONE }
}

unsafe extern "system" fn drag_enter(
	this: *mut DropTarget,
	data: *mut c_void,
	_keys: u32,
	pt: POINTL,
	eff: *mut u32,
) -> HRESULT {
	unsafe {
		let ok = (*this).target.enter(files_of(data), pt.x, pt.y);
		*eff = effect(ok);
	}
	S_OK
}

unsafe extern "system" fn drag_over(this: *mut DropTarget, _keys: u32, pt: POINTL, eff: *mut u32) -> HRESULT {
	unsafe {
		let ok = (*this).target.over(pt.x, pt.y);
		*eff = effect(ok);
	}
	S_OK
}

unsafe extern "system" fn drag_leave(this: *mut DropTarget) -> HRESULT {
	unsafe { (*this).target.leave() };
	S_OK
}

unsafe extern "system" fn on_drop(
	this: *mut DropTarget,
	data: *mut c_void,
	_keys: u32,
	pt: POINTL,
	eff: *mut u32,
) -> HRESULT {
	unsafe {
		let ok = (*this).target.dropped(files_of(data), pt.x, pt.y);
		*eff = effect(ok);
	}
	S_OK
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn never_move() {
		assert_eq!(effect(true), DROPEFFECT_COPY);
		assert_eq!(effect(false), DROPEFFECT_NONE);
	}
}
