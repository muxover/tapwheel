use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use tapwheel_core::Op;
use tapwheel_core::plan::Job;
use tapwheel_core::run::{Engines, run};
use tapwheel_ui::hud::{self, Button, Card, State};

#[derive(Clone, Debug)]
pub struct Batch {
	pub id: u64,
	pub op: Op,
	pub inputs: Vec<PathBuf>,
	pub total: usize,
	pub done: usize,
	pub progress: Option<f32>,
	pub state: State,
	pub outputs: Vec<PathBuf>,
	pub bytes_in: u64,
	pub bytes_out: u64,
	pub error: Option<String>,
	pub finished: Option<Instant>,
	pub cancel: Arc<AtomicBool>,
	/// A finished note with nothing to undo (Copied, Opened).
	pub note: Option<String>,
}

impl Batch {
	pub fn card(&self) -> Card {
		let single = self.total == 1;
		let name = |p: &PathBuf| {
			p.file_name()
				.map(|n| n.to_string_lossy().into_owned())
				.unwrap_or_default()
		};
		let target = self.op.describe();
		if let Some(note) = &self.note {
			return Card {
				title: note.clone(),
				status: self.inputs.first().map(name).unwrap_or_default(),
				state: State::Done,
				progress: None,
				buttons: vec![Button::Close],
				shown: 1.0,
			};
		}
		let title = match self.state {
			State::Done if single => self.outputs.first().map(name).unwrap_or_default(),
			_ if single => self.inputs.first().map(name).unwrap_or_default(),
			_ => format!("{} files", self.total),
		};
		let status = match self.state {
			State::Running => {
				let pct = self
					.progress
					.map(|p| format!(" · {:.0}%", p * 100.0))
					.unwrap_or_default();
				if single {
					format!("{target}{pct}")
				} else {
					format!("{target} · {} of {}{pct}", (self.done + 1).min(self.total), self.total)
				}
			}
			State::Done => {
				let sizes = hud::size_change(self.bytes_in, self.bytes_out);
				if single {
					sizes
				} else {
					format!("{} done · {sizes}", self.outputs.len())
				}
			}
			State::Failed => self.error.clone().unwrap_or_else(|| "Something went wrong".into()),
			State::Cancelled => "Cancelled, nothing was kept".into(),
		};
		let buttons = match self.state {
			State::Running => vec![Button::Cancel],
			State::Done if self.outputs.len() == 1 => {
				vec![Button::Undo, Button::Show, Button::Open, Button::Close]
			}
			State::Done => vec![Button::Undo, Button::Show, Button::Close],
			_ => vec![Button::Close],
		};
		let progress = if self.state == State::Running {
			match (self.total, self.progress) {
				(1, p) => p,
				(n, p) => Some((self.done as f32 + p.unwrap_or(0.0)) / n as f32),
			}
		} else {
			None
		};
		Card {
			title,
			status,
			state: self.state,
			progress,
			buttons,
			shown: 1.0,
		}
	}
}

/// At most two batches convert at once; the rest wait their turn.
struct Slots {
	free: Mutex<usize>,
	cv: Condvar,
}

pub struct Jobs {
	pub batches: Arc<Mutex<Vec<Batch>>>,
	slots: Arc<Slots>,
	next: u64,
	ffmpeg: PathBuf,
	scratch: PathBuf,
}

impl Jobs {
	pub fn new(ffmpeg: PathBuf, scratch: PathBuf) -> Jobs {
		Jobs {
			batches: Arc::new(Mutex::new(Vec::new())),
			slots: Arc::new(Slots {
				free: Mutex::new(2),
				cv: Condvar::new(),
			}),
			next: 1,
			ffmpeg,
			scratch,
		}
	}

	/// Adds a finished note card, for actions that need no undo.
	pub fn note(&mut self, op: Op, inputs: &[PathBuf], text: String) {
		let id = self.next;
		self.next += 1;
		self.batches.lock().unwrap().push(Batch {
			id,
			op,
			inputs: inputs.to_vec(),
			total: 1,
			done: 1,
			progress: None,
			state: State::Done,
			outputs: Vec::new(),
			bytes_in: 0,
			bytes_out: 0,
			error: None,
			finished: Some(Instant::now()),
			cancel: Arc::new(AtomicBool::new(false)),
			note: Some(text),
		});
	}

	/// Starts converting; `wake` is called from the worker after every change.
	pub fn start(&mut self, op: Op, inputs: Vec<PathBuf>, jobs: Vec<Job>, wake: impl Fn() + Send + 'static) {
		let id = self.next;
		self.next += 1;
		let cancel = Arc::new(AtomicBool::new(false));
		self.batches.lock().unwrap().push(Batch {
			id,
			op,
			inputs,
			total: jobs.len(),
			done: 0,
			progress: None,
			state: State::Running,
			outputs: Vec::new(),
			bytes_in: 0,
			bytes_out: 0,
			error: None,
			finished: None,
			cancel: cancel.clone(),
			note: None,
		});
		let batches = self.batches.clone();
		let slots = self.slots.clone();
		let engines = Engines {
			ffmpeg: self.ffmpeg.clone(),
			scratch: self.scratch.clone(),
		};
		std::thread::spawn(move || {
			{
				let mut free = slots.free.lock().unwrap();
				while *free == 0 {
					free = slots.cv.wait(free).unwrap();
				}
				*free -= 1;
			}
			wake();
			let update = |f: &mut dyn FnMut(&mut Batch)| {
				if let Some(b) = batches.lock().unwrap().iter_mut().find(|b| b.id == id) {
					f(b);
				}
			};
			let mut failures = Vec::new();
			let mut produced: Vec<PathBuf> = Vec::new();
			for job in &jobs {
				if cancel.load(Ordering::Relaxed) {
					break;
				}
				let mut last = Instant::now();
				let result = run(
					job,
					&engines,
					&mut |p| {
						if last.elapsed().as_millis() >= 100 {
							last = Instant::now();
							update(&mut |b| b.progress = Some(p));
							wake();
						}
					},
					&cancel,
				);
				match result {
					Ok(out) => {
						produced.push(out.output.clone());
						update(&mut |b| {
							b.outputs.push(out.output.clone());
							b.bytes_in += out.bytes_in;
							b.bytes_out += out.bytes_out;
						});
					}
					Err(tapwheel_core::Error::Cancelled) => break,
					Err(e) => {
						let name = job
							.inputs()
							.first()
							.and_then(|p| p.file_name())
							.map(|n| n.to_string_lossy().into_owned())
							.unwrap_or_default();
						failures.push(if jobs.len() > 1 {
							format!("{name}: {e}")
						} else {
							capitalize(&e.to_string())
						});
					}
				}
				update(&mut |b| {
					b.done += 1;
					b.progress = None;
				});
				wake();
			}
			let cancelled = cancel.load(Ordering::Relaxed);
			update(&mut |b| {
				b.finished = Some(Instant::now());
				if cancelled {
					b.state = State::Cancelled;
					b.outputs.clear();
				} else if b.outputs.is_empty() {
					b.state = State::Failed;
					b.error = failures.first().cloned();
				} else {
					b.state = State::Done;
					if !failures.is_empty() {
						b.error = Some(format!("{} failed", failures.len()));
					}
				}
			});
			// a cancelled batch keeps nothing, not even the files it finished,
			// and its card may already be gone, so this list is kept here
			for p in if cancelled { produced } else { Vec::new() } {
				let _ = if p.is_dir() {
					std::fs::remove_dir_all(&p)
				} else {
					std::fs::remove_file(&p)
				};
			}
			*slots.free.lock().unwrap() += 1;
			slots.cv.notify_one();
			wake();
		});
	}

	pub fn cards(&self) -> Vec<(u64, Card)> {
		self.batches.lock().unwrap().iter().map(|b| (b.id, b.card())).collect()
	}

	pub fn get(&self, id: u64) -> Option<Batch> {
		self.batches.lock().unwrap().iter().find(|b| b.id == id).cloned()
	}

	pub fn remove(&self, id: u64) {
		self.batches.lock().unwrap().retain(|b| b.id != id);
	}

	pub fn cancel(&self, id: u64) {
		if let Some(b) = self.batches.lock().unwrap().iter().find(|b| b.id == id) {
			b.cancel.store(true, Ordering::Relaxed);
		}
	}

	pub fn any_running(&self) -> bool {
		self.batches.lock().unwrap().iter().any(|b| b.state == State::Running)
	}

	/// Cards whose time is up: finished ones after a while (failures stay longer so they can be
	/// read), and the oldest finished ones when there are too many.
	pub fn expired(&self, hovered: bool) -> Vec<u64> {
		let list = self.batches.lock().unwrap();
		let mut out: Vec<u64> = if hovered {
			Vec::new()
		} else {
			list.iter()
				.filter(|b| match (b.state, b.finished) {
					(State::Running, _) | (_, None) => false,
					(State::Failed, Some(t)) => t.elapsed().as_secs() >= 20,
					(_, Some(t)) if b.note.is_some() => t.elapsed().as_millis() >= 2500,
					(State::Cancelled, Some(t)) => t.elapsed().as_millis() >= 1500,
					(_, Some(t)) => t.elapsed().as_secs() >= 10,
				})
				.map(|b| b.id)
				.collect()
		};
		let excess = list.len().saturating_sub(hud::MAX_CARDS);
		out.extend(
			list.iter()
				.filter(|b| b.state != State::Running)
				.take(excess)
				.map(|b| b.id),
		);
		out
	}
}

fn capitalize(s: &str) -> String {
	let mut c = s.chars();
	match c.next() {
		Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
		None => String::new(),
	}
}
