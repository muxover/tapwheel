use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use tapwheel_core::ops::{selection_badge, selection_kind};
use tapwheel_core::run::Engines;
use tapwheel_core::settings::Key;
use tapwheel_core::{Kind, Mode as OpMode, NoDrag, Op, Recipe, Settings, Wheel, choices, recipe_for};
use tapwheel_ui::hud::{self, Button, Card, HudView, State};
use tapwheel_ui::wheel::{self, Hover, KeyHint, Motion, Slice, WheelView};
use tapwheel_ui::{Fonts, Theme};
use tiny_skia::Pixmap;
use windows_sys::Win32::Foundation::*;
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
use super::web::Web;
use super::{
	WM_APP_DOUBLE, WM_APP_DRAG_END, WM_APP_ESCAPE, WM_APP_JOBS, WM_APP_NOTAP, WM_APP_PAGE, WM_APP_PRESS, WM_APP_TAP,
	WM_APP_TRAY, WM_APP_WEB, shell, system,
};

const MAIN_CLASS: &str = "TapWheel.Main";
const WHEEL_CLASS: &str = "TapWheel.Wheel";
const HUD_CLASS: &str = "TapWheel.Hud";

const WM_MOUSELEAVE: u32 = 0x02a3;

const T_ANIM: usize = 1;
const T_NUDGE: usize = 2;
const T_HIDE: usize = 3;
const T_PROBE: usize = 4;
const T_PICK: usize = 10;
/// On the main window: TapWheel has been quiet for a while, so it gives memory back.
const T_IDLE: usize = 11;
/// The Ctrl+Shift+Space shortcut, when Settings choose it over the double tap.
const HOTKEY_NO_DRAG: i32 = 1;
/// How long nothing has to happen before memory is given back.
const QUIET_MS: u32 = 30_000;
const T_HUD: usize = 20;

// Windows animation durations: 83, 167, 250 and 333 ms
const OPEN_MS: f32 = 250.0;
const CLOSE_MS: f32 = 167.0;
const PICK_MS: f32 = 250.0;
const HOVER_MS: f32 = 83.0;
const RING_MS: f32 = 167.0;
const PAGE_MS: f32 = 250.0;
const CARD_IN_MS: f32 = 250.0;
const CARD_OUT_MS: f32 = 167.0;
const EXPAND_MS: f32 = 167.0;
/// How long the pointer rests on More before the page turns.
const MORE_DWELL_MS: u128 = 380;
/// How long to wait for a drag to reach the wheel before deciding it carries no files. The wheel
/// shows as soon as files arrive, so this only ends a probe that will never see any; a busy
/// Explorer can take longer than the old 380 ms, which looked like a tap that did nothing.
const PROBE_MS: u32 = 1000;

const MENU_OPEN: usize = 1;
const MENU_PAUSE: usize = 2;
const MENU_QUIT: usize = 3;
const MENU_REMOVE: usize = 4;

pub const HEIC_SAMPLE: &[u8] = include_bytes!("../../../core/tests/fixtures/photo.heic");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
	Drag,
	Pick,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
	Hidden,
	/// Invisible, finding out whether the drag carries files.
	Probe,
	Open,
	Closing,
}

pub struct WheelState {
	pub phase: Phase,
	pub mode: Mode,
	pub wheel: Wheel,
	pub files: Option<Vec<PathBuf>>,
	/// The centre's line about the files, made once when they arrive: working it out looks at
	/// each file on disk, too slow for every frame.
	pub subtitle: String,
	pub kind: Option<Kind>,
	pub ops: Vec<Op>,
	/// Ring choices per op: (id, label).
	pub rings: Vec<Vec<(String, String)>>,
	pub page: usize,
	pub hover: Hover,
	hover_since: Instant,
	more_turned: bool,
	pub again: bool,
	pub scale: f32,
	pub x: i32,
	pub y: i32,
	backdrop: Option<Pixmap>,
	pub last_tap: Option<(usize, Instant)>,
	opened: Instant,
	closing: Option<Instant>,
	picked: Option<usize>,
	hover_amt: Vec<f32>,
	ring_amt: f32,
	page_turned: Option<Instant>,
	tick: Instant,
	/// The tap is confirmed. A probe starts when the key goes down, so the drag can reach the
	/// wheel while the key is still held, and the wheel shows the moment the key comes up.
	armed: bool,
	/// Files that reached the probe before the tap was confirmed.
	early: Option<Vec<PathBuf>>,
}

impl WheelState {
	fn set_files(&mut self, files: Option<Vec<PathBuf>>) {
		self.subtitle = files.as_deref().map(summary).unwrap_or_default();
		self.files = files;
	}

	fn new() -> Self {
		let now = Instant::now();
		WheelState {
			phase: Phase::Hidden,
			mode: Mode::Drag,
			wheel: Wheel::Convert,
			files: None,
			subtitle: String::new(),
			kind: None,
			ops: Vec::new(),
			rings: Vec::new(),
			page: 0,
			hover: Hover::None,
			hover_since: now,
			more_turned: false,
			again: false,
			scale: 1.0,
			x: 0,
			y: 0,
			backdrop: None,
			last_tap: None,
			opened: now,
			closing: None,
			picked: None,
			hover_amt: Vec::new(),
			ring_amt: 0.0,
			page_turned: None,
			tick: now,
			armed: true,
			early: None,
		}
	}

	pub fn visible(&self) -> bool {
		matches!(self.phase, Phase::Open | Phase::Probe)
	}
}

pub struct HudState {
	visible: bool,
	hover: Option<(usize, Option<Button>)>,
	ids: Vec<u64>,
	scale: f32,
	pub anchor: POINT,
	start: Instant,
	tracking: bool,
	/// When each card arrived and, once it's going, when it started to fold away.
	anim: HashMap<u64, (Instant, Option<Instant>)>,
	expand: HashMap<u64, f32>,
	cards: Vec<Card>,
	tick: Instant,
}

pub struct App {
	pub inst: HINSTANCE,
	pub main: HWND,
	pub wheel_hwnd: HWND,
	pub hud_hwnd: HWND,
	pub icon: HICON,
	_hooks: Option<Hooks>,
	pub settings: Settings,
	pub settings_path: PathBuf,
	pub theme: Theme,
	pub fonts: Fonts,
	pub packaged: bool,
	pub jobs: Jobs,
	pub wheel: WheelState,
	pub hud: HudState,
	pub web: Web,
	pub last: Option<(Op, Option<String>)>,
	pending_pick: Vec<PathBuf>,
	taskbar_created: u32,
}

thread_local! {
	static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Runs `f` on the app unless it's already borrowed further up this stack (a window
/// message sent from inside another handler); those messages fall back to the default.
pub fn with<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
	APP.with(|a| a.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

fn key_from(i: usize) -> Key {
	match i {
		0 => Key::Shift,
		1 => Key::Ctrl,
		2 => Key::Alt,
		_ => Key::Off,
	}
}

pub fn file_name(p: &std::path::Path) -> String {
	p.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default()
}

fn summary(files: &[PathBuf]) -> String {
	match files {
		[] => String::new(),
		[one] => file_name(one),
		many => {
			let noun = selection_kind(many).map(Kind::plural).unwrap_or("items");
			let badge = selection_badge(many);
			if badge.is_empty() || selection_kind(many) == Some(Kind::Folder) {
				format!("{} {noun}", many.len())
			} else {
				format!("{} {badge} {noun}", many.len())
			}
		}
	}
}

pub fn reduced_motion() -> bool {
	let mut on: windows_sys::core::BOOL = 1;
	unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut on as *mut _ as *mut _, 0) };
	on == 0
}

fn approach(cur: f32, target: f32, dt_ms: f32, span_ms: f32) -> f32 {
	let step = dt_ms / span_ms;
	if cur < target {
		(cur + step).min(target)
	} else {
		(cur - step).max(target)
	}
}

/// A blurred copy of the screen behind a rectangle, for the Windows 11 glass.
fn capture_backdrop(x: i32, y: i32, w: u32, h: u32, dark: bool) -> Option<Pixmap> {
	let (sw, sh) = ((w / 4).max(1), (h / 4).max(1));
	let mut small = Pixmap::new(sw, sh)?;
	unsafe {
		let screen = GetDC(std::ptr::null_mut());
		let mem = CreateCompatibleDC(screen);
		let mut bi: BITMAPINFO = std::mem::zeroed();
		bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
		bi.bmiHeader.biWidth = sw as i32;
		bi.bmiHeader.biHeight = -(sh as i32);
		bi.bmiHeader.biPlanes = 1;
		bi.bmiHeader.biBitCount = 32;
		bi.bmiHeader.biCompression = BI_RGB;
		let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
		let bmp = CreateDIBSection(mem, &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
		if bmp.is_null() || bits.is_null() {
			DeleteDC(mem);
			ReleaseDC(std::ptr::null_mut(), screen);
			return None;
		}
		let old = SelectObject(mem, bmp as HGDIOBJ);
		SetStretchBltMode(mem, HALFTONE);
		StretchBlt(
			mem, 0, 0, sw as i32, sh as i32, screen, x, y, w as i32, h as i32, SRCCOPY,
		);
		let src = std::slice::from_raw_parts(bits as *const u8, (sw * sh * 4) as usize);
		for (dst, s) in small
			.data_mut()
			.as_chunks_mut::<4>()
			.0
			.iter_mut()
			.zip(src.as_chunks::<4>().0)
		{
			*dst = [s[2], s[1], s[0], 255];
		}
		SelectObject(mem, old);
		DeleteObject(bmp as HGDIOBJ);
		DeleteDC(mem);
		ReleaseDC(std::ptr::null_mut(), screen);
	}
	tapwheel_ui::draw::frost(small, w, h, dark)
}

impl App {
	pub fn save(&self) {
		let _ = self.settings.save(&self.settings_path);
	}

	fn key_hints(&self) -> Vec<KeyHint> {
		Wheel::ALL
			.iter()
			.map(|w| KeyHint {
				key: self.settings.keys.key_for(*w).label().into(),
				name: w.title().into(),
				active: *w == self.wheel.wheel && !self.wheel.again,
			})
			.filter(|k| k.key != "Off")
			.collect()
	}

	// wheel

	/// Ring choices for a slice. Move to lists the recent folders, so it's built here.
	fn ring_for(&self, op: Op) -> Vec<(String, String)> {
		if op == Op::MoveTo {
			let mut r: Vec<(String, String)> = self
				.settings
				.recent_folders
				.iter()
				.enumerate()
				.filter(|(_, d)| d.is_dir())
				.map(|(i, d)| (format!("recent{i}"), file_name(d)))
				.collect();
			r.push(("choose".into(), "Choose".into()));
			return r;
		}
		op.presets(self.wheel.kind)
			.iter()
			.map(|p| (p.id.to_string(), p.label.to_string()))
			.collect()
	}

	fn refresh_ops(&mut self) {
		let ops = match &self.wheel.files {
			None => Vec::new(),
			Some(files) if self.wheel.again => match self.last.as_ref().map(|l| l.0) {
				Some(op) if Wheel::ALL.iter().any(|wh| choices(*wh, files).contains(&op)) => vec![op],
				_ => Vec::new(),
			},
			Some(files) => {
				let mut ops = choices(self.wheel.wheel, files);
				self.settings.order(&mut ops);
				ops
			}
		};
		self.wheel.rings = ops.iter().map(|op| self.ring_for(*op)).collect();
		self.wheel.ops = ops;
		self.wheel.page = 0;
		self.wheel.hover_amt = vec![0.0; wheel::PER_PAGE];
		self.wheel.ring_amt = 0.0;
	}

	fn wheel_view(&self) -> WheelView {
		let w = &self.wheel;
		let slices = w
			.ops
			.iter()
			.zip(&w.rings)
			.map(|(op, ring)| Slice {
				label: if w.again {
					format!("Again: {}", op.label())
				} else {
					op.label().to_string()
				},
				icon: op.icon(w.kind).to_string(),
				desc: op.describe(w.kind),
				ring: ring.iter().map(|(_, l)| l.clone()).collect(),
			})
			.collect();
		let message = match (&w.files, w.ops.is_empty()) {
			(Some(_), true) if w.again => Some("Your last choice doesn't fit these files".to_string()),
			(Some(_), true) => Some("Nothing here fits these files".to_string()),
			_ => None,
		};
		let now = Instant::now();
		let ms = |t: Instant| now.duration_since(t).as_secs_f32() * 1000.0;
		let motion = Motion {
			open: if reduced_motion() {
				1.0
			} else {
				(ms(w.opened) / OPEN_MS).min(1.0)
			},
			close: w
				.closing
				.map(|t| (ms(t) / if w.picked.is_some() { PICK_MS } else { CLOSE_MS }).min(1.0))
				.unwrap_or(0.0),
			picked: w.picked,
			hover: w.hover_amt.clone(),
			ring: w.ring_amt,
			page: w.page_turned.map(|t| (ms(t) / PAGE_MS).min(1.0)).unwrap_or(1.0),
		};
		WheelView {
			slices,
			page: w.page,
			hover: w.hover,
			keyboard: w.mode == Mode::Pick,
			title: if w.again {
				"Again".into()
			} else {
				w.wheel.title().into()
			},
			subtitle: w.subtitle.clone(),
			keys: self.key_hints(),
			message,
			hint: None,
			motion,
		}
	}

	fn draw_wheel(&mut self) {
		match self.wheel.phase {
			Phase::Open | Phase::Closing => {
				let pm = wheel::render(
					&self.wheel_view(),
					&self.theme,
					&self.fonts,
					self.wheel.scale,
					self.wheel.backdrop.as_ref(),
				);
				push_layered(self.wheel_hwnd, &pm, self.wheel.x, self.wheel.y);
			}
			Phase::Probe => push_layered(
				self.wheel_hwnd,
				&wheel::probe(self.wheel.scale),
				self.wheel.x,
				self.wheel.y,
			),
			Phase::Hidden => {}
		}
	}

	fn animate(&mut self) {
		unsafe { SetTimer(self.wheel_hwnd, T_ANIM, 8, None) };
	}

	/// Lets clicks and drops fall through the wheel while it closes.
	fn wheel_click_through(&self, on: bool) {
		unsafe {
			let ex = GetWindowLongPtrW(self.wheel_hwnd, GWL_EXSTYLE) as u32;
			let ex = if on {
				ex | WS_EX_TRANSPARENT
			} else {
				ex & !WS_EX_TRANSPARENT
			};
			SetWindowLongPtrW(self.wheel_hwnd, GWL_EXSTYLE, ex as isize);
		}
	}

	fn place_wheel(&mut self) {
		let pt = cursor();
		let (work, scale) = monitor_at(pt);
		let scale = wheel::fit_scale(scale, work.right - work.left, work.bottom - work.top);
		let (w, h) = wheel::size(scale);
		self.wheel.scale = scale;
		self.wheel.x = (pt.x - (wheel::CX * scale) as i32).clamp(work.left, (work.right - w as i32).max(work.left));
		self.wheel.y = (pt.y - (wheel::CY * scale) as i32).clamp(work.top, (work.bottom - h as i32).max(work.top));
		unsafe {
			SetWindowPos(
				self.wheel_hwnd,
				HWND_TOPMOST,
				self.wheel.x,
				self.wheel.y,
				w as i32,
				h as i32,
				SWP_NOACTIVATE,
			);
		}
	}

	/// During a drag: an invisible wheel catches the drag to see what it carries. It only becomes
	/// visible when files arrive, so dragging a window, text or nothing opens no wheel.
	fn probe(&mut self, wh: Wheel, armed: bool) {
		if self.wheel.closing.take().is_some() {
			self.wheel_click_through(false);
		}
		let last_tap = self.wheel.last_tap;
		self.wheel = WheelState::new();
		self.wheel.last_tap = last_tap;
		self.wheel.wheel = wh;
		self.wheel.phase = Phase::Probe;
		self.wheel.armed = armed;
		self.place_wheel();
		hooks::WHEEL_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
		self.draw_wheel();
		unsafe {
			ShowWindow(self.wheel_hwnd, SW_SHOWNOACTIVATE);
			SetTimer(self.wheel_hwnd, T_NUDGE, 16, None);
			SetTimer(self.wheel_hwnd, T_PROBE, PROBE_MS, None);
		}
	}

	/// Shows the wheel for files we know about.
	fn open(&mut self, mode: Mode, wh: Wheel, files: Vec<PathBuf>) {
		if self.wheel.phase != Phase::Probe {
			if self.wheel.closing.take().is_some() {
				self.wheel_click_through(false);
			}
			let last_tap = self.wheel.last_tap;
			self.wheel = WheelState::new();
			self.wheel.last_tap = last_tap;
			self.place_wheel();
		}
		unsafe { KillTimer(self.wheel_hwnd, T_PROBE) };
		self.wheel.phase = Phase::Open;
		self.wheel.mode = mode;
		self.wheel.wheel = wh;
		self.wheel.kind = selection_kind(&files);
		self.wheel.set_files(Some(files));
		self.refresh_ops();
		self.wheel.opened = Instant::now();
		// a picture's Tools wheel may lead to Remove BG: load the model while the hand is still moving
		if self.wheel.kind == Some(Kind::Image) {
			static WARMED: std::sync::Once = std::sync::Once::new();
			let engines = self.jobs.engines.clone();
			WARMED.call_once(|| {
				std::thread::spawn(move || tapwheel_core::bg::warm(&engines));
			});
		}
		if self.theme.win11 && !self.theme.high_contrast {
			let (w, h) = wheel::size(self.wheel.scale);
			self.wheel.backdrop = capture_backdrop(self.wheel.x, self.wheel.y, w, h, self.theme.dark);
		}
		if mode == Mode::Pick && !self.wheel.ops.is_empty() {
			self.wheel.hover = Hover::Slice(0);
		}
		hooks::WHEEL_OPEN.store(true, std::sync::atomic::Ordering::Relaxed);
		self.draw_wheel();
		// how long the wheel takes to appear, for finding slow spots on real PCs
		let took = self.wheel.opened.elapsed().as_millis();
		if took > 60 {
			super::log::write(&format!("slow: the wheel took {took} ms to open"));
		}
		unsafe {
			if mode == Mode::Pick {
				ShowWindow(self.wheel_hwnd, SW_SHOW);
				bring_to_front(self.wheel_hwnd);
			} else {
				ShowWindow(self.wheel_hwnd, SW_SHOWNOACTIVATE);
			}
		}
		self.animate();
	}

	/// Closes with the fade, or with the picked slice's pulse.
	fn close(&mut self, picked: Option<usize>) {
		if !self.wheel.visible() {
			return;
		}
		self.quiet_later();
		let was_probe = self.wheel.phase == Phase::Probe;
		self.wheel.phase = Phase::Closing;
		self.wheel.picked = picked;
		self.wheel.set_files(None);
		hooks::WHEEL_OPEN.store(false, std::sync::atomic::Ordering::Relaxed);
		unsafe {
			KillTimer(self.wheel_hwnd, T_HIDE);
			KillTimer(self.wheel_hwnd, T_PROBE);
		}
		if was_probe || reduced_motion() {
			self.finish_close();
			return;
		}
		self.wheel.closing = Some(Instant::now());
		self.wheel_click_through(true);
		self.animate();
	}

	fn finish_close(&mut self) {
		self.wheel.phase = Phase::Hidden;
		self.wheel.closing = None;
		self.wheel.backdrop = None;
		self.wheel.ops.clear();
		unsafe {
			KillTimer(self.wheel_hwnd, T_ANIM);
			ShowWindow(self.wheel_hwnd, SW_HIDE);
		}
		self.wheel_click_through(false);
	}

	fn tick(&mut self) {
		let now = Instant::now();
		let dt = now.duration_since(self.wheel.tick).as_secs_f32() * 1000.0;
		self.wheel.tick = now;
		let w = &mut self.wheel;
		if w.phase == Phase::Closing {
			let span = if w.picked.is_some() { PICK_MS } else { CLOSE_MS };
			if w.closing
				.is_none_or(|t| now.duration_since(t).as_secs_f32() * 1000.0 >= span)
			{
				self.finish_close();
				return;
			}
			self.draw_wheel();
			return;
		}
		if w.phase != Phase::Open {
			unsafe { KillTimer(self.wheel_hwnd, T_ANIM) };
			return;
		}
		let hovered = match w.hover {
			Hover::Slice(i) | Hover::Ring(i, _) => Some(i),
			Hover::More => Some(wheel::PER_PAGE - 1),
			_ => None,
		};
		let mut moving = false;
		for (i, a) in w.hover_amt.iter_mut().enumerate() {
			let target = if hovered == Some(i) { 1.0 } else { 0.0 };
			let next = approach(*a, target, dt, HOVER_MS);
			moving |= (next - target).abs() > f32::EPSILON;
			*a = next;
		}
		let ring_target = match w.hover {
			Hover::Slice(_) | Hover::Ring(..) => 1.0,
			_ => 0.0,
		};
		w.ring_amt = approach(w.ring_amt, ring_target, dt, RING_MS);
		moving |= (w.ring_amt - ring_target).abs() > f32::EPSILON;
		let opening = now.duration_since(w.opened).as_secs_f32() * 1000.0 < OPEN_MS;
		let turning = w
			.page_turned
			.is_some_and(|t| now.duration_since(t).as_secs_f32() * 1000.0 < PAGE_MS);
		// resting on More turns the page
		if w.hover == Hover::More && !w.more_turned && now.duration_since(w.hover_since).as_millis() >= MORE_DWELL_MS {
			w.more_turned = true;
			self.turn_page(1);
			return;
		}
		let dwelling = self.wheel.hover == Hover::More && !self.wheel.more_turned;
		self.draw_wheel();
		if !(moving || opening || turning || dwelling) {
			unsafe { KillTimer(self.wheel_hwnd, T_ANIM) };
		}
	}

	fn turn_page(&mut self, by: isize) {
		let pages = wheel::pages(self.wheel.ops.len());
		if pages <= 1 {
			return;
		}
		self.wheel.page = (self.wheel.page as isize + by).rem_euclid(pages as isize) as usize;
		self.wheel.page_turned = Some(Instant::now());
		self.wheel.ring_amt = 0.0;
		self.wheel.hover_amt = vec![0.0; wheel::PER_PAGE];
		self.wheel.hover = self.hover_at(cursor());
		self.wheel.hover_since = Instant::now();
		self.draw_wheel();
		self.animate();
	}

	fn tap(&mut self, key: usize) {
		let Some(wh) = self.settings.keys.wheel_for(key_from(key)) else {
			return;
		};
		let now = Instant::now();
		let double = matches!(self.wheel.last_tap, Some((k, t)) if k == key && now.duration_since(t).as_millis() < 450);
		self.wheel.last_tap = Some((key, now));
		match self.wheel.phase {
			Phase::Hidden | Phase::Closing => {
				if hooks::dragging() {
					self.probe(wh, true);
				}
			}
			Phase::Probe => {
				self.wheel.wheel = wh;
				self.wheel.armed = true;
				if let Some(files) = self.wheel.early.take() {
					self.open(Mode::Drag, wh, files);
					let h = self.hover_at(cursor());
					self.set_hover(h);
					self.draw_wheel();
					// the drag only asks the wheel again when the pointer moves
					hooks::nudge();
				}
			}
			Phase::Open => {
				if double && self.last.is_some() && self.wheel.wheel == wh {
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
				self.wheel.opened = Instant::now() - std::time::Duration::from_millis((OPEN_MS * 0.55) as u64);
				self.draw_wheel();
				self.animate();
			}
		}
	}

	/// The wheel on the files selected in Explorer or on the desktop, with no drag: from a double
	/// tap or the shortcut. Anywhere else the keys do nothing.
	fn no_drag(&mut self, wh: Wheel) {
		if self.wheel.visible() || hooks::dragging() {
			return;
		}
		if let Some(files) = super::selection::selected() {
			self.open(Mode::Pick, wh, files);
		}
	}

	/// Turns on the way in without a drag that Settings chose. False when another app already
	/// holds the shortcut.
	pub fn apply_no_drag(&self) -> bool {
		let how = self.settings.no_drag;
		hooks::DOUBLE_TAP.store(how == NoDrag::DoubleTap, std::sync::atomic::Ordering::Relaxed);
		unsafe {
			UnregisterHotKey(self.main, HOTKEY_NO_DRAG);
			if how == NoDrag::Shortcut
				&& RegisterHotKey(
					self.main,
					HOTKEY_NO_DRAG,
					MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT,
					VK_SPACE as u32,
				) == 0
			{
				super::log::write("Ctrl+Shift+Space is taken by another app");
				return false;
			}
		}
		true
	}

	/// A trigger key went down during a drag: start looking at what it carries.
	fn press(&mut self, key: usize) {
		let Some(wh) = self.settings.keys.wheel_for(key_from(key)) else {
			return;
		};
		if matches!(self.wheel.phase, Phase::Hidden | Phase::Closing) && hooks::dragging() {
			self.probe(wh, false);
		}
	}

	/// The key went down but wasn't a tap: held, or part of a shortcut.
	fn no_tap(&mut self) {
		if self.wheel.phase == Phase::Probe && !self.wheel.armed {
			self.close(None);
		}
	}

	fn hover_at(&self, pt: POINT) -> Hover {
		let s = self.wheel.scale;
		let x = (pt.x - self.wheel.x) as f32 / s;
		let y = (pt.y - self.wheel.y) as f32 / s;
		wheel::hit(&self.wheel_view(), &self.fonts, x, y)
	}

	fn set_hover(&mut self, h: Hover) {
		if self.wheel.hover != h {
			let same_slice = matches!((self.wheel.hover, h), (Hover::Slice(a) | Hover::Ring(a, _), Hover::Slice(b) | Hover::Ring(b, _)) if a == b);
			if !same_slice {
				self.wheel.ring_amt = 0.0;
			}
			if h == Hover::More && self.wheel.hover != Hover::More {
				self.wheel.more_turned = false;
			}
			self.wheel.hover = h;
			self.wheel.hover_since = Instant::now();
			self.animate();
		}
	}

	/// The op and ring choice under a hover, if it picks anything.
	fn choice(&self, h: Hover) -> Option<(usize, Op, Option<String>)> {
		let (items, _) = wheel::page_items(self.wheel.ops.len(), self.wheel.page);
		match h {
			Hover::Slice(i) => {
				let k = *items.get(i)?;
				Some((i, self.wheel.ops[k], None))
			}
			Hover::Ring(i, j) => {
				let k = *items.get(i)?;
				let id = self.wheel.rings.get(k)?.get(j)?.0.clone();
				Some((i, self.wheel.ops[k], Some(id)))
			}
			_ => None,
		}
	}

	fn pick(&mut self, h: Hover) {
		let files = self.wheel.files.clone();
		match (self.choice(h), files) {
			(Some((slot, op, preset)), Some(files)) => {
				self.close(Some(slot));
				self.apply(op, preset, files);
			}
			_ => self.close(None),
		}
	}

	// work

	/// Starts (or restarts) the wait before memory is given back.
	pub fn quiet_later(&self) {
		unsafe { SetTimer(self.main, T_IDLE, QUIET_MS, None) };
	}

	/// Gives back memory once nothing is running and no window is open: the background model,
	/// the heap's free blocks, and the pages only kept for speed. Opening the wheel brings back
	/// the few it needs.
	fn give_back_memory(&mut self) {
		unsafe { KillTimer(self.main, T_IDLE) };
		if self.jobs.any_running() || !self.web.windows.is_empty() || self.wheel.visible() || self.hud.visible {
			// still busy: look again later, so memory comes back once the last thing ends
			self.quiet_later();
			return;
		}
		tapwheel_core::bg::release();
		unsafe {
			windows_sys::Win32::System::Memory::HeapCompact(windows_sys::Win32::System::Memory::GetProcessHeap(), 0);
			windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet(
				windows_sys::Win32::System::Threading::GetCurrentProcess(),
			);
		}
	}

	pub fn apply(&mut self, op: Op, preset: Option<String>, files: Vec<PathBuf>) {
		self.settings.record(op);
		self.save();
		self.last = Some((op, preset.clone()));
		self.hud.anchor = cursor();
		let kind = selection_kind(&files);
		if op.mode() == OpMode::Action {
			self.action(op, preset.as_deref(), &files);
		} else {
			match recipe_for(op, preset.as_deref(), kind, &self.settings.defaults) {
				Some(r) => self.run_recipe(op, r, files),
				None => self.open_editor(op, files),
			}
		}
		self.refresh_hud();
	}

	pub fn run_recipe(&mut self, op: Op, recipe: Recipe, files: Vec<PathBuf>) {
		let kind = selection_kind(&files);
		let main = self.main as usize;
		// the job's thread works out its files and names, which looks at each one on disk
		self.jobs.start(
			op,
			kind,
			files,
			recipe,
			self.settings.defaults.clone(),
			move || unsafe {
				PostMessageW(main as HWND, WM_APP_JOBS, 0, 0);
			},
		);
		self.refresh_hud();
	}

	pub fn wake(&self) -> impl Fn() + Send + 'static {
		let main = self.main as usize;
		move || unsafe {
			PostMessageW(main as HWND, WM_APP_JOBS, 0, 0);
		}
	}

	// progress cards

	fn hud_view(&self) -> HudView {
		HudView {
			cards: self.hud.cards.clone(),
			hover: self
				.hud
				.hover
				.and_then(|(i, b)| b.map(|b| (i, b)))
				.or(self.hud.hover.map(|(i, _)| (i, Button::Close))),
			time: self.hud.start.elapsed().as_secs_f32(),
		}
	}

	fn expand_list(&self) -> Vec<f32> {
		self.hud
			.ids
			.iter()
			.map(|id| self.hud.expand.get(id).copied().unwrap_or(0.0))
			.collect()
	}

	/// Starts folding a card away; it's dropped once the animation ends.
	fn leave(&mut self, id: u64) {
		let entry = self.hud.anim.entry(id).or_insert((Instant::now(), None));
		if entry.1.is_none() {
			entry.1 = Some(Instant::now());
		}
	}

	pub fn refresh_hud(&mut self) {
		let hovered_id = self.hud.hover.and_then(|(i, _)| self.hud.ids.get(i).copied());
		for id in self.jobs.expired(hovered_id) {
			self.leave(id);
		}
		let still = reduced_motion();
		let now = Instant::now();
		let dt = now.duration_since(self.hud.tick).as_secs_f32() * 1000.0;
		self.hud.tick = now;
		let mut cards = Vec::new();
		let mut ids = Vec::new();
		let mut moving = false;
		let mut gone = Vec::new();
		for (id, mut card) in self.jobs.cards() {
			let (arrived, leaving) = *self.hud.anim.entry(id).or_insert((now, None));
			let ms = |t: Instant| now.duration_since(t).as_secs_f32() * 1000.0;
			card.shown = match leaving {
				Some(_) if still => 0.0,
				Some(t) => 1.0 - ms(t) / CARD_OUT_MS,
				None if still => 1.0,
				None => ms(arrived) / CARD_IN_MS,
			}
			.clamp(0.0, 1.0);
			if leaving.is_some() && card.shown <= 0.0 {
				gone.push(id);
				continue;
			}
			moving |= card.shown < 1.0;
			let target = if hovered_id == Some(id) { 1.0 } else { 0.0 };
			let e = self.hud.expand.entry(id).or_insert(0.0);
			*e = if still {
				target
			} else {
				approach(*e, target, dt, EXPAND_MS)
			};
			moving |= (*e - target).abs() > f32::EPSILON;
			ids.push(id);
			cards.push(card);
		}
		for id in gone {
			self.jobs.remove(id);
			self.hud.anim.remove(&id);
			self.hud.expand.remove(&id);
		}
		self.hud.anim.retain(|id, _| ids.contains(id));
		if self.hud.hover.is_some_and(|(i, _)| i >= cards.len()) {
			self.hud.hover = None;
		}
		self.hud.ids = ids;
		self.hud.cards = cards;
		if self.hud.cards.is_empty() {
			if self.hud.visible {
				self.hud.visible = false;
				self.hud.hover = None;
				unsafe {
					KillTimer(self.hud_hwnd, T_HUD);
					ShowWindow(self.hud_hwnd, SW_HIDE);
				}
				self.quiet_later();
			}
			return;
		}
		let (work, scale) = monitor_at(self.hud.anchor);
		self.hud.scale = scale;
		let expand = self.expand_list();
		let (w, h) = hud::size(&self.hud.cards, &expand, scale);
		let x = work.right - w as i32;
		let y = work.bottom - h as i32;
		let pm = hud::render(&self.hud_view(), &expand, &self.theme, &self.fonts, scale);
		push_layered(self.hud_hwnd, &pm, x, y);
		unsafe {
			SetWindowPos(self.hud_hwnd, HWND_TOPMOST, x, y, w as i32, h as i32, SWP_NOACTIVATE);
			if !self.hud.visible {
				ShowWindow(self.hud_hwnd, SW_SHOWNOACTIVATE);
				self.hud.visible = true;
			}
			let every = if moving {
				8
			} else if self.jobs.any_running() {
				50
			} else {
				400
			};
			SetTimer(self.hud_hwnd, T_HUD, every, None);
		}
	}

	fn hud_click(&mut self, i: usize, b: Button) {
		let Some(id) = self.hud.ids.get(i).copied() else {
			return;
		};
		let Some(batch) = self.jobs.get(id) else {
			return;
		};
		let (op, kind) = (batch.op, batch.kind);
		let mut kept: Vec<PathBuf> = Vec::new();
		let mut again: Option<(Recipe, Vec<PathBuf>)> = None;
		match b {
			Button::Open => {
				if let Some(p) = batch.outputs.first() {
					shell::open(p);
				}
			}
			Button::Show => {
				shell::reveal(&batch.outputs);
			}
			Button::Undo => {
				// only what TapWheel made and nobody has touched since goes to the Recycle Bin
				let mut gone = Vec::new();
				for (i, p) in batch.outputs.iter().enumerate() {
					match batch.stamps.get(i).copied().flatten() {
						Some(was) if !tapwheel_core::run::unchanged(p, was) => kept.push(p.clone()),
						_ => gone.push(p.clone()),
					}
				}
				shell::recycle(&gone);
			}
			Button::Retry => {
				again = batch.recipe.clone().map(|r| (r, batch.failed.clone()));
			}
			Button::Copy => {
				if let Some(c) = &batch.copy {
					shell::copy_text(self.main, c);
				}
			}
			Button::Cancel | Button::Close => {
				if matches!(batch.state, State::Running | State::Waiting) {
					self.jobs.cancel(id);
				}
			}
		}
		self.leave(id);
		if let Some((recipe, files)) = again {
			self.run_recipe(op, recipe, files);
		}
		if !kept.is_empty() {
			let text = match kept.as_slice() {
				[one] => format!("{} changed after it was made, so Undo left it", file_name(one)),
				many => format!("{} files changed after they were made, so Undo left them", many.len()),
			};
			self.jobs.note(op, kind, &kept, text);
		}
		self.hud.hover = None;
		self.refresh_hud();
	}

	// settings window and tray

	pub fn show_settings(&mut self) {
		let init = self.settings_payload();
		super::web::open(self, super::web::Page::Settings, init);
	}

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
			// the Store removes its own; the zip version has nothing else that undoes its setup
			if !self.packaged {
				AppendMenuW(menu, MF_STRING, MENU_REMOVE, wide("Remove from this PC").as_ptr());
			}
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
				MENU_OPEN => self.show_settings(),
				MENU_PAUSE => hooks::PAUSED.store(!paused, std::sync::atomic::Ordering::Relaxed),
				MENU_QUIT => {
					self.remove_tray();
					self.close(None);
					PostQuitMessage(0);
				}
				MENU_REMOVE => {
					let ask = "Remove TapWheel's right-click entry, its start-up entry and its settings from this PC, then quit?\n\nYour files aren't touched. Afterwards you can delete the TapWheel folder.";
					let yes = MessageBoxW(
						self.main,
						wide(ask).as_ptr(),
						wide("Remove TapWheel").as_ptr(),
						MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
					) == IDYES;
					if yes {
						shell::set_context_menu(false);
						shell::set_autostart(false, false);
						let _ = std::fs::remove_dir_all(shell::appdata("APPDATA"));
						let _ = std::fs::remove_dir_all(shell::appdata("LOCALAPPDATA"));
						shell::remove_samples();
						self.remove_tray();
						self.close(None);
						PostQuitMessage(0);
					}
				}
				_ => {}
			}
		}
	}

	fn theme_changed(&mut self) {
		self.theme = system::theme();
		if self.wheel.visible() {
			self.draw_wheel();
		}
		if self.hud.visible {
			self.refresh_hud();
		}
		super::web::theme_changed(self);
	}

	fn message(&mut self, text: &str) {
		let mut lines = text.lines();
		match lines.next() {
			Some("show") => self.show_settings(),
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
			match a.wheel.phase {
				Phase::Probe => match files.filter(|f| !f.is_empty()) {
					Some(files) => {
						hooks::FILES.store(true, std::sync::atomic::Ordering::Relaxed);
						if !a.wheel.armed {
							a.wheel.early = Some(files);
							return false;
						}
						let wh = a.wheel.wheel;
						a.open(Mode::Drag, wh, files);
					}
					// not files: no wheel at all
					None => {
						a.close(None);
						return false;
					}
				},
				Phase::Open if a.wheel.mode == Mode::Drag => {
					if let Some(f) = files.filter(|f| !f.is_empty()) {
						hooks::FILES.store(true, std::sync::atomic::Ordering::Relaxed);
						a.wheel.set_files(Some(f));
					}
				}
				_ => return false,
			}
			let h = a.hover_at(POINT { x, y });
			a.set_hover(h);
			a.draw_wheel();
			matches!(h, Hover::Slice(_) | Hover::Ring(..))
		})
		.unwrap_or(false)
	}

	fn over(&self, x: i32, y: i32) -> bool {
		with(|a| {
			if a.wheel.phase != Phase::Open {
				return false;
			}
			let h = a.hover_at(POINT { x, y });
			a.set_hover(h);
			matches!(h, Hover::Slice(_) | Hover::Ring(..)) && !a.wheel.ops.is_empty()
		})
		.unwrap_or(false)
	}

	fn leave(&self) {
		with(|a| {
			if a.wheel.phase == Phase::Open {
				a.set_hover(Hover::None);
			}
		});
	}

	fn dropped(&self, files: Option<Vec<PathBuf>>, x: i32, y: i32) -> bool {
		with(|a| {
			if a.wheel.phase != Phase::Open {
				return false;
			}
			let h = a.hover_at(POINT { x, y });
			if let Some(f) = files.filter(|f| !f.is_empty()) {
				a.wheel.set_files(Some(f));
			}
			let acts = a.choice(h).is_some();
			a.pick(h);
			acts
		})
		.unwrap_or(false)
	}
}

unsafe extern "system" fn main_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	let handled = match msg {
		WM_APP_TAP => with(|a| a.tap(w)),
		WM_APP_DOUBLE => with(|a| {
			if let Some(wh) = a.settings.keys.wheel_for(key_from(w)) {
				a.no_drag(wh);
			}
		}),
		WM_HOTKEY if w as i32 == HOTKEY_NO_DRAG => with(|a| a.no_drag(Wheel::Convert)),
		WM_APP_PRESS => with(|a| a.press(w)),
		WM_APP_NOTAP => with(|a| a.no_tap()),
		WM_APP_ESCAPE => with(|a| a.close(None)),
		WM_APP_PAGE => with(|a| {
			if a.wheel.phase == Phase::Open {
				a.turn_page(if w == 0 { 1 } else { -1 });
			}
		}),
		WM_APP_DRAG_END => with(|a| {
			if a.wheel.visible() && a.wheel.mode == Mode::Drag {
				// the drop arrives just after the button goes up; give it time to land
				unsafe { SetTimer(a.wheel_hwnd, T_HIDE, 400, None) };
			}
		}),
		WM_APP_JOBS => with(|a| {
			if let Some(why) = super::share::take_failure() {
				a.jobs
					.problem(Op::Share, None, &[], format!("Sharing didn't open: {why}"));
			}
			a.refresh_hud()
		}),
		WM_APP_WEB => with(super::web::pump),
		WM_APP_TRAY => with(|a| match l as u32 {
			WM_LBUTTONUP => a.show_settings(),
			WM_RBUTTONUP | WM_CONTEXTMENU => a.tray_menu(),
			_ => {}
		}),
		WM_TIMER if w == T_IDLE => with(|a| a.give_back_memory()),
		WM_TIMER if w == T_PICK => with(|a| unsafe {
			KillTimer(hwnd, T_PICK);
			let files = std::mem::take(&mut a.pending_pick);
			if !files.is_empty() {
				a.open(Mode::Pick, Wheel::Convert, files);
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
				a.close(None);
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
				T_ANIM => a.tick(),
				T_NUDGE => {
					KillTimer(hwnd, T_NUDGE);
					hooks::nudge();
				}
				T_PROBE => {
					// the drag never reached the wheel with files
					KillTimer(hwnd, T_PROBE);
					if a.wheel.phase == Phase::Probe {
						a.close(None);
					}
				}
				T_HIDE => {
					KillTimer(hwnd, T_HIDE);
					a.close(None);
				}
				_ => {}
			}
		}),
		WM_MOUSEMOVE => with(|a| {
			if a.wheel.mode == Mode::Pick && a.wheel.phase == Phase::Open {
				let h = a.hover_at(cursor());
				if h != Hover::None {
					a.set_hover(h);
				}
			}
		}),
		WM_LBUTTONUP => with(|a| {
			if a.wheel.mode == Mode::Pick && a.wheel.phase == Phase::Open {
				let h = a.hover_at(cursor());
				a.pick(h);
			}
		}),
		WM_ACTIVATE if (w & 0xffff) as u32 == WA_INACTIVE => with(|a| {
			if a.wheel.mode == Mode::Pick {
				a.close(None);
			}
		}),
		WM_KEYDOWN | WM_SYSKEYDOWN => with(|a| {
			let (items, more) = wheel::page_items(a.wheel.ops.len(), a.wheel.page);
			let n = items.len();
			let cur = match a.wheel.hover {
				Hover::Slice(i) | Hover::Ring(i, _) => i,
				_ => 0,
			};
			let vk = w as u16;
			match vk {
				VK_ESCAPE => a.close(None),
				VK_NEXT if more => a.turn_page(1),
				VK_PRIOR if more => a.turn_page(-1),
				VK_RIGHT | VK_DOWN | VK_TAB if n > 0 => {
					let back = vk == VK_TAB && unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
					if !back && cur + 1 == n && more {
						a.turn_page(1);
						a.set_hover(Hover::Slice(0));
					} else {
						let next = if back { (cur + n - 1) % n } else { (cur + 1) % n };
						a.set_hover(Hover::Slice(next));
					}
				}
				VK_LEFT | VK_UP if n > 0 => a.set_hover(Hover::Slice((cur + n - 1) % n)),
				VK_RETURN | VK_SPACE if n > 0 => {
					let h = a.wheel.hover;
					a.pick(h);
				}
				0x31..=0x39 => {
					let i = (vk - 0x31) as usize;
					if i < n {
						a.pick(Hover::Slice(i));
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
		WM_TIMER => with(|a| a.refresh_hud()),
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
			let expand = a.expand_list();
			let h = hud::hit(&view, &expand, &a.fonts, get_x(l) as f32 / s, get_y(l) as f32 / s);
			if h != a.hud.hover {
				a.hud.hover = h;
				a.refresh_hud();
			}
		}),
		WM_SETCURSOR if (l & 0xffff) as u32 == HTCLIENT => {
			let hand = with(|a| a.hud.hover.is_some_and(|(_, b)| b.is_some())).unwrap_or(false);
			unsafe {
				SetCursor(LoadCursorW(
					std::ptr::null_mut(),
					if hand { IDC_HAND } else { IDC_ARROW },
				))
			};
			return 1;
		}
		WM_MOUSELEAVE => with(|a| {
			a.hud.tracking = false;
			a.hud.hover = None;
			a.refresh_hud();
		}),
		WM_LBUTTONUP => with(|a| {
			let s = a.hud.scale;
			let view = a.hud_view();
			let expand = a.expand_list();
			if let Some((i, Some(b))) = hud::hit(&view, &expand, &a.fonts, get_x(l) as f32 / s, get_y(l) as f32 / s) {
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

pub fn register(inst: HINSTANCE, name: &str, proc: WNDPROC, icon: HICON, cursor: bool) {
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

/// Brings a TapWheel window to the front. Windows only lets the app with the last input do that,
/// and a key watched through the hooks isn't TapWheel's input, so it joins the front window's
/// input for the moment of the switch.
fn bring_to_front(hwnd: HWND) {
	unsafe {
		if SetForegroundWindow(hwnd) != 0 && GetForegroundWindow() == hwnd {
			return;
		}
		let front = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
		let me = windows_sys::Win32::System::Threading::GetCurrentThreadId();
		let joined =
			front != 0 && front != me && windows_sys::Win32::System::Threading::AttachThreadInput(me, front, 1) != 0;
		SetForegroundWindow(hwnd);
		if joined {
			windows_sys::Win32::System::Threading::AttachThreadInput(me, front, 0);
		}
		if GetForegroundWindow() != hwnd {
			super::log::write("the wheel couldn't take the keyboard");
		}
	}
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
	super::web::register_class(inst, big);
	super::float::register_class(inst, big);

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
	let local = shell::appdata("LOCALAPPDATA");
	let mut engines = Engines::in_dir(&exe_dir().join("engines"), std::env::temp_dir().join("TapWheel"));
	engines.cache = local.join("Cache");
	tapwheel_core::run::clear_leftovers(&engines);
	if !smoke {
		tapwheel_core::run::clear_scratch(&engines);
	}
	let packaged = system::packaged();
	let hooks = if smoke { None } else { Some(Hooks::install(main)) };
	let hooks_ok = hooks.as_ref().is_none_or(Hooks::ok);

	let now = Instant::now();
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
		packaged,
		jobs: Jobs::new(engines),
		wheel: WheelState::new(),
		hud: HudState {
			visible: false,
			hover: None,
			ids: Vec::new(),
			scale: 1.0,
			anchor: cursor(),
			start: now,
			tracking: false,
			anim: HashMap::new(),
			expand: HashMap::new(),
			cards: Vec::new(),
			tick: now,
		},
		web: Web::new(local.join("WebView")),
		last: None,
		pending_pick: Vec::new(),
		taskbar_created: unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) },
	};
	APP.with(|a| *a.borrow_mut() = Some(app));

	if smoke {
		return smoke_test();
	}

	with(|a| {
		a.add_tray();
		a.apply_no_drag();
		if !a.settings.welcomed {
			a.settings.welcomed = true;
			a.save();
			shell::set_autostart(a.settings.start_with_windows, a.packaged);
			if !a.packaged {
				shell::set_context_menu(true);
			}
		}
		if !hooks_ok {
			a.jobs.problem(
				Op::Copy,
				None,
				&[],
				"Windows blocked the key watcher; restart TapWheel".into(),
			);
			a.refresh_hud();
		}
		if !picked.is_empty() {
			a.message(&request);
		} else if !background {
			a.show_settings();
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
		let photo = exe_dir().join("photo.heic");
		let _ = std::fs::write(&photo, HEIC_SAMPLE);
		let files = vec![photo];
		a.open(Mode::Pick, Wheel::Convert, files.clone());
		let drew = a.wheel.ops.len() >= 6;
		a.tap(1);
		let tools = a.wheel.wheel == Wheel::Tools && a.wheel.ops.len() > 8;
		a.turn_page(1);
		let paged = a.wheel.page == 1;
		a.close(None);
		a.finish_close();
		a.jobs.note(Op::CopyPath, None, &files, "Copied the path".into());
		a.refresh_hud();
		let hud = a.hud.visible;
		let web = super::web::smoke(a);
		let line = format!(
			"wheel={drew} tools={tools} paged={paged} hud={hud} web={web} theme={:?}\n",
			a.theme
		);
		let _ = std::fs::write(std::env::temp_dir().join("tapwheel-smoke.txt"), line);
		drew && tools && paged && hud
	})
	.unwrap_or(false);
	if ok { 0 } else { 1 }
}
