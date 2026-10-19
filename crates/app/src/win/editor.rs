use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};
use tapwheel_core::files::{self, RenameRule};
use tapwheel_core::kind::ext_of;
use tapwheel_core::ops::selection_kind;
use tapwheel_core::run::Engines;
use tapwheel_core::{Kind, Op, Recipe, Settings, ff, media};
use windows_sys::Win32::Foundation::HWND;

use super::app::{App, HEIC_SAMPLE, file_name};
use super::web::{self, Page};
use super::{hooks, shell};

/// Links pages may open, so a page can't send the browser anywhere else.
const LINKS: &[&str] = &[
	"https://tapwheel.muxover.com",
	"https://github.com/muxover/tapwheel",
	"https://muxover.com",
	"https://apps.microsoft.com/detail/9MW1GB4106VS",
];

fn size_for(op: Op) -> (f32, f32) {
	match op {
		Op::Protect => (560.0, 460.0),
		Op::Info => (720.0, 640.0),
		Op::Rename => (980.0, 660.0),
		Op::Metadata | Op::Contents => (820.0, 680.0),
		Op::Trim | Op::MakeGif => (1120.0, 740.0),
		_ => (1140.0, 800.0),
	}
}

fn scratch_file(e: &Engines, tag: &str, ext: &str) -> PathBuf {
	static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
	let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let _ = std::fs::create_dir_all(&e.scratch);
	e.scratch.join(format!("{tag}-{}-{n}.{ext}", std::process::id()))
}

impl App {
	pub fn settings_payload(&self) -> Value {
		json!({
			"title": "TapWheel",
			"version": env!("CARGO_PKG_VERSION"),
			"settings": self.settings,
			"paused": hooks::PAUSED.load(std::sync::atomic::Ordering::Relaxed),
			"packaged": self.packaged,
			"voices": tapwheel_core::speech::voices().into_iter().map(|(id, name)| json!({ "id": id, "name": name })).collect::<Vec<_>>(),
		})
	}

	pub fn open_editor(&mut self, op: Op, files: Vec<PathBuf>) {
		let kind = selection_kind(&files);
		let title = match files.as_slice() {
			[one] => format!("{} · {}", op.label(), file_name(one)),
			many => format!("{} · {} files", op.label(), many.len()),
		};
		let list: Vec<Value> = files
			.iter()
			.map(|f| {
				json!({
					"name": file_name(f),
					"path": f,
					"ext": ext_of(f),
					"size": std::fs::metadata(f).map(|m| m.len()).unwrap_or(0),
				})
			})
			.collect();
		let presets: Vec<Value> = op
			.presets(kind)
			.iter()
			.map(|p| json!({ "id": p.id, "label": p.label }))
			.collect();
		let init = json!({
			"title": title,
			"tool": op.id(),
			"label": op.label(),
			"describe": op.describe(kind),
			"kind": format!("{kind:?}").to_lowercase(),
			"files": list,
			"defaults": self.settings.defaults,
			"presets": presets,
		});
		let Some(hwnd) = web::open_sized(self, Page::Editor, init, files.clone(), size_for(op)) else {
			return;
		};
		self.prepare(hwnd, op, kind, files);
	}

	/// Work the editor needs before it can show the files: previews, timelines, details.
	fn prepare(&mut self, hwnd: HWND, op: Op, kind: Option<Kind>, files: Vec<PathBuf>) {
		let send = self.web.later(hwnd);
		let engines = self.jobs.engines.clone();
		let files_map = self.web.files.clone();
		let share = move |path: &Path| -> String {
			static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1_000_000);
			let token = format!("p{}", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
			files_map.lock().unwrap().insert(token.clone(), path.to_path_buf());
			// a same-site path: WebView2 serves the pages from http://tapwheel.localhost, where a
			// tapwheel:// address would never load
			format!("/file/{token}/{}", web::urlencode(&file_name(path)))
		};
		let worker = std::thread::Builder::new().stack_size(tapwheel_core::run::JOB_STACK);
		let spawned = worker.spawn(move || {
			let started = std::time::Instant::now();
			let never = AtomicBool::new(false);
			let mut out = Vec::new();
			for (i, f) in files.iter().enumerate() {
				let mut info = json!({ "index": i });
				let result = tapwheel_core::caught("This file couldn't be opened here", || {
					match (op, kind) {
						(Op::Rename, _) => {}
						(Op::RemoveBackground, _) => {
							// the original, upright, and the AI's cut-out of it at the same size
							let preview = tapwheel_core::image::preview(&engines, f)?;
							info["url"] = json!(share(&preview));
							info["cutout"] = json!(share(&tapwheel_core::bg::cutout_file(&engines, f)?));
						}
						(Op::Info, _) => {
							info["info"] = json!(tapwheel_core::files::info(&engines, f)?);
						}
						(Op::Contents, _) => {
							info["items"] = json!(tapwheel_core::archive::list(f)?);
						}
						(Op::CodeImage, _) => {
							let text = tapwheel_core::text::read_within(f, tapwheel_core::text::DATA_LIMIT)?;
							let ext = ext_of(f);
							let spans = |dark| {
								tapwheel_core::code::highlight(&text, &ext, dark)
									.into_iter()
									.map(|line| {
										line.into_iter()
											.map(|(st, t)| {
												let c = st.foreground;
												json!([format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b), t])
											})
											.collect::<Vec<_>>()
									})
									.collect::<Vec<_>>()
							};
							info["dark"] = json!(spans(true));
							info["light"] = json!(spans(false));
							info["language"] = json!(tapwheel_core::code::language(&ext));
						}
						(Op::Metadata, Some(Kind::Audio)) => {
							let (tags, cover) = ff::tags(&engines.ffprobe, f)?;
							info["tags"] = json!(tags.into_iter().collect::<std::collections::BTreeMap<_, _>>());
							info["hasCover"] = json!(cover);
						}
						(Op::Metadata, _) => {
							let bytes = std::fs::read(f)?;
							info["fields"] = json!(tapwheel_core::metadata::fields(&bytes));
							info["location"] = json!(tapwheel_core::metadata::has_location(&bytes));
							let preview = tapwheel_core::image::preview(&engines, f)?;
							info["url"] = json!(share(&preview));
						}
						(_, Some(Kind::Video | Kind::Audio)) => {
							let p = ff::probe(&engines.ffprobe, f)?;
							info["duration"] = json!(p.duration);
							info["width"] = json!(p.shown_size().0);
							info["height"] = json!(p.shown_size().1);
							info["hasVideo"] = json!(p.has_video());
							info["hasAudio"] = json!(p.has_audio());
							info["fps"] = json!(p.fps);
							// the playable copy, the frame strip and the sound wave each run FFmpeg on
							// their own, so they run side by side
							let (playable, strip, wave) = std::thread::scope(|sc| {
								let playable = sc.spawn(|| {
									if media::plays_in_browser(f, &p) {
										Ok(f.clone())
									} else {
										let copy =
											scratch_file(&engines, "play", if p.has_video() { "mp4" } else { "m4a" });
										media::preview_copy(&engines, f, &p, &copy, &never).map(|_| copy)
									}
								});
								let strip = sc.spawn(|| {
									p.has_video().then(|| {
										let strip = scratch_file(&engines, "strip", "png");
										media::filmstrip(&engines, f, p.duration, 14, 72, &strip)
											.ok()
											.map(|_| strip)
									})
								});
								let wave = sc.spawn(|| p.has_audio().then(|| media::waveform(&engines, f, 900)));
								(playable.join(), strip.join(), wave.join())
							});
							let gone =
								|| tapwheel_core::Error::Engine("getting the clip ready stopped unexpectedly".into());
							info["url"] = json!(share(&playable.map_err(|_| gone())??));
							if let Some(Some(strip)) = strip.map_err(|_| gone())? {
								info["filmstrip"] = json!(share(&strip));
							}
							if let Some(wave) = wave.map_err(|_| gone())? {
								info["wave"] = json!(wave?);
							}
						}
						_ => {
							// pictures: full size, upright, in a format the page can read
							let preview = tapwheel_core::image::preview(&engines, f)?;
							// the size from the file's header; FFmpeg only for unusual files
							let (w, h) = match tapwheel_core::image::header_size(&preview) {
								Some(size) => size,
								None => ff::probe(&engines.ffprobe, &preview)?.shown_size(),
							};
							info["url"] = json!(share(&preview));
							info["width"] = json!(w);
							info["height"] = json!(h);
						}
					}
					Ok(())
				});
				if let Err(e) = result {
					super::log::write(&format!("{op:?} preparing {}: {e}", file_name(f)));
					info["error"] = json!(e.to_string());
				}
				out.push(info);
			}
			let took = started.elapsed().as_millis();
			if took > 500 {
				super::log::write(&format!("slow: {op:?} took {took} ms to get its files ready"));
			}
			send(json!({ "type": "prepared", "files": out }));
		});
		if let Err(e) = spawned {
			super::log::write(&format!("{op:?}: couldn't start getting the files ready: {e}"));
		}
	}

	pub fn web_message(&mut self, hwnd: HWND, msg: Value) {
		let cmd = msg["cmd"].as_str().unwrap_or("");
		match cmd {
			"ready" => {
				if let Some(mut init) = self.web.get(hwnd).map(|w| w.init.clone()) {
					init["type"] = json!("init");
					self.web.send(hwnd, &init);
				}
			}
			"apply" => {
				let Some(w) = self.web.get(hwnd) else { return };
				let mut inputs = w.inputs.clone();
				// Combine lets the order change
				if let Some(order) = msg["order"].as_array() {
					let picked: Vec<PathBuf> = order
						.iter()
						.filter_map(|i| i.as_u64())
						.filter_map(|i| inputs.get(i as usize).cloned())
						.collect();
					if picked.len() == inputs.len() {
						inputs = picked;
					}
				}
				let op = w.init["tool"].as_str().and_then(Op::from_id).unwrap_or(Op::Crop);
				match serde_json::from_value::<Recipe>(msg["recipe"].clone()) {
					Ok(recipe) => {
						if let Some(d) = msg.get("defaults")
							&& let Ok(d) = serde_json::from_value(d.clone())
						{
							// "remember these settings" in an editor
							self.settings.defaults = d;
							self.save();
						}
						self.hud.anchor = super::util::cursor();
						self.run_recipe(op, recipe, inputs);
						if !msg["keep"].as_bool().unwrap_or(false) {
							web::close(self, hwnd);
						}
					}
					Err(e) => self.web.send(
						hwnd,
						&json!({ "type": "error", "message": format!("That didn't work: {e}") }),
					),
				}
			}
			"close" => web::close(self, hwnd),
			// errors inside a page, which would otherwise vanish with it
			"log" => super::log::write(&format!(
				"page: {}",
				msg["text"].as_str().unwrap_or("").chars().take(500).collect::<String>()
			)),
			"renamePreview" | "rename" => {
				let Some(w) = self.web.get(hwnd) else { return };
				let inputs = w.inputs.clone();
				let rule: RenameRule = serde_json::from_value(msg["rule"].clone()).unwrap_or_default();
				let plan = files::rename_plan(&inputs, &rule);
				if cmd == "renamePreview" {
					let items: Vec<Value> = plan
						.iter()
						.map(|r| json!({ "from": file_name(&r.from), "to": file_name(&r.to), "problem": r.problem }))
						.collect();
					self.web.send(hwnd, &json!({ "type": "renamePlan", "items": items }));
					return;
				}
				let kind = selection_kind(&inputs);
				match files::rename_all(&plan) {
					Ok(done) => {
						web::close(self, hwnd);
						let renamed: Vec<PathBuf> = done.iter().map(|(_, to)| to.clone()).collect();
						let text = match done.len() {
							1 => format!("Renamed to {}", file_name(&renamed[0])),
							n => format!("Renamed {n} items"),
						};
						self.jobs.note(Op::Rename, kind, &renamed, text);
						self.refresh_hud();
					}
					Err(e) => self.web.send(
						hwnd,
						&json!({ "type": "error", "message": format!("Nothing was renamed: {e}") }),
					),
				}
			}
			"copyText" => {
				let text = msg["text"].as_str().unwrap_or("");
				let what = msg["what"].as_str().unwrap_or("it");
				let reply = if shell::copy_text(hwnd, text) {
					format!("Copied {what}")
				} else {
					"The clipboard is busy; try again".to_string()
				};
				self.web.send(hwnd, &json!({ "type": "toast", "message": reply }));
			}
			"settings" => {
				if let Ok(s) = serde_json::from_value::<Settings>(msg["settings"].clone()) {
					let autostart = s.start_with_windows != self.settings.start_with_windows;
					// usage counts and recent folders change outside the page; keep ours
					let usage = std::mem::take(&mut self.settings.usage);
					let recent = std::mem::take(&mut self.settings.recent_folders);
					self.settings = s;
					self.settings.usage = usage;
					self.settings.recent_folders = recent;
					if autostart {
						shell::set_autostart(self.settings.start_with_windows, self.packaged);
					}
					self.save();
				}
			}
			"pause" => hooks::PAUSED.store(
				msg["on"].as_bool().unwrap_or(false),
				std::sync::atomic::Ordering::Relaxed,
			),
			"samples" => {
				let png = tapwheel_ui::icon::logo(512).encode_png().unwrap_or_default();
				match shell::samples(&png, HEIC_SAMPLE) {
					Ok(dir) => {
						shell::open(&dir);
					}
					Err(e) => self.web.send(
						hwnd,
						&json!({ "type": "error", "message": format!("Couldn't make the sample files: {e}") }),
					),
				}
			}
			"open" => {
				let url = msg["url"].as_str().unwrap_or("");
				if LINKS.iter().any(|l| url == *l || url.starts_with(&format!("{l}/"))) {
					shell::open_url(url);
				}
			}
			"reveal" => {
				if let Some(p) = msg["path"].as_str() {
					shell::reveal(&[PathBuf::from(p)]);
				}
			}
			"clearUsage" => {
				self.settings.usage.clear();
				self.settings.recent_folders.clear();
				self.save();
			}
			_ => {}
		}
	}
}
