//! Float: a small shelf that keeps files at hand. Drop files on Float to park them, drag them
//! out later into any folder or app, and drop more onto the shelf itself.

use std::cell::RefCell;
use std::path::PathBuf;

use tapwheel_core::Kind;
use tapwheel_core::kind::kind_of;
use tapwheel_ui::draw::*;
use tapwheel_ui::{Fonts, Theme, Weight};
use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Ole::{DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, RegisterDragDrop};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::drop::{DropTarget, Target};
use super::util::*;

const CLASS: &str = "TapWheel.Float";
const W: f32 = 272.0;
const HEAD: f32 = 44.0;
const ROW: f32 = 46.0;
const FOOT: f32 = 34.0;
const MARGIN: f32 = 14.0;
const MAX: usize = 8;

struct Shelf {
	hwnd: HWND,
	items: Vec<PathBuf>,
	theme: Theme,
	fonts: Fonts,
	scale: f32,
	x: i32,
	y: i32,
	hover: Hit,
	press: Option<(Hit, POINT)>,
	dropping: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hit {
	None,
	Close,
	Head,
	Row(usize),
}

thread_local! {
	static SHELF: RefCell<Option<Shelf>> = const { RefCell::new(None) };
	static INST: RefCell<HINSTANCE> = const { RefCell::new(std::ptr::null_mut()) };
}

fn with<R>(f: impl FnOnce(&mut Shelf) -> R) -> Option<R> {
	SHELF.with(|s| s.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

pub fn register_class(inst: HINSTANCE, icon: HICON) {
	INST.with(|i| *i.borrow_mut() = inst);
	super::app::register(inst, CLASS, Some(proc_), icon, true);
}

fn height(n: usize) -> f32 {
	HEAD + ROW * n.max(1) as f32 + FOOT
}

fn icon_for(p: &std::path::Path) -> &'static str {
	match kind_of(p) {
		Kind::Image => "image",
		Kind::Video => "video",
		Kind::Audio => "music_note_2",
		Kind::Icon | Kind::Program => "app_generic",
		Kind::Text => "document_text",
		Kind::Code => "code",
		Kind::Archive => "archive",
		Kind::Folder => "folder",
		Kind::Other => "document",
	}
}

impl Shelf {
	fn size(&self) -> (u32, u32) {
		let s = self.scale;
		(
			((W + MARGIN * 2.0) * s).ceil() as u32,
			((height(self.items.len()) + MARGIN * 2.0) * s).ceil() as u32,
		)
	}

	fn hit(&self, x: f32, y: f32) -> Hit {
		let (x, y) = (x - MARGIN, y - MARGIN);
		if x < 0.0 || x > W || y < 0.0 || y > height(self.items.len()) {
			return Hit::None;
		}
		if y < HEAD {
			return if x > W - 40.0 { Hit::Close } else { Hit::Head };
		}
		let i = ((y - HEAD) / ROW) as usize;
		if i < self.items.len() { Hit::Row(i) } else { Hit::None }
	}

	fn render(&self) -> Pixmap {
		let (w, h) = self.size();
		let mut pm = Pixmap::new(w, h).expect("size");
		let s = self.scale;
		let p = self.theme.palette();
		let r = self.theme.radius() * s * 1.25;
		let (x, y) = (MARGIN * s, MARGIN * s);
		let (cw, ch) = (W * s, height(self.items.len()) * s);
		if !self.theme.high_contrast {
			shadow_rrect(&mut pm, x, y, cw, ch, r, 14.0 * s, p.shadow);
		}
		let bg = if self.dropping {
			p.surface.mix(p.accent, 0.12)
		} else {
			p.surface
		};
		fill_rrect(&mut pm, x, y, cw, ch, r, bg);
		stroke_rrect(
			&mut pm,
			x,
			y,
			cw,
			ch,
			r,
			s.max(1.0),
			if self.dropping { p.accent } else { p.stroke.alpha(1.8) },
		);
		icon(&mut pm, "pin", x + 14.0 * s, y + 12.0 * s, 20.0 * s, p.accent);
		let fs = &self.fonts;
		fs.draw(
			&mut pm,
			"Float",
			x + 42.0 * s,
			y + 13.0 * s,
			14.0 * s,
			Weight::Semibold,
			p.text,
			None,
		);
		let count = match self.items.len() {
			0 => String::new(),
			1 => "1 item".into(),
			n => format!("{n} items"),
		};
		fs.draw(
			&mut pm,
			&count,
			x + 90.0 * s,
			y + 15.0 * s,
			12.0 * s,
			Weight::Regular,
			p.text2,
			None,
		);
		if self.hover == Hit::Close {
			fill_rrect(
				&mut pm,
				x + cw - 38.0 * s,
				y + 8.0 * s,
				28.0 * s,
				28.0 * s,
				4.0 * s,
				p.text.alpha(0.08),
			);
		}
		icon(&mut pm, "dismiss", x + cw - 32.0 * s, y + 14.0 * s, 16.0 * s, p.text2);
		for (i, item) in self.items.iter().enumerate() {
			let top = y + (HEAD + ROW * i as f32) * s;
			if self.hover == Hit::Row(i) {
				fill_rrect(
					&mut pm,
					x + 6.0 * s,
					top + 2.0 * s,
					cw - 12.0 * s,
					(ROW - 4.0) * s,
					6.0 * s,
					p.text.alpha(0.06),
				);
			}
			icon(&mut pm, icon_for(item), x + 16.0 * s, top + 13.0 * s, 20.0 * s, p.text2);
			let name = item
				.file_name()
				.map(|n| n.to_string_lossy().into_owned())
				.unwrap_or_default();
			fs.draw(
				&mut pm,
				&name,
				x + 46.0 * s,
				top + 7.0 * s,
				13.0 * s,
				Weight::Semibold,
				p.text,
				Some(cw - 60.0 * s),
			);
			let where_ = item
				.parent()
				.and_then(|d| d.file_name())
				.map(|n| n.to_string_lossy().into_owned())
				.unwrap_or_default();
			fs.draw(
				&mut pm,
				&where_,
				x + 46.0 * s,
				top + 25.0 * s,
				11.5 * s,
				Weight::Regular,
				p.text2,
				Some(cw - 60.0 * s),
			);
		}
		let hint = if self.items.is_empty() {
			"Drop files here to keep them"
		} else {
			"Drag out to use, or drop more in"
		};
		let foot = y + ch - FOOT * s + 9.0 * s;
		fs.draw_centered(
			&mut pm,
			hint,
			x + cw / 2.0,
			foot,
			12.0 * s,
			Weight::Regular,
			p.text2,
			None,
		);
		pm
	}

	fn draw(&self) {
		push_layered(self.hwnd, &self.render(), self.x, self.y);
		let (w, h) = self.size();
		unsafe {
			SetWindowPos(
				self.hwnd,
				HWND_TOPMOST,
				self.x,
				self.y,
				w as i32,
				h as i32,
				SWP_NOACTIVATE,
			)
		};
	}

	fn add(&mut self, files: &[PathBuf]) {
		for f in files {
			self.items.retain(|i| i != f);
			self.items.insert(0, f.clone());
		}
		self.items.truncate(MAX);
		self.draw();
	}
}

/// Adds files to the shelf, opening it near the pointer the first time.
pub fn add(theme: Theme, files: &[PathBuf]) {
	if with(|s| {
		s.theme = theme;
		s.add(files);
	})
	.is_some()
	{
		return;
	}
	let inst = INST.with(|i| *i.borrow());
	let hwnd = unsafe {
		CreateWindowExW(
			WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
			wide(CLASS).as_ptr(),
			wide("Float").as_ptr(),
			WS_POPUP,
			0,
			0,
			1,
			1,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			inst,
			std::ptr::null(),
		)
	};
	if hwnd.is_null() {
		return;
	}
	unsafe { RegisterDragDrop(hwnd, DropTarget::create(Box::new(ShelfTarget))) };
	let pt = cursor();
	let (work, scale) = monitor_at(pt);
	let w = ((W + MARGIN * 2.0) * scale) as i32;
	let mut shelf = Shelf {
		hwnd,
		items: Vec::new(),
		theme,
		fonts: super::system::fonts(),
		scale,
		x: (work.right - w - (24.0 * scale) as i32).max(work.left),
		y: work.top + (96.0 * scale) as i32,
		hover: Hit::None,
		press: None,
		dropping: false,
	};
	shelf.add(files);
	SHELF.with(|s| *s.borrow_mut() = Some(shelf));
	unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
}

fn close() {
	let hwnd = with(|s| s.hwnd);
	SHELF.with(|s| *s.borrow_mut() = None);
	if let Some(h) = hwnd {
		unsafe {
			windows_sys::Win32::System::Ole::RevokeDragDrop(h);
			DestroyWindow(h);
		}
	}
}

/// Drags shelf items out with the shell's own drag, so any app or folder takes them.
fn drag_out(hwnd: HWND, items: &[PathBuf]) -> u32 {
	unsafe {
		let pidls: Vec<_> = items
			.iter()
			.filter(|p| p.exists())
			.map(|p| ILCreateFromPathW(wide(p.as_os_str()).as_ptr()))
			.filter(|p| !p.is_null())
			.collect();
		if pidls.is_empty() {
			return 0;
		}
		let ptrs: Vec<*const _> = pidls.iter().map(|p| *p as *const _).collect();
		let mut data: *mut std::ffi::c_void = std::ptr::null_mut();
		const IID_IDATAOBJECT: windows_sys::core::GUID =
			windows_sys::core::GUID::from_u128(0x0000010e_0000_0000_c000_000000000046);
		let hr = SHCreateDataObject(
			std::ptr::null(),
			ptrs.len() as u32,
			ptrs.as_ptr(),
			std::ptr::null_mut(),
			&IID_IDATAOBJECT,
			&mut data,
		);
		let mut effect = 0u32;
		if hr >= 0 && !data.is_null() {
			SHDoDragDrop(
				hwnd,
				data,
				std::ptr::null_mut(),
				DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK,
				&mut effect,
			);
			// IDataObject::Release
			let vtbl = *(data as *const *const usize);
			let release: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 = std::mem::transmute(*vtbl.add(2));
			release(data);
		}
		for p in pidls {
			ILFree(p);
		}
		effect
	}
}

struct ShelfTarget;

impl Target for ShelfTarget {
	fn enter(&self, files: Option<Vec<PathBuf>>, _x: i32, _y: i32) -> bool {
		let ok = files.is_some();
		with(|s| {
			s.dropping = ok;
			s.draw();
		});
		ok
	}

	fn over(&self, _x: i32, _y: i32) -> bool {
		with(|s| s.dropping).unwrap_or(false)
	}

	fn leave(&self) {
		with(|s| {
			s.dropping = false;
			s.draw();
		});
	}

	fn dropped(&self, files: Option<Vec<PathBuf>>, _x: i32, _y: i32) -> bool {
		with(|s| {
			s.dropping = false;
			match files {
				Some(f) => {
					s.add(&f);
					true
				}
				None => {
					s.draw();
					false
				}
			}
		})
		.unwrap_or(false)
	}
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	let handled: Option<()> = match msg {
		WM_MOUSEACTIVATE => return MA_NOACTIVATE as isize,
		WM_MOUSEMOVE => {
			let (x, y) = (get_x(l), get_y(l));
			let start = with(|s| {
				let hit = s.hit(x as f32 / s.scale, y as f32 / s.scale);
				if hit != s.hover {
					s.hover = hit;
					s.draw();
				}
				let mut tme = TRACKMOUSEEVENT {
					cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
					dwFlags: 0x0000_0002,
					hwndTrack: hwnd,
					dwHoverTime: 0,
				};
				unsafe { TrackMouseEvent(&mut tme) };
				match s.press {
					Some((Hit::Row(i), at)) if (x - at.x).abs() + (y - at.y).abs() > 6 => {
						s.press = None;
						// with Shift held, every item goes along
						let all = unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
						Some(if all { s.items.clone() } else { vec![s.items[i].clone()] })
					}
					Some((Hit::Head, at)) => {
						let pt = cursor();
						s.x = pt.x - at.x;
						s.y = pt.y - at.y;
						s.draw();
						None
					}
					_ => None,
				}
			})
			.flatten();
			if let Some(items) = start {
				unsafe { ReleaseCapture() };
				let effect = drag_out(hwnd, &items);
				// moved away: the shelf lets go of them
				if effect & DROPEFFECT_MOVE != 0 {
					with(|s| {
						s.items.retain(|i| !items.contains(i));
						s.draw();
					});
				}
			}
			Some(())
		}
		WM_LBUTTONDOWN => with(|s| {
			let (x, y) = (get_x(l), get_y(l));
			let hit = s.hit(x as f32 / s.scale, y as f32 / s.scale);
			s.press = Some((hit, POINT { x, y }));
			unsafe { SetCapture(hwnd) };
		}),
		WM_LBUTTONUP => {
			let closing = with(|s| {
				unsafe { ReleaseCapture() };
				let was = s.press.take().map(|p| p.0);
				let hit = s.hit(get_x(l) as f32 / s.scale, get_y(l) as f32 / s.scale);
				if let (Some(Hit::Row(i)), Hit::Row(j)) = (was, hit)
					&& i == j
				{
					super::shell::reveal(std::slice::from_ref(&s.items[i]));
				}
				was == Some(Hit::Close) && hit == Hit::Close
			})
			.unwrap_or(false);
			if closing {
				close();
			}
			Some(())
		}
		0x02a3 => with(|s| {
			s.hover = Hit::None;
			s.draw();
		}),
		_ => None,
	};
	match handled {
		Some(()) => 0,
		None => unsafe { DefWindowProcW(hwnd, msg, w, l) },
	}
}
