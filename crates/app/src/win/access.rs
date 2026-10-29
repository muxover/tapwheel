//! Screen readers: the wheel and the cards are drawn by TapWheel itself, so they tell UI
//! Automation what they are and announce what changes (the wheel opening, the slice under the
//! pointer or the keyboard, a job finishing). Narrator, NVDA and JAWS read the announcements.

use std::cell::RefCell;
use std::collections::HashMap;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
	IRawElementProviderSimple, IRawElementProviderSimple_Impl, NotificationKind_ActionCompleted,
	NotificationKind_Other, NotificationProcessing_ImportantMostRecent, NotificationProcessing_MostRecent,
	ProviderOptions, ProviderOptions_ServerSideProvider, ProviderOptions_UseComThreading, UIA_CONTROLTYPE_ID,
	UIA_ControlTypePropertyId, UIA_IsKeyboardFocusablePropertyId, UIA_MenuControlTypeId, UIA_NamePropertyId,
	UIA_PATTERN_ID, UIA_PROPERTY_ID, UIA_StatusBarControlTypeId, UiaClientsAreListening, UiaHostProviderFromHwnd,
	UiaRaiseNotificationEvent, UiaReturnRawElementProvider,
};
use windows::core::{AsImpl, BSTR, IUnknown, Result, implement};

/// What a window stands for, for screen readers.
#[derive(Clone, Copy)]
pub enum Role {
	Wheel,
	Cards,
}

#[implement(IRawElementProviderSimple)]
struct Element {
	hwnd: HWND,
	role: Role,
	name: RefCell<String>,
}

impl IRawElementProviderSimple_Impl for Element_Impl {
	fn ProviderOptions(&self) -> Result<ProviderOptions> {
		Ok(ProviderOptions_ServerSideProvider | ProviderOptions_UseComThreading)
	}

	fn GetPatternProvider(&self, _id: UIA_PATTERN_ID) -> Result<IUnknown> {
		Err(windows::Win32::Foundation::E_NOTIMPL.into())
	}

	fn GetPropertyValue(&self, id: UIA_PROPERTY_ID) -> Result<VARIANT> {
		Ok(if id == UIA_NamePropertyId {
			VARIANT::from(BSTR::from(self.name.borrow().as_str()))
		} else if id == UIA_ControlTypePropertyId {
			VARIANT::from(control_type(self.role).0)
		} else if id == UIA_IsKeyboardFocusablePropertyId {
			VARIANT::from(matches!(self.role, Role::Wheel))
		} else {
			VARIANT::default()
		})
	}

	fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
		unsafe { UiaHostProviderFromHwnd(self.hwnd) }
	}
}

fn control_type(role: Role) -> UIA_CONTROLTYPE_ID {
	match role {
		Role::Wheel => UIA_MenuControlTypeId,
		Role::Cards => UIA_StatusBarControlTypeId,
	}
}

thread_local! {
	// one element per window, made on the UI thread and answered there
	static ELEMENTS: RefCell<HashMap<isize, IRawElementProviderSimple>> = RefCell::new(HashMap::new());
}

fn element(hwnd: windows_sys::Win32::Foundation::HWND, role: Role) -> IRawElementProviderSimple {
	ELEMENTS.with_borrow_mut(|all| {
		all.entry(hwnd as isize)
			.or_insert_with(|| {
				Element {
					hwnd: HWND(hwnd),
					role,
					name: RefCell::new(match role {
						Role::Wheel => "TapWheel".into(),
						Role::Cards => "TapWheel jobs".into(),
					}),
				}
				.into()
			})
			.clone()
	})
}

/// Answers WM_GETOBJECT for one of TapWheel's own windows. `None` lets Windows answer.
pub fn get_object(hwnd: windows_sys::Win32::Foundation::HWND, role: Role, w: usize, l: isize) -> Option<isize> {
	// UiaRootObjectId
	if l as i32 != -25 {
		return None;
	}
	let e = element(hwnd, role);
	let r = unsafe {
		UiaReturnRawElementProvider(
			HWND(hwnd),
			windows::Win32::Foundation::WPARAM(w),
			windows::Win32::Foundation::LPARAM(l),
			&e,
		)
	};
	Some(r.0)
}

/// Says `text` through whatever screen reader is running, and keeps it as the window's name so a
/// reader that asks later hears the same. Nothing happens when no reader is listening.
pub fn announce(hwnd: windows_sys::Win32::Foundation::HWND, role: Role, text: &str, done: bool) {
	if !unsafe { UiaClientsAreListening() }.as_bool() {
		return;
	}
	let e = element(hwnd, role);
	// the element was made from `Element` just above, so it can be read back as one
	let el: &Element = unsafe { e.as_impl() };
	*el.name.borrow_mut() = text.to_string();
	let (kind, processing) = if done {
		(
			NotificationKind_ActionCompleted,
			NotificationProcessing_ImportantMostRecent,
		)
	} else {
		(NotificationKind_Other, NotificationProcessing_MostRecent)
	};
	let result = unsafe { UiaRaiseNotificationEvent(&e, kind, processing, &BSTR::from(text), &BSTR::from("TapWheel")) };
	if let Err(err) = result {
		super::log::write(&format!("screen reader announcement failed: {err}"));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use windows::Win32::System::Com::{
		CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
	};
	use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
	use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
	use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
	use windows_sys::Win32::UI::WindowsAndMessaging::{
		CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, MSG, PostMessageW,
		PostQuitMessage, RegisterClassW, TranslateMessage, WM_APP, WM_CLOSE, WM_GETOBJECT, WNDCLASSW, WS_POPUP,
	};

	const SAY: u32 = WM_APP + 1;

	unsafe extern "system" fn proc(
		hwnd: windows_sys::Win32::Foundation::HWND,
		msg: u32,
		w: WPARAM,
		l: LPARAM,
	) -> LRESULT {
		match msg {
			WM_GETOBJECT => {
				if let Some(r) = get_object(hwnd, Role::Wheel, w, l) {
					return r;
				}
			}
			SAY => {
				announce(hwnd, Role::Wheel, "Convert wheel for photo.jpg. 9 choices.", false);
				return 0;
			}
			WM_CLOSE => {
				unsafe {
					DestroyWindow(hwnd);
					PostQuitMessage(0);
				}
				return 0;
			}
			_ => {}
		}
		unsafe { DefWindowProcW(hwnd, msg, w, l) }
	}

	/// The wheel's window runs on its own thread with a message loop, as in the app, since a
	/// screen reader asks it questions from outside its thread.
	#[test]
	fn a_screen_reader_finds_the_wheel_and_hears_what_changes() {
		let (tx, rx) = std::sync::mpsc::channel::<isize>();
		let ui = std::thread::spawn(move || unsafe {
			let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
			let class: Vec<u16> = "TapWheel.TestWheel\0".encode_utf16().collect();
			let wc = WNDCLASSW {
				lpfnWndProc: Some(proc),
				hInstance: GetModuleHandleW(std::ptr::null()),
				lpszClassName: class.as_ptr(),
				..std::mem::zeroed()
			};
			RegisterClassW(&wc);
			let hwnd = CreateWindowExW(
				0,
				class.as_ptr(),
				std::ptr::null(),
				WS_POPUP,
				0,
				0,
				100,
				100,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
				wc.hInstance,
				std::ptr::null(),
			);
			tx.send(hwnd as isize).unwrap();
			let mut msg: MSG = std::mem::zeroed();
			while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			}
		});
		let hwnd = rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap() as windows_sys::Win32::Foundation::HWND;
		assert!(!hwnd.is_null());
		unsafe {
			let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
			let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).unwrap();
			let e = uia.ElementFromHandle(HWND(hwnd)).unwrap();
			assert_eq!(e.CurrentName().unwrap().to_string(), "TapWheel");
			assert_eq!(e.CurrentControlType().unwrap(), UIA_MenuControlTypeId);
			// a client is listening now, so an announcement also renames the element
			PostMessageW(hwnd, SAY, 0, 0);
			let mut name = String::new();
			for _ in 0..50 {
				std::thread::sleep(std::time::Duration::from_millis(50));
				name = uia
					.ElementFromHandle(HWND(hwnd))
					.unwrap()
					.CurrentName()
					.unwrap()
					.to_string();
				if name != "TapWheel" {
					break;
				}
			}
			assert_eq!(name, "Convert wheel for photo.jpg. 9 choices.");
			PostMessageW(hwnd, WM_CLOSE, 0, 0);
		}
		ui.join().unwrap();
	}
}
