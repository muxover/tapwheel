//! Editor and settings windows: a native window holding a WebView2, which shows the pages
//! bundled in `web/`. Pages talk to the app through small JSON messages; the app answers by
//! calling `TW.receive(...)` in the page.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use include_dir::{Dir, include_dir};
use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};
use serde_json::{Value, json};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use wry::http::{Request, Response, StatusCode, header};
use wry::{WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows};

use super::WM_APP_WEB;
use super::app::{App, with};
use super::util::*;

static PAGES: Dir = include_dir!("$CARGO_MANIFEST_DIR/web");
const CLASS: &str = "TapWheel.Web";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
	Settings,
	Editor,
}

pub struct WebWin {
	pub hwnd: HWND,
	view: WebView,
	pub page: Page,
	/// What the page needs first; sent when it says it's ready.
	pub init: Value,
	pub inputs: Vec<PathBuf>,
	/// Set once the page has its first message; anything sent before then waits in `held`, since
	/// a script run before the page has loaded is simply lost.
	ready: bool,
	held: Vec<Value>,
}

/// Messages between the WebView threads and the UI thread.
#[derive(Default)]
pub struct Mailbox {
	/// From pages: (window, JSON text).
	pub inbox: VecDeque<(usize, String)>,
	/// To pages: (window, JSON value), from worker threads.
	pub outbox: VecDeque<(usize, Value)>,
}

pub struct Web {
	context: WebContext,
	pub windows: Vec<WebWin>,
	/// Files pages may read, by token.
	pub files: Arc<Mutex<HashMap<String, PathBuf>>>,
	pub mail: Arc<Mutex<Mailbox>>,
	main: usize,
	pub scratch: PathBuf,
}

impl Web {
	pub fn new(data: PathBuf) -> Web {
		let _ = std::fs::create_dir_all(&data);
		Web {
			context: WebContext::new(Some(data)),
			windows: Vec::new(),
			files: Arc::new(Mutex::new(HashMap::new())),
			mail: Arc::new(Mutex::new(Mailbox::default())),
			main: 0,
			scratch: std::env::temp_dir().join("TapWheel").join("web"),
		}
	}

	pub fn get(&self, hwnd: HWND) -> Option<&WebWin> {
		self.windows.iter().find(|w| w.hwnd == hwnd)
	}

	/// Sends a message to a page. The first is always `init`, answering the page's "ready";
	/// anything that comes earlier (a fast preview, say) is held and follows it in order.
	pub fn send(&mut self, hwnd: HWND, msg: &Value) {
		let Some(w) = self.windows.iter_mut().find(|w| w.hwnd == hwnd) else {
			return;
		};
		if msg["type"] == "init" {
			w.ready = true;
			let _ = w.view.evaluate_script(&format!("window.TW && TW.receive({msg})"));
			for m in std::mem::take(&mut w.held) {
				let _ = w.view.evaluate_script(&format!("window.TW && TW.receive({m})"));
			}
		} else if w.ready {
			let _ = w.view.evaluate_script(&format!("window.TW && TW.receive({msg})"));
		} else {
			w.held.push(msg.clone());
		}
	}

	/// A handle worker threads use to send to a page later.
	pub fn later(&self, hwnd: HWND) -> impl Fn(Value) + Send + 'static {
		let mail = self.mail.clone();
		let main = self.main;
		let hwnd = hwnd as usize;
		move |v| {
			mail.lock().unwrap().outbox.push_back((hwnd, v));
			unsafe { PostMessageW(main as HWND, WM_APP_WEB, 0, 0) };
		}
	}
}

/// Percent-encodes a file name for the last part of a page URL.
pub fn urlencode(s: &str) -> String {
	s.bytes()
		.map(|b| match b {
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
			_ => format!("%{b:02X}"),
		})
		.collect()
}

struct Handle(HWND);

impl HasWindowHandle for Handle {
	fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
		let nz = std::num::NonZeroIsize::new(self.0 as isize).ok_or(HandleError::Unavailable)?;
		let raw = RawWindowHandle::Win32(Win32WindowHandle::new(nz));
		Ok(unsafe { WindowHandle::borrow_raw(raw) })
	}
}

fn mime(path: &str) -> &'static str {
	match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
		"html" => "text/html; charset=utf-8",
		"css" => "text/css; charset=utf-8",
		"js" | "mjs" => "text/javascript; charset=utf-8",
		"json" => "application/json",
		"svg" => "image/svg+xml",
		"png" => "image/png",
		"jpg" | "jpeg" | "jfif" => "image/jpeg",
		"webp" => "image/webp",
		"avif" => "image/avif",
		"gif" => "image/gif",
		"bmp" => "image/bmp",
		"ico" => "image/x-icon",
		"mp4" | "m4v" => "video/mp4",
		"mov" => "video/quicktime",
		"webm" => "video/webm",
		"mkv" => "video/x-matroska",
		"mp3" => "audio/mpeg",
		"m4a" | "aac" => "audio/mp4",
		"wav" => "audio/wav",
		"flac" => "audio/flac",
		"ogg" | "oga" => "audio/ogg",
		"opus" => "audio/ogg",
		"woff2" => "font/woff2",
		"ttf" => "font/ttf",
		_ => "application/octet-stream",
	}
}

fn respond(status: StatusCode, ctype: &str, body: Vec<u8>) -> Response<Cow<'static, [u8]>> {
	Response::builder()
		.status(status)
		.header(header::CONTENT_TYPE, ctype)
		.header(header::CACHE_CONTROL, "no-store")
		.body(Cow::Owned(body))
		.expect("valid response")
}

/// Serves pages, shared files (with byte ranges, so video can seek) and saved renders.
fn serve(
	files: &Mutex<HashMap<String, PathBuf>>,
	scratch: &std::path::Path,
	req: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
	let path = req.uri().path().trim_start_matches('/').to_string();
	if req.method() == "POST" && path == "save" {
		let ext = req
			.uri()
			.query()
			.and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("ext=")))
			.filter(|e| e.chars().all(|c| c.is_ascii_alphanumeric()) && e.len() <= 5)
			.unwrap_or("png")
			.to_string();
		let _ = std::fs::create_dir_all(scratch);
		static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
		let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let out = scratch.join(format!("render-{}-{n}.{ext}", std::process::id()));
		return match std::fs::write(&out, req.body()) {
			Ok(()) => respond(
				StatusCode::OK,
				"application/json",
				json!({ "path": out }).to_string().into_bytes(),
			),
			Err(e) => respond(
				StatusCode::INTERNAL_SERVER_ERROR,
				"text/plain",
				e.to_string().into_bytes(),
			),
		};
	}
	if let Some(rest) = path.strip_prefix("file/") {
		let token = rest.split('/').next().unwrap_or("");
		let Some(file) = files.lock().unwrap().get(token).cloned() else {
			return respond(StatusCode::NOT_FOUND, "text/plain", Vec::new());
		};
		let ctype = mime(&file.to_string_lossy());
		let Ok(meta) = std::fs::metadata(&file) else {
			return respond(StatusCode::NOT_FOUND, "text/plain", Vec::new());
		};
		let len = meta.len();
		let range = req
			.headers()
			.get(header::RANGE)
			.and_then(|v| v.to_str().ok())
			.and_then(|v| v.strip_prefix("bytes="))
			.and_then(|v| v.split_once('-'))
			.map(|(a, b)| {
				let start: u64 = a.parse().unwrap_or(0);
				// chunks of at most 4 MB keep memory flat for big videos
				let end: u64 = b
					.parse()
					.unwrap_or(len.saturating_sub(1))
					.min(start + 4 * 1024 * 1024 - 1)
					.min(len.saturating_sub(1));
				(start, end)
			});
		use std::io::{Read, Seek, SeekFrom};
		let Ok(mut f) = std::fs::File::open(&file) else {
			return respond(StatusCode::NOT_FOUND, "text/plain", Vec::new());
		};
		return match range {
			Some((start, end)) if start <= end && start < len => {
				let mut buf = vec![0u8; (end - start + 1) as usize];
				if f.seek(SeekFrom::Start(start)).is_err() || f.read_exact(&mut buf).is_err() {
					return respond(StatusCode::INTERNAL_SERVER_ERROR, "text/plain", Vec::new());
				}
				Response::builder()
					.status(StatusCode::PARTIAL_CONTENT)
					.header(header::CONTENT_TYPE, ctype)
					.header(header::ACCEPT_RANGES, "bytes")
					.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{len}"))
					.body(Cow::Owned(buf))
					.expect("valid response")
			}
			_ => {
				let mut buf = Vec::with_capacity(len as usize);
				let _ = f.read_to_end(&mut buf);
				Response::builder()
					.status(StatusCode::OK)
					.header(header::CONTENT_TYPE, ctype)
					.header(header::ACCEPT_RANGES, "bytes")
					.body(Cow::Owned(buf))
					.expect("valid response")
			}
		};
	}
	let path = if path.is_empty() {
		"index.html".to_string()
	} else {
		path
	};
	match PAGES.get_file(&path) {
		Some(f) => respond(StatusCode::OK, mime(&path), f.contents().to_vec()),
		None => respond(StatusCode::NOT_FOUND, "text/plain", b"not found".to_vec()),
	}
}

pub fn register_class(inst: HINSTANCE, icon: HICON) {
	super::app::register(inst, CLASS, Some(proc_), icon, true);
}

fn theme_json(a: &App) -> Value {
	let c = a.theme.accent;
	let hex = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
	json!({
		"dark": a.theme.dark,
		"win11": a.theme.win11,
		"highContrast": a.theme.high_contrast,
		"accent": format!("#{:02x}{:02x}{:02x}", hex(c.r), hex(c.g), hex(c.b)),
	})
}

fn style_window(a: &App, hwnd: HWND) {
	let dark: u32 = a.theme.dark as u32;
	unsafe {
		// DWMWA_USE_IMMERSIVE_DARK_MODE
		DwmSetWindowAttribute(hwnd, 20, &dark as *const u32 as *const _, 4);
		if a.theme.win11 && !a.theme.high_contrast {
			// DWMWA_SYSTEMBACKDROP_TYPE: Mica behind the page
			let mica: u32 = 2;
			DwmSetWindowAttribute(hwnd, 38, &mica as *const u32 as *const _, 4);
		}
	}
}

fn fit(hwnd: HWND, view: &WebView) {
	let mut r: RECT = unsafe { std::mem::zeroed() };
	unsafe { GetClientRect(hwnd, &mut r) };
	let _ = view.set_bounds(wry::Rect {
		position: wry::dpi::PhysicalPosition::new(0, 0).into(),
		size: wry::dpi::PhysicalSize::new((r.right - r.left) as u32, (r.bottom - r.top) as u32).into(),
	});
}

/// Opens a page window. Settings has one window; each editor gets its own.
pub fn open(a: &mut App, page: Page, init: Value) -> Option<HWND> {
	open_sized(a, page, init, Vec::new(), (920.0, 680.0))
}

pub fn open_sized(a: &mut App, page: Page, mut init: Value, inputs: Vec<PathBuf>, size: (f32, f32)) -> Option<HWND> {
	if page == Page::Settings
		&& let Some(w) = a.web.windows.iter().find(|w| w.page == Page::Settings)
	{
		unsafe {
			ShowWindow(w.hwnd, SW_SHOWNORMAL);
			SetForegroundWindow(w.hwnd);
		}
		return Some(w.hwnd);
	}
	a.web.main = a.main as usize;
	init["theme"] = theme_json(a);
	init["page"] = json!(match page {
		Page::Settings => "settings",
		Page::Editor => "editor",
	});
	let pt = cursor();
	let (work, scale) = monitor_at(pt);
	let style = WS_OVERLAPPEDWINDOW;
	let mut r = RECT {
		left: 0,
		top: 0,
		right: (size.0 * scale) as i32,
		bottom: (size.1 * scale) as i32,
	};
	unsafe { AdjustWindowRectExForDpi(&mut r, style, 0, 0, (scale * 96.0) as u32) };
	let (w, h) = (
		(r.right - r.left).min(work.right - work.left),
		(r.bottom - r.top).min(work.bottom - work.top),
	);
	let x = work.left + ((work.right - work.left) - w) / 2;
	let y = work.top + ((work.bottom - work.top) - h) / 2;
	let title = init["title"].as_str().unwrap_or("TapWheel").to_string();
	let hwnd = unsafe {
		CreateWindowExW(
			0,
			wide(CLASS).as_ptr(),
			wide(&title).as_ptr(),
			style,
			x,
			y.max(work.top),
			w,
			h,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			a.inst,
			std::ptr::null(),
		)
	};
	if hwnd.is_null() {
		return None;
	}
	style_window(a, hwnd);
	let files = a.web.files.clone();
	let scratch = a.web.scratch.clone();
	let mail = a.web.mail.clone();
	let main = a.main as usize;
	let id = hwnd as usize;
	let dark = a.theme.dark;
	let built = WebViewBuilder::new_with_web_context(&mut a.web.context)
		.with_url("tapwheel://localhost/index.html")
		// opaque, in the page's own colour, so no white shows while it loads or resizes
		.with_background_color(if dark { (32, 32, 32, 255) } else { (243, 243, 243, 255) })
		.with_devtools(cfg!(debug_assertions))
		.with_default_context_menus(cfg!(debug_assertions))
		.with_browser_accelerator_keys(false)
		.with_theme(if dark { wry::Theme::Dark } else { wry::Theme::Light })
		.with_custom_protocol("tapwheel".into(), move |_, req| serve(&files, &scratch, req))
		.with_ipc_handler(move |req| {
			mail.lock().unwrap().inbox.push_back((id, req.into_body()));
			unsafe { PostMessageW(main as HWND, WM_APP_WEB, 0, 0) };
		})
		.with_navigation_handler(|url| url.starts_with("tapwheel:") || url.starts_with("http://tapwheel.localhost"))
		.with_new_window_req_handler(|url, _| {
			// links in pages open in the browser instead of a new WebView
			if url.starts_with("https://") {
				super::shell::open_url(&url);
			}
			wry::NewWindowResponse::Deny
		})
		.build_as_child(&Handle(hwnd));
	let view = match built {
		Ok(v) => v,
		Err(e) => {
			unsafe { DestroyWindow(hwnd) };
			a.jobs.problem(
				tapwheel_core::Op::Crop,
				None,
				&[],
				format!("This window needs Microsoft Edge WebView2, which Windows didn't load: {e}"),
			);
			a.refresh_hud();
			return None;
		}
	};
	fit(hwnd, &view);
	a.web.windows.push(WebWin {
		hwnd,
		view,
		page,
		init,
		inputs,
		ready: false,
		held: Vec::new(),
	});
	unsafe {
		ShowWindow(hwnd, SW_SHOWNORMAL);
		SetForegroundWindow(hwnd);
	}
	Some(hwnd)
}

pub fn close(a: &mut App, hwnd: HWND) {
	a.web.windows.retain(|w| w.hwnd != hwnd);
	unsafe { DestroyWindow(hwnd) };
}

pub fn theme_changed(a: &mut App) {
	let theme = theme_json(a);
	let hwnds: Vec<HWND> = a.web.windows.iter().map(|w| w.hwnd).collect();
	for h in hwnds {
		style_window(a, h);
		a.web.send(h, &json!({ "type": "theme", "theme": theme }));
	}
}

/// Delivers queued messages in both directions. Runs on the UI thread.
pub fn pump(a: &mut App) {
	loop {
		let next = {
			let mut m = a.web.mail.lock().unwrap();
			match m.outbox.pop_front() {
				Some(out) => Some(Err(out)),
				None => m.inbox.pop_front().map(Ok),
			}
		};
		match next {
			Some(Ok((hwnd, text))) => {
				let Ok(msg) = serde_json::from_str::<Value>(&text) else {
					continue;
				};
				a.web_message(hwnd as HWND, msg);
			}
			Some(Err((hwnd, v))) => a.web.send(hwnd as HWND, &v),
			None => break,
		}
	}
}

/// Opens the settings page hidden, for the start-up check in CI.
pub fn smoke(a: &mut App) -> bool {
	let init = a.settings_payload();
	let ok = open(a, Page::Settings, init).is_some();
	let hwnds: Vec<HWND> = a.web.windows.iter().map(|w| w.hwnd).collect();
	for h in hwnds {
		close(a, h);
	}
	ok
}

unsafe extern "system" fn proc_(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
	let handled = match msg {
		WM_SIZE => with(|a| {
			if let Some(win) = a.web.get(hwnd) {
				fit(hwnd, &win.view);
			}
		}),
		WM_DPICHANGED => with(|a| unsafe {
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
			if let Some(win) = a.web.get(hwnd) {
				fit(hwnd, &win.view);
			}
			let _ = GetDpiForWindow(hwnd);
		}),
		WM_SETFOCUS => with(|a| {
			if let Some(win) = a.web.get(hwnd) {
				let _ = win.view.focus();
			}
		}),
		WM_CLOSE => with(|a| close(a, hwnd)),
		_ => None,
	};
	match handled {
		Some(()) => 0,
		None => unsafe { DefWindowProcW(hwnd, msg, w, l) },
	}
}
