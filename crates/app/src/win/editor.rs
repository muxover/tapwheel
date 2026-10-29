use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde_json::{Value, json};
use tapwheel_core::files::{self, RenameRule};
use tapwheel_core::kind::ext_of;
use tapwheel_core::ops::selection_kind;
use tapwheel_core::{Kind, Op, Recipe, Settings, ff, media, recipe_for};
use windows_sys::Win32::Foundation::HWND;

use super::app::{App, HEIC_SAMPLE, file_name};
use super::web::{self, Page};
use super::{hooks, shell};

/// Links pages may open, so a page can't send the browser anywhere else.
const LINKS: &[&str] = &[
	"https://tapwheel.app",
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

/// How much of a video the page plays at a time when it can't play the file itself.
const CLIP_SECS: f64 = 8.0;

/// Every slice each wheel can show, by kind of file, for the Wheels page in Settings.
fn wheel_catalog() -> Value {
	let list: Vec<Value> = tapwheel_core::catalog()
		.into_iter()
		.map(|(wheel, kind, ops)| {
			json!({
				"key": tapwheel_core::settings::layout_key(wheel, Some(kind)),
				"wheel": wheel.id(),
				"kind": kind.id(),
				"slices": ops.iter().map(|op| json!({
					"id": op.id(),
					"label": op.label(),
					"icon": op.icon(Some(kind)),
				})).collect::<Vec<_>>(),
			})
		})
		.collect();
	Value::Array(list)
}

impl App {
	pub fn settings_payload(&self) -> Value {
		json!({
			"title": "TapWheel",
			"version": env!("CARGO_PKG_VERSION"),
			"settings": self.settings,
			"paused": hooks::PAUSED.load(std::sync::atomic::Ordering::Relaxed),
			"packaged": self.packaged,
			"wheels": wheel_catalog(),
			"device": self.device,
			"slots": self.device.slots(),
			"encoder": self.hw_encoder.lock().ok().and_then(|e| e.map(|e| e.name())),
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
			"kind": kind.map(Kind::id),
			"files": list,
			"defaults": self.settings.defaults,
			"output": self.settings.defaults.output,
			"presets": presets,
			// the window offers to keep its settings as a ring choice when they'd mean the same on any file
			"keepable": op.presets(kind).first().and_then(|p| recipe_for(op, Some(p.id), kind, &self.settings.defaults)).is_some_and(|r| r.reusable()),
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
		let hosts = self.web.get(hwnd).map(|w| w.hosts.clone()).unwrap_or_default();
		let share = move |path: &Path| -> String { web::url_for(&hosts, &files_map, path) };
		let temps = self.web.get(hwnd).map(|w| w.temps.clone()).unwrap_or_default();
		// only Trim and Make GIF show a timeline
		let timeline = matches!(op, Op::Trim | Op::MakeGif);
		let worker = std::thread::Builder::new().stack_size(tapwheel_core::run::JOB_STACK);
		let spawned = worker.spawn(move || {
			let started = std::time::Instant::now();
			let mut steps: Vec<(String, u128)> = Vec::new();
			let mut timelines = Vec::new();
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
							// the window needs only the probe to show; the timeline follows
							let t = std::time::Instant::now();
							let p = ff::probe(&engines.ffprobe, f)?;
							steps.push(("probe".into(), t.elapsed().as_millis()));
							info["duration"] = json!(p.duration);
							info["width"] = json!(p.shown_size().0);
							info["height"] = json!(p.shown_size().1);
							info["hasVideo"] = json!(p.has_video());
							info["hasAudio"] = json!(p.has_audio());
							info["fps"] = json!(p.fps);
							if media::plays_in_browser(f, &p) {
								info["url"] = json!(share(f));
							} else {
								// frames and short clips on request, never a copy of the whole file
								info["stills"] = json!(true);
							}
							if timeline {
								info["timeline"] = json!(true);
								timelines.push((i, f.clone(), p));
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
			send(json!({ "type": "prepared", "files": out }));
			steps.push(("window ready".into(), started.elapsed().as_millis()));
			for (i, f, p) in timelines {
				// the frame strip and the sound wave each run FFmpeg on their own, side by side
				std::thread::scope(|sc| {
					let strip = p.has_video().then(|| {
						sc.spawn(|| {
							let t = std::time::Instant::now();
							let strip = engines.scratch_path("strip", "png");
							temps.lock().unwrap().push(strip.clone());
							let made = tapwheel_core::caught("The timeline couldn't be made", || {
								media::filmstrip(&engines, &f, p.duration, 14, 72, &strip)
							});
							(made.map(|()| strip), t.elapsed().as_millis())
						})
					});
					let wave = p.has_audio().then(|| {
						sc.spawn(|| {
							let t = std::time::Instant::now();
							let wave = tapwheel_core::caught("The sound wave couldn't be made", || {
								media::waveform(&engines, &f, p.duration, 900)
							});
							(wave, t.elapsed().as_millis())
						})
					});
					if let Some(Ok((made, ms))) = strip.map(|h| h.join()) {
						steps.push(("filmstrip".into(), ms));
						match made {
							Ok(strip) => send(json!({ "type": "filmstrip", "index": i, "url": share(&strip) })),
							Err(e) => {
								super::log::write(&format!("{op:?} filmstrip for {}: {e}", file_name(&f)));
								send(json!({ "type": "filmstrip", "index": i, "error": e.to_string() }));
							}
						}
					}
					if let Some(Ok((wave, ms))) = wave.map(|h| h.join()) {
						steps.push(("waveform".into(), ms));
						match wave {
							Ok(wave) => send(json!({ "type": "wave", "index": i, "wave": wave })),
							Err(e) => {
								super::log::write(&format!("{op:?} waveform for {}: {e}", file_name(&f)));
								send(json!({ "type": "wave", "index": i, "error": e.to_string() }));
							}
						}
					}
				});
			}
			let names: Vec<String> = files.iter().map(|f| file_name(f)).collect();
			super::log::write(&format!(
				"{op:?} on {}: {}",
				names.join(", "),
				steps
					.iter()
					.map(|(what, ms)| format!("{what} {ms} ms"))
					.collect::<Vec<_>>()
					.join(", ")
			));
		});
		if let Err(e) = spawned {
			super::log::write(&format!("{op:?}: couldn't start getting the files ready: {e}"));
		}
	}

	/// A frame, or a few seconds the page can play, of a video the editor's browser can't play.
	fn preview_part(&mut self, hwnd: HWND, cmd: &str, msg: &Value) {
		let Some(w) = self.web.get(hwnd) else { return };
		let index = msg["index"].as_u64().unwrap_or(0) as usize;
		let Some(input) = w.inputs.get(index).cloned() else {
			return;
		};
		let t = msg["t"].as_f64().unwrap_or(0.0).max(0.0);
		let (hosts, temps, files_map) = (w.hosts.clone(), w.temps.clone(), self.web.files.clone());
		let send = self.web.later(hwnd);
		let engines = self.jobs.engines.clone();
		let clip = cmd == "clip";
		let reply = json!({ "type": cmd, "index": index, "t": t, "seq": msg["seq"] });
		let spawned = std::thread::Builder::new()
			.stack_size(tapwheel_core::run::JOB_STACK)
			.spawn(move || {
				let out = engines.scratch_path(if clip { "clip" } else { "still" }, if clip { "mp4" } else { "jpg" });
				temps.lock().unwrap().push(out.clone());
				let made = tapwheel_core::caught("This part of the video couldn't be shown", || {
					if clip {
						let p = ff::probe(&engines.ffprobe, &input)?;
						media::clip(&engines, &input, &p, t, CLIP_SECS, &out, &AtomicBool::new(false))
					} else {
						media::still(&engines, &input, t, &out)
					}
				});
				let mut reply = reply;
				match made {
					Ok(()) => reply["url"] = json!(web::url_for(&hosts, &files_map, &out)),
					Err(e) => {
						let _ = std::fs::remove_file(&out);
						super::log::write(&format!("preview of {} at {t:.1} s: {e}", file_name(&input)));
						reply["error"] = json!(e.to_string());
					}
				}
				send(reply);
			});
		if let Err(e) = spawned {
			super::log::write(&format!("a video preview couldn't start: {e}"));
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
						if let Some(label) = msg["keep"].as_str() {
							let kind = selection_kind(&inputs);
							if let Err(why) = self.settings.keep_choice(op, kind, label, recipe.clone()) {
								// nothing runs, so the name can be changed and saved again
								self.web.send(hwnd, &json!({ "type": "error", "message": why }));
								return;
							}
							self.save();
							// an open Settings window must not save its older list over the new choice
							let list = json!({ "type": "ownChoices", "list": self.settings.own_choices });
							let open: Vec<HWND> = self
								.web
								.windows
								.iter()
								.filter(|w| w.page == Page::Settings)
								.map(|w| w.hwnd)
								.collect();
							for w in open {
								self.web.send(w, &list);
							}
						}
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
			"still" | "clip" => self.preview_part(hwnd, cmd, &msg),
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
						let text = match done.as_slice() {
							[(_, to)] => format!("Renamed to {}", file_name(to)),
							many => format!("Renamed {} items", many.len()),
						};
						self.jobs.renamed(kind, done, text);
						self.refresh_hud();
					}
					Err(e) => self
						.web
						.send(hwnd, &json!({ "type": "error", "message": e.to_string() })),
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
			// About, Report a problem: a filled-in issue in the browser, posted by the user or not at all
			"report" => {
				let log: Vec<String> = std::fs::read_to_string(super::log::path())
					.unwrap_or_default()
					.lines()
					.rev()
					.take(200)
					.map(str::to_string)
					.collect::<Vec<_>>()
					.into_iter()
					.rev()
					.collect();
				let build = super::system::build_number();
				let windows = format!(
					"Windows {}, build {build}",
					if super::system::is_win11() { 11 } else { 10 }
				);
				let home = std::env::var("USERPROFILE").ok();
				let url = tapwheel_core::report::issue_url(&tapwheel_core::report::Report {
					version: env!("CARGO_PKG_VERSION"),
					windows: &windows,
					edition: if self.packaged { "Store" } else { "Portable" },
					displays: &super::device::displays(),
					log: &log,
					home: home.as_deref(),
				});
				if !shell::open_url(&url) {
					self.web.send(hwnd, &json!({ "type": "error", "message": "The browser didn't open; the report can also be emailed to contact@muxover.com" }));
				}
			}
			// Settings, where results go: one fixed folder
			"pickFolder" => {
				let start = match &self.settings.defaults.output {
					tapwheel_core::plan::OutputRule::Folder { path } => Some(path.clone()),
					_ => shell::documents(),
				};
				if let Some(dir) = super::share::pick_folder(hwnd as isize, start.as_deref()) {
					self.web.send(hwnd, &json!({ "type": "folderPicked", "path": dir }));
				}
			}
			"settings" => {
				if let Ok(s) = serde_json::from_value::<Settings>(msg["settings"].clone()) {
					let autostart = s.start_with_windows != self.settings.start_with_windows;
					let no_drag = s.no_drag != self.settings.no_drag;
					let accent = s.accent != self.settings.accent;
					// usage counts and recent folders change outside the page; keep ours
					let usage = std::mem::take(&mut self.settings.usage);
					let recent = std::mem::take(&mut self.settings.recent_folders);
					self.settings = s;
					self.settings.usage = usage;
					self.settings.recent_folders = recent;
					if autostart {
						shell::set_autostart(self.settings.start_with_windows, self.packaged);
					}
					if accent {
						self.theme_changed();
					}
					if no_drag && !self.apply_no_drag() {
						self.web.send(
							hwnd,
							&json!({ "type": "toast", "message": "Another app already uses Ctrl+Shift+Space" }),
						);
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
