use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Instant;

use tapwheel_core::kind::kind_of;
use tapwheel_core::ops::selection_kind;
use tapwheel_core::settings::Key;
use tapwheel_core::{Kind, Op, Settings, Wheel, choices, plan};
use tapwheel_ui::home::{self, Action, HomeView};
use tapwheel_ui::hud::{self, Button, HudView, State};
use tapwheel_ui::wheel::{self, Hover, Slice, WheelView};
use tapwheel_ui::{Fonts, Theme, icon};
use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Ole::{OleInitialize, RegisterDragDrop};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use super::drop::{DropTarget, Target};
use super::hooks::{self, Hooks};
use super::jobs::Jobs;
use super::util::*;
use super::{WM_APP_DRAG_END, WM_APP_ESCAPE, WM_APP_JOBS, WM_APP_TAP, WM_APP_TRAY, shell, system};

const MAIN_CLASS: &str = "TapWheel.Main";
const WHEEL_CLASS: &str = "TapWheel.Wheel";
const HUD_CLASS: &str = "TapWheel.Hud";
const HOME_CLASS: &str = "TapWheel.Home";

const WM_MOUSELEAVE: u32 = 0x02a3;

const T_APPEAR: usize = 1;
const T_NUDGE: usize = 2;
const T_HIDE: usize = 3;
const T_PICK: usize = 10;
const T_HUD: usize = 20;

const MENU_OPEN: usize = 1;
const MENU_PAUSE: usize = 2;
const MENU_QUIT: usize = 3;

const WEBSITE: &str = "https://tapwheel.muxover.com";
const PRIVACY: &str = "https://tapwheel.muxover.com/privacy";
const SOURCE: &str = "https://github.com/muxover/tapwheel";
const HEIC_SAMPLE: &[u8] = include_bytes!("../../../core/tests/fixtures/photo.heic");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
	Drag,
	Pick,
}

struct WheelState {
	visible: bool,
	mode: Mode,
	wheel: Wheel,
	files: Option<Vec<PathBuf>>,
	/// The drag reached the wheel, so `files == None` means it carries no files.
	data_seen: bool,
	ops: Vec<Op>,
	hover: Hover,
	again: bool,
	scale: f32,
	x: i32,
	y: i32,
	appear: f32,
	last_tap: Option<(usize, Instant)>,
}

struct HudState {
	visible: bool,
	hover: Option<(usize, Button)>,
	ids: Vec<u64>,
	scale: f32,
	anchor: POINT,
	start: Instant,
	tracking: bool,
}

struct HomeState {
	hwnd: HWND,
	hover: Option<Action>,
	focus: Option<Action>,
	scale: f32,
	frame: Option<Pixmap>,
	tracking: bool,
}

struct App {
	inst: HINSTANCE,
	main: HWND,
	wheel_hwnd: HWND,
	hud_hwnd: HWND,
	icon: HICON,
	_hooks: Option<Hooks>,
	settings: Settings,
	settings_path: PathBuf,
	theme: Theme,
	fonts: Fonts,
	logo: Pixmap,
	packaged: bool,
	jobs: Jobs,
	wheel: WheelState,
	hud: HudState,
	home: HomeState,
	last_op: Option<Op>,
	pending_pick: Vec<PathBuf>,
	taskbar_created: u32,
}

thread_local! {
	static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` on the app unless it's already borrowed further up this stack (a window
/// message sent from inside another handler); those messages fall back to the default.
fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
	APP.with(|a| a.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

fn key_index(k: Key) -> usize {
	match k {
		Key::Shift => 0,
		Key::Ctrl => 1,
		Key::Alt => 2,
		Key::Off => 3,
	}
}

fn key_from(i: usize) -> Key {
	match i {
		0 => Key::Shift,
		1 => Key::Ctrl,
		2 => Key::Alt,
		_ => Key::Off,
	}
}

fn wheel_index(w: Wheel) -> usize {
	match w {
		Wheel::Convert => 0,
		Wheel::Tools => 1,
		Wheel::Actions => 2,
	}
}

fn summary(files: &[PathBuf]) -> String {
	match files {
		[] => String::new(),
		[one] => one
			.file_name()
			.map(|n| n.to_string_lossy().into_owned())
			.unwrap_or_default(),
		many => {
			let noun = match selection_kind(many) {
				Some(Kind::Image) => "images",
				Some(Kind::Audio) => "sounds",
				Some(Kind::Video) => "videos",
				Some(Kind::Subtitle) => "subtitles",
				Some(Kind::Archive) => "archives",
				Some(Kind::Folder) => "folders",
				_ => "items",
			};
			format!("{} {noun}", many.len())
		}
	}
}

fn reduced_motion() -> bool {
	let mut on: windows_sys::core::BOOL = 1;
	unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut on as *mut _ as *mut _, 0) };
	on == 0
}

impl App {
	fn save(&self) {
		let _ = self.settings.save(&self.settings_path);
	}

	fn keys(&self) -> [usize; 3] {
		[
			key_index(self.settings.keys.convert),
			key_index(self.settings.keys.tools),
			key_index(self.settings.keys.actions),
		]
	}

	// wheel

	fn refresh_ops(&mut self) {
		let w = &mut self.wheel;
		w.ops = match &w.files {
			None => Vec::new(),
			Some(files) if w.again => match self.last_op {
				Some(op) if Wheel::ALL.iter().any(|wh| choices(*wh, files).contains(&op)) => {
					vec![op]
				}
				_ => Vec::new(),
			},
			Some(files) => {
				let mut ops = choices(w.wheel, files);
				self.settings.order(&mut ops);
				ops
			}
		};
	}

	fn wheel_view(&self) -> WheelView {
		let w = &self.wheel;
		let slices = w
			.ops
			.iter()
			.map(|op| Slice {
				label: if w.again {
					format!("Again: {}", op.label())
				} else {
					op.label().to_string()
				},
				desc: op.describe(),
			})
			.collect();
		let message = match (&w.files, w.data_seen) {
			(None, true) => Some("Only files and folders work here".to_string()),
			(Some(_), _) if w.ops.is_empty() && w.again => Some("Your last choice doesn't fit these files".to_string()),
			(Some(_), _) if w.ops.is_empty() => Some("Nothing here fits these files".to_string()),
			_ => None,
		};
		WheelView {
			slices,
			hover: w.hover,
			keyboard: w.mode == Mode::Pick,
			title: if w.again {
				"Again".into()
			} else {
				w.wheel.title().into()
			},
			subtitle: w.files.as_deref().map(summary).unwrap_or_default(),
			keys: home::key_hints(self.keys(), wheel_index(w.wheel)),
			message,
			appear: w.appear,
		}
	}

	fn draw_wheel(&self) {
		if !self.wheel.visible {
			return;
		}
		let pm = wheel::render(&self.wheel_view(), &self.theme, &self.fonts, self.wheel.scale);
		push_layered(self.wheel_hwnd, &pm, self.wheel.x, self.wheel.y);
	}

	fn show_wheel(&mut self, mode: Mode, wh: Wheel, files: Option<Vec<PathBuf>>) {
		let pt = cursor();
		let (work, scale) = monitor_at(pt);
		let (w, h) = wheel::size(scale);
		let x = (pt.x - (wheel::CX * scale) as i32).clamp(work.left, (work.right - w as i32).max(work.left));
		let y = (pt.y - (wheel::CY * scale) as i32).clamp(work.top, (work.bottom - h as i32).max(work.top));
		let animate = !reduced_motion();
		self.wheel = WheelState {
			visible: true,
			mode,
			wheel: wh,
			data_seen: files.is_some(),
			files,
			ops: Vec::new(),
			hover: Hover::None,
			again: false,
			scale,
			x,
			y,
			appear: if animate { 0.0 } else { 1.0 },
			last_tap: self.wheel.last_tap,
		};
		self.refresh_ops();
		if mode == Mode::Pick && !self.wheel.ops.is_empty() {
			self.wheel.hover = Hover::Slice(0);
		}
		hooks::WHEEL_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
		self.draw_wheel();
		unsafe {
			SetWindowPos(self.wheel_hwnd, HWND_TOPMOST, x, y, w as i32, h as i32, SWP_NOACTIVATE);
			if mode == Mode::Pick {
				ShowWindow(self.wheel_hwnd, SW_SHOW);
				SetForegroundWindow(self.wheel_hwnd);
			} else {
				ShowWindow(self.wheel_hwnd, SW_SHOWNOACTIVATE);
				SetTimer(self.wheel_hwnd, T_NUDGE, 16, None);
			}
			if animate {
				SetTimer(self.wheel_hwnd, T_APPEAR, 15, None);
			}
		}
	}

	fn hide_wheel(&mut self) {
		if !self.wheel.visible {
			return;
		}
		self.wheel.visible = false;
		self.wheel.files = None;
		self.wheel.ops.clear();
		hooks::WHEEL_OPEN.store(false, std::sync::atomic::Ordering::Relaxed);
		unsafe {
			KillTimer(self.wheel_hwnd, T_HIDE);
			KillTimer(self.wheel_hwnd, T_APPEAR);
			ShowWindow(self.wheel_hwnd, SW_HIDE);
		}
	}

	fn tap(&mut self, key: usize) {
		let Some(wh) = self.settings.keys.wheel_for(key_from(key)) else {
			return;
		};
		let now = Instant::now();
		let double = matches!(self.wheel.last_tap, Some((k, t)) if k == key && now.duration_since(t).as_millis() < 450);
		self.wheel.last_tap = Some((key, now));
		if !self.wheel.visible {
			if hooks::dragging() {
				self.show_wheel(Mode::Drag, wh, None);
			}
			return;
		}
		if double && self.last_op.is_some() && self.wheel.wheel == wh {
			self.wheel.again = !self.wheel.again;
		} else {
			self.wheel.again = false;
			self.wheel.wheel = wh;
		}
		self.refresh_ops();
		self.wheel.hover = if self.wheel.mode == Mode::Pick && !self.wheel.ops.is_empty() {
			Hover::Slice(0)
		} else {
			self.hover_at(cursor())
		};
		self.draw_wheel();
	}

	fn hover_at(&self, pt: POINT) -> Hover {
		let s = self.wheel.scale;
		let x = (pt.x - self.wheel.x) as f32 / s;
		let y = (pt.y - self.wheel.y) as f32 / s;
		wheel::hit(self.wheel.ops.len(), x, y)
	}

	fn set_hover(&mut self, h: Hover) {
		if self.wheel.hover != h {
			self.wheel.hover = h;
			self.draw_wheel();
		}
	}

	fn pick(&mut self, i: usize) {
		let (Some(op), Some(files)) = (self.wheel.ops.get(i).copied(), self.wheel.files.clone()) else {
			return;
		};
		self.hide_wheel();
		self.apply(op, files);
	}

	// work

	fn apply(&mut self, op: Op, files: Vec<PathBuf>) {
		self.settings.record(op);
		self.save();
		self.last_op = Some(op);
		self.hud.anchor = cursor();
		if op.is_action() {
			self.action(op, &files);
		} else {
			let jobs = plan(op, &files);
			if jobs.is_empty() {
				self.jobs.note(op, &files, "Nothing to do for these files".into());
			} else {
				let main = self.main as usize;
				self.jobs.start(op, files, jobs, move || unsafe {
					PostMessageW(main as HWND, WM_APP_JOBS, 0, 0);
				});
			}
		}
		self.refresh_hud();
	}

	fn action(&mut self, op: Op, files: &[PathBuf]) {
		let plural = |n: usize, one: &str, many: &str| {
			if n == 1 { one.to_string() } else { format!("{n} {many}") }
		};
		let note = match op {
			Op::CopyPath => {
				let text = files
					.iter()
					.map(|p| p.display().to_string())
					.collect::<Vec<_>>()
					.join("\r\n");
				shell::copy_text(self.main, &text)
					.then(|| format!("Copied {}", plural(files.len(), "the path", "paths")))
			}
			Op::CopyFile => shell::copy_files(self.main, files)
				.then(|| format!("Copied {}, ready to paste", plural(files.len(), "1 item", "items"))),
			Op::Terminal => {
				let dir = files
					.iter()
					.find(|p| kind_of(p) == Kind::Folder)
					.cloned()
					.or_else(|| files[0].parent().map(PathBuf::from))
					.unwrap_or_default();
				shell::terminal(&dir).then(|| "Opened a terminal".to_string())
			}
			Op::Reveal => shell::reveal(files).then(|| "Shown in File Explorer".to_string()),
			_ => None,
		};
		let text = note.unwrap_or_else(|| "Windows didn't allow that just now".into());
		self.jobs.note(op, files, text);
	}

	// progress cards

	fn hud_view(&self) -> HudView {
		HudView {
			cards: self.jobs.cards().into_iter().map(|(_, c)| c).collect(),
			hover: self.hud.hover,
			time: self.hud.start.elapsed().as_secs_f32(),
		}
	}

	fn refresh_hud(&mut self) {
		let cards = self.jobs.cards();
		self.hud.ids = cards.iter().map(|(id, _)| *id).collect();
		if cards.is_empty() {
			if self.hud.visible {
				self.hud.visible = false;
				self.hud.hover = None;
				unsafe {
					KillTimer(self.hud_hwnd, T_HUD);
					ShowWindow(self.hud_hwnd, SW_HIDE);
				}
			}
			return;
		}
		let (work, scale) = monitor_at(self.hud.anchor);
		self.hud.scale = scale;
		let (w, h) = hud::size(cards.len(), scale);
		let x = work.right - w as i32;
		let y = work.bottom - h as i32;
		let pm = hud::render(&self.hud_view(), &self.theme, &self.fonts, scale);
		push_layered(self.hud_hwnd, &pm, x, y);
		unsafe {
			SetWindowPos(self.hud_hwnd, HWND_TOPMOST, x, y, w as i32, h as i32, SWP_NOACTIVATE);
			if !self.hud.visible {
				ShowWindow(self.hud_hwnd, SW_SHOWNOACTIVATE);
				self.hud.visible = true;
			}
			let fast = self.jobs.any_running();
			SetTimer(self.hud_hwnd, T_HUD, if fast { 50 } else { 500 }, None);
		}
	}

	fn hud_click(&mut self, i: usize, b: Button) {
		let Some(id) = self.hud.ids.get(i).copied() else {
			return;
		};
		let Some(batch) = self.jobs.get(id) else {
			return;
		};
		match b {
			Button::Open => {
				if let Some(p) = batch.outputs.first() {
					shell::open(p);
				}
				self.jobs.remove(id);
			}
			Button::Show => {
				shell::reveal(&batch.outputs);
				self.jobs.remove(id);
			}
			Button::Undo => {
				shell::recycle(&batch.outputs);
				self.jobs.remove(id);
			}
			Button::Cancel => self.jobs.cancel(id),
			Button::Close => {
				if batch.state == State::Running {
					self.jobs.cancel(id);
				}
				self.jobs.remove(id);
			}
		}
		self.hud.hover = None;
		self.refresh_hud();
	}

	// home window

	fn home_view(&self) -> HomeView {
		HomeView {
			version: env!("CARGO_PKG_VERSION").into(),
			keys: self.keys(),
			start_with_windows: self.settings.start_with_windows,
			learn_order: self.settings.learn_order,
			paused: hooks::PAUSED.load(std::sync::atomic::Ordering::Relaxed),
			hover: self.home.hover,
			focus: self.home.focus,
		}
	}

	fn redraw_home(&mut self) {
		if self.home.hwnd.is_null() {
			return;
		}
		self.home.frame = Some(home::render(
			&self.home_view(),
			&self.theme,
			&self.fonts,
			self.home.scale,
			&self.logo,
		));
		unsafe { InvalidateRect(self.home.hwnd, std::ptr::null(), 0) };
	}

	fn style_caption(&self, hwnd: HWND) {
		let dark: u32 = self.theme.dark as u32;
		// DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_CAPTION_COLOR (Windows 11 only, ignored before)
		let caption: u32 = if self.theme.dark { 0x0020_2020 } else { 0x00f3_f3f3 };
		unsafe {
			DwmSetWindowAttribute(hwnd, 20, &dark as *const u32 as *const _, 4);
			if self.theme.win11 && !self.theme.high_contrast {
				DwmSetWindowAttribute(hwnd, 35, &caption as *const u32 as *const _, 4);
			}
		}
	}

	fn show_home(&mut self) {
		unsafe {
			if !self.home.hwnd.is_null() {
				ShowWindow(self.home.hwnd, SW_SHOWNORMAL);
				SetForegroundWindow(self.home.hwnd);
				return;
			}
			let pt = cursor();
			let (work, scale) = monitor_at(pt);
			let style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;
			let mut r = RECT {
				left: 0,
				top: 0,
				right: (home::WIDTH * scale) as i32,
				bottom: (home::HEIGHT * scale) as i32,
			};
			AdjustWindowRectExForDpi(&mut r, style, 0, 0, (scale * 96.0) as u32);
			let (w, h) = (r.right - r.left, r.bottom - r.top);
			let x = work.left + ((work.right - work.left) - w) / 2;
			let y = work.top + ((work.bottom - work.top) - h) / 2;
			let hwnd = CreateWindowExW(
				0,
				wide(HOME_CLASS).as_ptr(),
				wide("TapWheel").as_ptr(),
				style,
				x,
				y.max(work.top),
				w,
				h,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
				self.inst,
				std::ptr::null(),
			);
			self.home.hwnd = hwnd;
			self.home.scale = GetDpiForWindow(hwnd) as f32 / 96.0;
			self.style_caption(hwnd);
			self.redraw_home();
			ShowWindow(hwnd, SW_SHOWNORMAL);
			SetForegroundWindow(hwnd);
		}
	}

	fn home_action(&mut self, a: Action) {
		match a {
			Action::Key(w, c) => {
				let wheel = Wheel::ALL[w];
				self.settings.keys.assign(wheel, key_from(c));
				self.save();
			}
			Action::StartWithWindows => {
				self.settings.start_with_windows = !self.settings.start_with_windows;
				shell::set_autostart(self.settings.start_with_windows, self.packaged);
				self.save();
			}
			Action::LearnOrder => {
				self.settings.learn_order = !self.settings.learn_order;
				self.save();
			}
			Action::Paused => {
				let p = !hooks::PAUSED.load(std::sync::atomic::Ordering::Relaxed);
				hooks::PAUSED.store(p, std::sync::atomic::Ordering::Relaxed);
			}
			Action::Samples => {
				let png = icon::logo(512).encode_png().unwrap_or_default();
				match shell::samples(&png, HEIC_SAMPLE) {
					Ok(dir) => {
						shell::open(&dir);
					}
					Err(e) => {
						self.jobs
							.note(Op::Reveal, &[], format!("Couldn't make the sample files: {e}"));
						self.refresh_hud();
					}
				}
			}
			Action::Website => {
				shell::open_url(WEBSITE);
			}
			Action::Privacy => {
				shell::open_url(PRIVACY);
			}
			Action::Source => {
				shell::open_url(SOURCE);
			}
		}
		self.redraw_home();
	}

	fn home_key(&mut self, vk: u16) {
		let order = home::tab_order(&self.fonts);
		let shift = unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
		let pos = self.home.focus.and_then(|f| {
			order.iter().position(|o| match (o, f) {
				(Action::Key(a, _), Action::Key(b, _)) => *a == b,
				(o, f) => *o == f,
			})
		});
		match vk {
			VK_TAB => {
				let n = order.len();
				let next = match pos {
					None => 0,
					Some(i) if shift => (i + n - 1) % n,
					Some(i) => (i + 1) % n,
				};
				self.home.focus = Some(match order[next] {
					Action::Key(w, _) => Action::Key(w, self.keys()[w]),
					a => a,
				});
			}
			VK_LEFT | VK_RIGHT => {
				if let Some(Action::Key(w, _)) = self.home.focus {
					let cur = self.keys()[w];
					let next = if vk == VK_LEFT { (cur + 3) % 4 } else { (cur + 1) % 4 };
					self.home_action(Action::Key(w, next));
					self.home.focus = Some(Action::Key(w, next));
				}
			}
			VK_SPACE | VK_RETURN => {
				if let Some(f) = self.home.focus
					&& !matches!(f, Action::Key(..))
				{
					self.home_action(f);
				}
			}
			VK_ESCAPE => unsafe {
				ShowWindow(self.home.hwnd, SW_HIDE);
			},
			_ => return,
		}
		self.redraw_home();
	}

	// tray

	fn add_tray(&self) {
		let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
		nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
		nid.hWnd = self.main;
		nid.uID = 1;
		nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
		nid.uCallbackMessage = WM_APP_TRAY;
		nid.hIcon = self.icon;
		fill(&mut nid.szTip, "TapWheel");
		unsafe { Shell_NotifyIconW(NIM_ADD, &nid) };
	}

	fn remove_tray(&self) {
		let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
		nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
		nid.hWnd = self.main;
		nid.uID = 1;
		unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
	}

	fn tray_menu(&mut self) {
		unsafe {
			let menu = CreatePopupMenu();
			let paused = hooks::PAUSED.load(std::sync::atomic::Ordering::Relaxed);
			AppendMenuW(menu, MF_STRING, MENU_OPEN, wide("Open TapWheel").as_ptr());
			AppendMenuW(
				menu,
				MF_STRING | if paused { MF_CHECKED } else { 0 },
				MENU_PAUSE,
				wide("Pause").as_ptr(),
			);
			AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
			AppendMenuW(menu, MF_STRING, MENU_QUIT, wide("Quit TapWheel").as_ptr());
			let pt = cursor();
			// the menu only closes on an outside click when its owner is in front
			SetForegroundWindow(self.main);
			let cmd = TrackPopupMenu(
				menu,
				TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
				pt.x,
				pt.y,
				0,
				self.main,
				std::ptr::null(),
			);
			DestroyMenu(menu);
			match cmd as usize {
				MENU_OPEN => self.show_home(),
				MENU_PAUSE => {
					hooks::PAUSED.store(!paused, std::sync::atomic::Ordering::Relaxed);
					self.redraw_home();
				}
				MENU_QUIT => {
					self.remove_tray();
					self.hide_wheel();
					PostQuitMessage(0);
				}
				_ => {}
			}
		}
	}

	fn theme_changed(&mut self) {
		self.theme = system::theme();
		self.draw_wheel();
		if self.hud.visible {
			self.refresh_hud();
		}
		if !self.home.hwnd.is_null() {
			self.style_caption(self.home.hwnd);
			self.redraw_home();
		}
	}

	fn message(&mut self, text: &str) {
		let mut lines = text.lines();
		match lines.next() {
			Some("show") => self.show_home(),
			Some("pick") => {
				self.pending_pick
					.extend(lines.filter(|l| !l.is_empty()).map(PathBuf::from));
				// Explorer starts one process per selected file; gather them into one wheel
				unsafe { SetTimer(self.main, T_PICK, 180, None) };
			}
			_ => {}
		}
	}
}

struct WheelTarget;

impl Target for WheelTarget {
	fn enter(&self, files: Option<Vec<PathBuf>>, x: i32, y: i32) -> bool {
		with(|a| {
			if !a.wheel.visible || a.wheel.mode != Mode::Drag {
				return false;
			}
			a.wheel.data_seen = true;
			a.wheel.files = files;
			a.refresh_ops();
			let h = a.hover_at(POINT { x, y });
			a.wheel.hover = h;
			a.draw_wheel();
			matches!(h, Hover::Slice(_))
		})
		.unwrap_or(false)
	}

	fn over(&self, x: i32, y: i32) -> bool {
		with(|a| {
			let h = a.hover_at(POINT { x, y });
			a.set_hover(h);
			matches!(h, Hover::Slice(_)) && a.wheel.files.is_some()
		})
		.unwrap_or(false)
	}

	fn leave(&self) {
		with(|a| a.set_hover(Hover::None));
	}

	fn dropped(&self, files: Option<Vec<PathBuf>>, x: i32, y: i32) -> bool {
		with(|a| {
			let h = a.hover_at(POINT { x, y });
			let files = files.or_else(|| a.wheel.files.clone());
			let op = match h {
				Hover::Slice(i) => a.wheel.ops.get(i).copied(),
				_ => None,
			};
			a.hide_wheel();
			match (op, files) {
				(Some(op), Some(files)) => {
					a.apply(op, files);
					true
				}
				_ => false,
			}
		})
		.unwrap_or(false)
	}
}

unsafe extern "system" fn main_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	let handled = match msg {
		WM_APP_TAP => with(|a| a.tap(w)),
		WM_APP_ESCAPE => with(|a| a.hide_wheel()),
		WM_APP_DRAG_END => with(|a| {
			if a.wheel.visible && a.wheel.mode == Mode::Drag {
				// the drop arrives just after the button goes up; give it time to land
				unsafe { SetTimer(a.wheel_hwnd, T_HIDE, 400, None) };
			}
		}),
		WM_APP_JOBS => with(|a| {
			a.jobs.expire(a.hud.hover.is_some());
			a.refresh_hud();
		}),
		WM_APP_TRAY => with(|a| match l as u32 {
			WM_LBUTTONUP => a.show_home(),
			WM_RBUTTONUP | WM_CONTEXTMENU => a.tray_menu(),
			_ => {}
		}),
		WM_TIMER if w == T_PICK => with(|a| unsafe {
			KillTimer(hwnd, T_PICK);
			let files = std::mem::take(&mut a.pending_pick);
			if !files.is_empty() {
				a.show_wheel(Mode::Pick, Wheel::Convert, Some(files));
			}
		}),
		WM_COPYDATA => {
			let cds = unsafe { &*(l as *const COPYDATASTRUCT) };
			let bytes = unsafe { std::slice::from_raw_parts(cds.lpData as *const u16, cds.cbData as usize / 2) };
			let text = String::from_utf16_lossy(bytes);
			with(|a| a.message(&text));
			return 1;
		}
		WM_SETTINGCHANGE | WM_THEMECHANGED | WM_SYSCOLORCHANGE | WM_DWMCOLORIZATIONCOLORCHANGED => {
			with(|a| a.theme_changed())
		}
		WM_DESTROY => {
			with(|a| {
				a.remove_tray();
				a.hide_wheel();
			});
			unsafe { PostQuitMessage(0) };
			Some(())
		}
		m if m != 0 && with(|a| a.taskbar_created == m).unwrap_or(false) => with(|a| a.add_tray()),
		_ => None,
	};
	match handled {
		Some(()) => 0,
		None => unsafe { DefWindowProcW(hwnd, msg, w, l) },
	}
}

unsafe extern "system" fn wheel_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	let handled = match msg {
		WM_MOUSEACTIVATE => {
			let pick = with(|a| a.wheel.mode == Mode::Pick).unwrap_or(false);
			return if pick {
				MA_ACTIVATE as isize
			} else {
				MA_NOACTIVATE as isize
			};
		}
		WM_TIMER => with(|a| unsafe {
			match w {
				T_APPEAR => {
					a.wheel.appear = (a.wheel.appear + 0.22).min(1.0);
					if a.wheel.appear >= 1.0 {
						KillTimer(hwnd, T_APPEAR);
					}
					a.draw_wheel();
				}
				T_NUDGE => {
					KillTimer(hwnd, T_NUDGE);
					hooks::nudge();
				}
				T_HIDE => {
					KillTimer(hwnd, T_HIDE);
					a.hide_wheel();
				}
				_ => {}
			}
		}),
		WM_MOUSEMOVE => with(|a| {
			if a.wheel.mode == Mode::Pick {
				let h = a.hover_at(cursor());
				if h != Hover::None {
					a.set_hover(h);
				}
			}
		}),
		WM_LBUTTONUP => with(|a| {
			if a.wheel.mode == Mode::Pick {
				match a.hover_at(cursor()) {
					Hover::Slice(i) => a.pick(i),
					_ => a.hide_wheel(),
				}
			}
		}),
		WM_ACTIVATE if (w & 0xffff) as u32 == WA_INACTIVE => with(|a| {
			if a.wheel.mode == Mode::Pick {
				a.hide_wheel();
			}
		}),
		WM_KEYDOWN | WM_SYSKEYDOWN => with(|a| {
			let n = a.wheel.ops.len();
			let cur = match a.wheel.hover {
				Hover::Slice(i) => i,
				_ => 0,
			};
			let vk = w as u16;
			match vk {
				VK_ESCAPE => a.hide_wheel(),
				VK_RIGHT | VK_DOWN | VK_TAB if n > 0 => {
					let back = vk == VK_TAB && unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
					let next = if back { (cur + n - 1) % n } else { (cur + 1) % n };
					a.set_hover(Hover::Slice(next));
				}
				VK_LEFT | VK_UP if n > 0 => a.set_hover(Hover::Slice((cur + n - 1) % n)),
				VK_RETURN | VK_SPACE if n > 0 => a.pick(cur),
				0x31..=0x39 => {
					let i = (vk - 0x31) as usize;
					if i < n {
						a.pick(i);
					}
				}
				_ => {}
			}
		}),
		_ => None,
	};
	match handled {
		Some(()) => 0,
		None => unsafe { DefWindowProcW(hwnd, msg, w, l) },
	}
}

unsafe extern "system" fn hud_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	let handled = match msg {
		WM_MOUSEACTIVATE => return MA_NOACTIVATE as isize,
		WM_TIMER => with(|a| {
			let changed = a.jobs.expire(a.hud.hover.is_some());
			if changed || a.jobs.any_running() || a.hud.visible {
				a.refresh_hud();
			}
		}),
		WM_MOUSEMOVE => with(|a| unsafe {
			if !a.hud.tracking {
				let mut tme = TRACKMOUSEEVENT {
					cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
					dwFlags: TME_LEAVE,
					hwndTrack: hwnd,
					dwHoverTime: 0,
				};
				TrackMouseEvent(&mut tme);
				a.hud.tracking = true;
			}
			let s = a.hud.scale;
			let view = a.hud_view();
			let h = hud::hit(&view, &a.fonts, get_x(l) as f32 / s, get_y(l) as f32 / s);
			if h != a.hud.hover {
				a.hud.hover = h;
				a.refresh_hud();
			}
		}),
		WM_MOUSELEAVE => with(|a| {
			a.hud.tracking = false;
			a.hud.hover = None;
			a.refresh_hud();
		}),
		WM_LBUTTONUP => with(|a| {
			let s = a.hud.scale;
			let view = a.hud_view();
			if let Some((i, b)) = hud::hit(&view, &a.fonts, get_x(l) as f32 / s, get_y(l) as f32 / s) {
				a.hud_click(i, b);
			}
		}),
		_ => None,
	};
	match handled {
		Some(()) => 0,
		None => unsafe { DefWindowProcW(hwnd, msg, w, l) },
	}
}

unsafe extern "system" fn home_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	let handled = match msg {
		WM_PAINT => {
			let mut ps: PAINTSTRUCT = unsafe { std::mem::zeroed() };
			let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
			with(|a| {
				if a.home.frame.is_none() {
					a.home.frame = Some(home::render(&a.home_view(), &a.theme, &a.fonts, a.home.scale, &a.logo));
				}
				if let Some(f) = &a.home.frame {
					paint_pixmap(hdc, f);
				}
			});
			unsafe { EndPaint(hwnd, &ps) };
			Some(())
		}
		WM_ERASEBKGND => return 1,
		WM_MOUSEMOVE => with(|a| unsafe {
			if !a.home.tracking {
				let mut tme = TRACKMOUSEEVENT {
					cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
					dwFlags: TME_LEAVE,
					hwndTrack: hwnd,
					dwHoverTime: 0,
				};
				TrackMouseEvent(&mut tme);
				a.home.tracking = true;
			}
			let s = a.home.scale;
			let h = home::hit(&a.fonts, get_x(l) as f32 / s, get_y(l) as f32 / s);
			if h != a.home.hover {
				a.home.hover = h;
				a.redraw_home();
				let cursor = if h.is_some() { IDC_HAND } else { IDC_ARROW };
				SetCursor(LoadCursorW(std::ptr::null_mut(), cursor));
			}
		}),
		WM_SETCURSOR if (l & 0xffff) as u32 == HTCLIENT => {
			let hand = with(|a| a.home.hover.is_some()).unwrap_or(false);
			unsafe {
				SetCursor(LoadCursorW(
					std::ptr::null_mut(),
					if hand { IDC_HAND } else { IDC_ARROW },
				))
			};
			return 1;
		}
		WM_MOUSELEAVE => with(|a| {
			a.home.tracking = false;
			a.home.hover = None;
			a.redraw_home();
		}),
		WM_LBUTTONUP => with(|a| {
			let s = a.home.scale;
			if let Some(act) = home::hit(&a.fonts, get_x(l) as f32 / s, get_y(l) as f32 / s) {
				a.home.focus = None;
				a.home_action(act);
			}
		}),
		WM_KEYDOWN => with(|a| a.home_key(w as u16)),
		WM_DPICHANGED => with(|a| unsafe {
			a.home.scale = (w & 0xffff) as f32 / 96.0;
			let r = &*(l as *const RECT);
			SetWindowPos(
				hwnd,
				std::ptr::null_mut(),
				r.left,
				r.top,
				r.right - r.left,
				r.bottom - r.top,
				SWP_NOZORDER | SWP_NOACTIVATE,
			);
			a.redraw_home();
		}),
		WM_CLOSE => with(|a| unsafe {
			ShowWindow(hwnd, SW_HIDE);
			a.home.frame = None;
		}),
		_ => None,
	};
	match handled {
		Some(()) => 0,
		None => unsafe { DefWindowProcW(hwnd, msg, w, l) },
	}
}

fn register(inst: HINSTANCE, name: &str, proc: WNDPROC, icon: HICON, cursor: bool) {
	let class = wide(name);
	let wc = WNDCLASSEXW {
		cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
		style: 0,
		lpfnWndProc: proc,
		cbClsExtra: 0,
		cbWndExtra: 0,
		hInstance: inst,
		hIcon: icon,
		hCursor: if cursor {
			unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) }
		} else {
			std::ptr::null_mut()
		},
		hbrBackground: std::ptr::null_mut(),
		lpszMenuName: std::ptr::null(),
		lpszClassName: class.as_ptr(),
		hIconSm: icon,
	};
	unsafe { RegisterClassExW(&wc) };
}

/// Hands the request to a TapWheel that's already running. True when one took it.
fn forward(text: &str) -> bool {
	unsafe {
		let hwnd = FindWindowW(wide(MAIN_CLASS).as_ptr(), std::ptr::null());
		if hwnd.is_null() {
			return false;
		}
		let data: Vec<u16> = text.encode_utf16().collect();
		let cds = COPYDATASTRUCT {
			dwData: 1,
			cbData: (data.len() * 2) as u32,
			lpData: data.as_ptr() as *mut _,
		};
		AllowSetForegroundWindow(ASFW_ANY);
		SendMessageW(hwnd, WM_COPYDATA, 0, &cds as *const _ as LPARAM);
		true
	}
}

pub fn run() -> i32 {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let smoke = args.iter().any(|a| a == "--smoke");
	let background = args.iter().any(|a| a == "--background") || (system::packaged() && system::started_at_sign_in());
	let picked: Vec<String> = match args.iter().position(|a| a == "--pick") {
		Some(i) => args[i + 1..].iter().filter(|a| !a.starts_with("--")).cloned().collect(),
		None => Vec::new(),
	};
	let request = if picked.is_empty() {
		"show".to_string()
	} else {
		format!("pick\n{}", picked.join("\n"))
	};

	unsafe {
		SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
		if !smoke {
			CreateMutexW(std::ptr::null(), 0, wide("Local\\TapWheel.Instance").as_ptr());
			if GetLastError() == ERROR_ALREADY_EXISTS {
				if !background {
					forward(&request);
				}
				return 0;
			}
		}
		OleInitialize(std::ptr::null_mut());
	}

	let inst = unsafe { GetModuleHandleW(std::ptr::null()) } as HINSTANCE;
	let (ix, iy) = unsafe { (GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)) };
	let icon = unsafe { LoadImageW(inst, 1 as _, IMAGE_ICON, ix, iy, LR_DEFAULTCOLOR) } as HICON;
	let big = unsafe { LoadImageW(inst, 1 as _, IMAGE_ICON, 0, 0, LR_DEFAULTSIZE) } as HICON;
	register(inst, MAIN_CLASS, Some(main_proc), big, false);
	register(inst, WHEEL_CLASS, Some(wheel_proc), big, true);
	register(inst, HUD_CLASS, Some(hud_proc), big, true);
	register(inst, HOME_CLASS, Some(home_proc), big, true);

	let create = |class: &str, ex: u32, style: u32| unsafe {
		CreateWindowExW(
			ex,
			wide(class).as_ptr(),
			wide("TapWheel").as_ptr(),
			style,
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
	let main = create(MAIN_CLASS, WS_EX_TOOLWINDOW, WS_POPUP);
	let layered = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
	let wheel_hwnd = create(WHEEL_CLASS, layered, WS_POPUP);
	let hud_hwnd = create(HUD_CLASS, layered | WS_EX_NOACTIVATE, WS_POPUP);
	if main.is_null() || wheel_hwnd.is_null() || hud_hwnd.is_null() {
		return 2;
	}
	unsafe { RegisterDragDrop(wheel_hwnd, DropTarget::create(Box::new(WheelTarget))) };

	let settings_path = shell::appdata("APPDATA").join("settings.toml");
	let settings = Settings::load(&settings_path);
	let scratch = std::env::temp_dir().join("TapWheel");
	let ffmpeg = exe_dir().join("engines").join("ffmpeg.exe");
	let packaged = system::packaged();
	let hooks = if smoke { None } else { Some(Hooks::install(main)) };
	let hooks_ok = hooks.as_ref().is_none_or(Hooks::ok);

	let app = App {
		inst,
		main,
		wheel_hwnd,
		hud_hwnd,
		icon,
		_hooks: hooks,
		settings,
		settings_path,
		theme: system::theme(),
		fonts: system::fonts(),
		logo: icon::logo(128),
		packaged,
		jobs: Jobs::new(ffmpeg, scratch),
		wheel: WheelState {
			visible: false,
			mode: Mode::Drag,
			wheel: Wheel::Convert,
			files: None,
			data_seen: false,
			ops: Vec::new(),
			hover: Hover::None,
			again: false,
			scale: 1.0,
			x: 0,
			y: 0,
			appear: 1.0,
			last_tap: None,
		},
		hud: HudState {
			visible: false,
			hover: None,
			ids: Vec::new(),
			scale: 1.0,
			anchor: cursor(),
			start: Instant::now(),
			tracking: false,
		},
		home: HomeState {
			hwnd: std::ptr::null_mut(),
			hover: None,
			focus: None,
			scale: 1.0,
			frame: None,
			tracking: false,
		},
		last_op: None,
		pending_pick: Vec::new(),
		taskbar_created: unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) },
	};
	APP.with(|a| *a.borrow_mut() = Some(app));

	if smoke {
		return smoke_test();
	}

	with(|a| {
		a.add_tray();
		if !a.settings.welcomed {
			a.settings.welcomed = true;
			a.save();
			shell::set_autostart(a.settings.start_with_windows, a.packaged);
			if !a.packaged {
				shell::set_context_menu(true);
			}
		}
		if !hooks_ok {
			a.jobs.note(
				Op::Reveal,
				&[],
				"Windows blocked the key watcher; restart TapWheel".into(),
			);
			a.refresh_hud();
		}
		if !picked.is_empty() {
			a.message(&request);
		} else if !background {
			a.show_home();
		}
	});

	let mut msg: MSG = unsafe { std::mem::zeroed() };
	unsafe {
		while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
			TranslateMessage(&msg);
			DispatchMessageW(&msg);
		}
	}
	0
}

/// Opens every surface once without hooks or a tray icon, for CI on a real Windows runner.
fn smoke_test() -> i32 {
	let ok = with(|a| {
		let files = vec![exe_dir().join("photo.heic")];
		a.show_wheel(Mode::Pick, Wheel::Convert, Some(files.clone()));
		let drew = a.wheel.ops.len() >= 6;
		a.tap(1);
		let tools = a.wheel.wheel == Wheel::Tools;
		a.hide_wheel();
		a.jobs.note(Op::CopyPath, &files, "Copied the path".into());
		a.refresh_hud();
		let hud = a.hud.visible;
		a.show_home();
		a.home_action(Action::LearnOrder);
		a.home_action(Action::LearnOrder);
		let home = !a.home.hwnd.is_null() && a.home.frame.is_some();
		let line = format!("wheel={drew} tools={tools} hud={hud} home={home} theme={:?}\n", a.theme);
		let _ = std::fs::write(std::env::temp_dir().join("tapwheel-smoke.txt"), line);
		drew && tools && hud && home
	})
	.unwrap_or(false);
	if ok { 0 } else { 1 }
}
