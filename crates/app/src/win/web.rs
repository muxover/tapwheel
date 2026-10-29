//! Editor and settings windows: a native window holding a WebView2, which shows the pages
//! bundled in `web/`. Pages talk to the app through small JSON messages; the app answers by
//! calling `TW.receive(...)` in the page.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use include_dir::{Dir, include_dir};
use raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle};
use serde_json::{Value, json};
use webview2_com::Microsoft::Web::WebView2::Win32::{COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_ALLOW, ICoreWebView2_3};
use windows::core::{HSTRING, Interface};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use wry::http::{Request, Response, StatusCode, header};
use wry::{WebContext, WebView, WebViewBuilder, WebViewBuilderExtWindows, WebViewExtWindows};

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
	pub hosts: Hosts,
	/// Files made for this window (frame strips, frames, clips), deleted when it closes.
	pub temps: Arc<Mutex<Vec<PathBuf>>>,
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

/// Folders a page may read, each under a host name of its own. WebView2 serves their files
/// itself, straight from disk and by byte ranges, so a video of any size never passes through
/// TapWheel's memory or its UI thread.
pub type Hosts = Vec<(String, PathBuf)>;

pub fn hosts_for(folders: impl IntoIterator<Item = PathBuf>) -> Hosts {
	let mut hosts: Hosts = Vec::new();
	for f in folders {
		if !hosts.iter().any(|(_, g)| *g == f) {
			hosts.push((format!("f{}.tapwheel.example", hosts.len()), f));
		}
	}
	hosts
}

fn map_hosts(view: &WebView, hosts: &Hosts) {
	let mapped = view.webview().cast::<ICoreWebView2_3>().and_then(|wv| {
		for (host, folder) in hosts {
			unsafe {
				wv.SetVirtualHostNameToFolderMapping(
					&HSTRING::from(host.as_str()),
					&HSTRING::from(folder.as_os_str()),
					// as if the files came with Access-Control-Allow-Origin: *, so pictures drawn on
					// a canvas can still be read back
					COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_ALLOW,
				)?;
			}
		}
		Ok(())
	});
	if let Err(e) = mapped {
		super::log::write(&format!("pages can't read files from their folders: {e}"));
	}
}

/// The address a page reads a file at: under its folder's host when it has one, otherwise
/// through TapWheel, a chunk at a time.
pub fn url_for(hosts: &Hosts, files: &Mutex<HashMap<String, PathBuf>>, path: &Path) -> String {
	for (host, folder) in hosts {
		if let Ok(rel) = path.strip_prefix(folder) {
			let parts: Vec<String> = rel
				.components()
				.map(|c| urlencode(&c.as_os_str().to_string_lossy()))
				.collect();
			return format!("http://{host}/{}", parts.join("/"));
		}
	}
	static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1_000_000);
	let token = format!("p{}", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
	files.lock().unwrap().insert(token.clone(), path.to_path_buf());
	let name = path
		.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default();
	// a same-site path: WebView2 serves the pages from http://tapwheel.localhost, where a
	// tapwheel:// address would never load
	format!("/file/{token}/{}", urlencode(&name))
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
		let range = req.headers().get(header::RANGE).and_then(|v| v.to_str().ok());
		return match crate::served::read(&file, range) {
			Ok(part) => {
				let mut r = Response::builder()
					.status(part.status)
					.header(header::CONTENT_TYPE, mime(&file.to_string_lossy()))
					.header(header::ACCEPT_RANGES, "bytes");
				if let Some(cr) = part.content_range {
					r = r.header(header::CONTENT_RANGE, cr);
				}
				r.body(Cow::Owned(part.body)).expect("valid response")
			}
			Err(_) => respond(StatusCode::NOT_FOUND, "text/plain", Vec::new()),
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
		// the mark in the current accent; the window class only has the packaged one
		if !a.themed_icon.is_null() {
			SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, a.themed_icon as isize);
			SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, a.themed_icon as isize);
		}
		// DWMWA_USE_IMMERSIVE_DARK_MODE: 20 since Windows 10 2004, 19 on the builds before it
		let attribute = if super::system::build_number() >= 18985 { 20 } else { 19 };
		DwmSetWindowAttribute(hwnd, attribute, &dark as *const u32 as *const _, 4);
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
	let mail = a.web.mail.clone();
	let main = a.main as usize;
	let id = hwnd as usize;
	let hosts = match page {
		Page::Editor => hosts_for(
			inputs
				.iter()
				.filter_map(|f| f.parent().map(Path::to_path_buf))
				.chain([a.jobs.engines.scratch.clone(), a.jobs.engines.cache.clone()]),
		),
		Page::Settings => Vec::new(),
	};
	let built = build(
		&mut a.web.context,
		hwnd,
		a.theme.dark,
		a.web.files.clone(),
		a.web.scratch.clone(),
		&hosts,
		move |body| {
			mail.lock().unwrap().inbox.push_back((id, body));
			unsafe { PostMessageW(main as HWND, WM_APP_WEB, 0, 0) };
		},
	);
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
		hosts,
		temps: Arc::default(),
		ready: false,
		held: Vec::new(),
	});
	unsafe {
		ShowWindow(hwnd, SW_SHOWNORMAL);
		SetForegroundWindow(hwnd);
	}
	Some(hwnd)
}

/// A page's WebView in `parent`, reading pages from TapWheel and shared files from `hosts`.
fn build(
	context: &mut WebContext,
	parent: HWND,
	dark: bool,
	files: Arc<Mutex<HashMap<String, PathBuf>>>,
	scratch: PathBuf,
	hosts: &Hosts,
	ipc: impl Fn(String) + 'static,
) -> wry::Result<WebView> {
	let view = WebViewBuilder::new_with_web_context(context)
		.with_url("tapwheel://localhost/index.html")
		// opaque, in the page's own colour, so no white shows while it loads or resizes
		.with_background_color(if dark { (32, 32, 32, 255) } else { (243, 243, 243, 255) })
		.with_devtools(cfg!(debug_assertions))
		.with_default_context_menus(cfg!(debug_assertions))
		.with_browser_accelerator_keys(false)
		.with_theme(if dark { wry::Theme::Dark } else { wry::Theme::Light })
		.with_asynchronous_custom_protocol("tapwheel".into(), move |_, req, responder| {
			let (files, scratch) = (files.clone(), scratch.clone());
			// files are read on a thread of their own, never on the UI thread
			let spawned = std::thread::Builder::new()
				.name("serve".into())
				.spawn(move || responder.respond(serve(&files, &scratch, req)));
			if let Err(e) = spawned {
				super::log::write(&format!("a page's file couldn't be read: {e}"));
			}
		})
		.with_ipc_handler(move |req| ipc(req.into_body()))
		.with_navigation_handler(|url| url.starts_with("tapwheel:") || url.starts_with("http://tapwheel.localhost"))
		.with_new_window_req_handler(|url, _| {
			// links in pages open in the browser instead of a new WebView
			if url.starts_with("https://") {
				super::shell::open_url(&url);
			}
			wry::NewWindowResponse::Deny
		})
		.build_as_child(&Handle(parent))?;
	map_hosts(&view, hosts);
	super::crash::keep();
	Ok(view)
}

pub fn close(a: &mut App, hwnd: HWND) {
	for w in a.web.windows.iter().filter(|w| w.hwnd == hwnd) {
		for f in w.temps.lock().unwrap().drain(..) {
			let _ = std::fs::remove_file(f);
		}
	}
	a.web.windows.retain(|w| w.hwnd != hwnd);
	unsafe { DestroyWindow(hwnd) };
	a.quiet_later();
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

#[cfg(test)]
mod tests {
	use std::cell::RefCell;
	use std::os::windows::io::AsRawHandle;
	use std::rc::Rc;
	use std::time::{Duration, Instant};

	use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
	use windows_sys::Win32::System::IO::DeviceIoControl;
	use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;
	use windows_sys::Win32::System::ProcessStatus::{
		GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
	};
	use windows_sys::Win32::System::Threading::GetCurrentProcess;

	use super::*;

	const MB: u64 = 1024 * 1024;

	fn private_mb() -> u64 {
		let mut c: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
		c.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
		unsafe {
			GetProcessMemoryInfo(
				GetCurrentProcess(),
				&mut c as *mut PROCESS_MEMORY_COUNTERS_EX as *mut PROCESS_MEMORY_COUNTERS,
				c.cb,
			)
		};
		c.PrivateUsage as u64 / MB
	}

	/// A file that says it's `len` bytes and takes no space.
	fn sparse(path: &Path, len: u64) {
		let f = std::fs::File::create(path).unwrap();
		let mut ret = 0;
		let ok = unsafe {
			DeviceIoControl(
				f.as_raw_handle() as HANDLE,
				FSCTL_SET_SPARSE,
				std::ptr::null(),
				0,
				std::ptr::null_mut(),
				0,
				&mut ret,
				std::ptr::null_mut(),
			)
		};
		assert_ne!(ok, 0, "couldn't make the file sparse");
		f.set_len(len).unwrap();
	}

	/// Runs this thread's messages, as the UI thread would, until `done` or the time runs out.
	fn pump(limit: Duration, mut each: impl FnMut() -> bool) {
		let end = Instant::now() + limit;
		let mut msg: MSG = unsafe { std::mem::zeroed() };
		while Instant::now() < end {
			while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
				unsafe {
					TranslateMessage(&msg);
					DispatchMessageW(&msg);
				}
			}
			if each() {
				return;
			}
			std::thread::sleep(Duration::from_millis(10));
		}
	}

	const PROBE: &str = r#"(async () => {
		const say = (m) => window.ipc.postMessage(JSON.stringify({ probe: m }));
		const video = document.createElement("video");
		video.preload = "auto";
		video.src = HOST;
		document.body.append(video);
		for (const url of [HOST, TOKEN]) {
			try {
				const r = await fetch(url);
				const reader = r.body.getReader();
				let n = 0;
				while (n < 64 * 1024 * 1024) {
					const { done, value } = await reader.read();
					if (done) break;
					n += value.length;
				}
				await reader.cancel();
				say(`${r.status} ${n}`);
			} catch (e) {
				say(`error ${e}`);
			}
		}
		say("done");
	})();"#;

	#[test]
	fn a_video_over_4_gb_loads_without_tapwheel_holding_it() {
		let dir = std::env::temp_dir().join(format!("tw-web-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&dir);
		std::fs::create_dir_all(&dir).unwrap();
		let video = dir.join("big video.mp4");
		sparse(&video, 5 * 1024 * MB);
		let parent = unsafe {
			CreateWindowExW(
				0,
				wide("STATIC").as_ptr(),
				std::ptr::null(),
				WS_OVERLAPPEDWINDOW,
				0,
				0,
				800,
				600,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
				std::ptr::null_mut(),
				std::ptr::null(),
			)
		};
		assert!(!parent.is_null());
		let files = Arc::new(Mutex::new(HashMap::new()));
		let hosts = hosts_for([dir.clone()]);
		let host_url = url_for(&hosts, &files, &video);
		let token_url = url_for(&Vec::new(), &files, &video);
		let heard = Rc::new(RefCell::new(Vec::<Value>::new()));
		let ear = heard.clone();
		unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
		let mut context = WebContext::new(Some(dir.join("profile")));
		let view = build(
			&mut context,
			parent,
			false,
			files,
			dir.join("scratch"),
			&hosts,
			move |body| {
				if let Ok(v) = serde_json::from_str(&body) {
					ear.borrow_mut().push(v);
				}
			},
		)
		.expect("WebView2 didn't start");
		// the first webview is when WebView2 would put its own crash filter in
		assert!(
			super::super::crash::ours(),
			"the crash filter isn't ours after making a webview"
		);

		pump(Duration::from_secs(60), || {
			heard.borrow().iter().any(|m| m["cmd"] == "ready")
		});
		assert!(
			heard.borrow().iter().any(|m| m["cmd"] == "ready"),
			"the page never loaded"
		);
		assert!(
			super::super::crash::ours(),
			"the crash filter isn't ours after the page loaded"
		);
		let script = PROBE
			.replace("HOST", &serde_json::to_string(&host_url).unwrap())
			.replace("TOKEN", &serde_json::to_string(&token_url).unwrap());
		view.evaluate_script(&script).unwrap();
		let mut peak = private_mb();
		pump(Duration::from_secs(90), || {
			peak = peak.max(private_mb());
			heard.borrow().iter().any(|m| m["probe"] == "done")
		});
		let said: Vec<String> = heard
			.borrow()
			.iter()
			.filter_map(|m| m["probe"].as_str().map(str::to_string))
			.collect();
		drop(view);
		unsafe { DestroyWindow(parent) };
		let _ = std::fs::remove_dir_all(&dir);
		assert_eq!(said.last().map(String::as_str), Some("done"), "{said:?}");
		// from its folder's host: read on and on, as a stream
		assert_eq!(said[0], format!("200 {}", 64 * MB), "{said:?}");
		// through TapWheel without a range: one chunk, marked as a range
		assert_eq!(said[1], format!("206 {}", crate::served::CHUNK), "{said:?}");
		assert!(peak < 300, "TapWheel's own memory reached {peak} MB");
	}
}
