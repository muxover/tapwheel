use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

use tapwheel_core::plan::Job;
use tapwheel_core::run::{Engines, Stamp, run};
use tapwheel_core::{Defaults, Kind, Op, Recipe};
use tapwheel_ui::hud::{self, Button, Card, State};

#[derive(Clone, Debug)]
pub struct Batch {
	pub id: u64,
	pub op: Op,
	pub kind: Option<Kind>,
	pub inputs: Vec<PathBuf>,
	pub total: usize,
	pub done: usize,
	pub progress: Option<f32>,
	pub state: State,
	pub outputs: Vec<PathBuf>,
	/// How each output looked when it was made, for Undo.
	pub stamps: Vec<Option<Stamp>>,
	pub bytes_in: u64,
	pub bytes_out: u64,
	pub error: Option<String>,
	pub finished: Option<Instant>,
	pub cancel: Arc<AtomicBool>,
	/// A finished note with nothing to undo (Copied, Moved).
	pub note: Option<String>,
	/// Text a Copy button puts on the clipboard, like a checksum.
	pub copy: Option<String>,
	/// What ran, so Retry can run it again on the files that failed.
	pub recipe: Option<Recipe>,
	pub failed: Vec<PathBuf>,
	/// Why a waiting batch hasn't started, when it's something other than another batch.
	pub waiting_for: Option<String>,
}

fn name(p: &Path) -> String {
	p.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default()
}

impl Batch {
	pub fn card(&self) -> Card {
		let single = self.total == 1;
		let icon = self.op.icon(self.kind).to_string();
		if let Some(note) = &self.note {
			let detail = match (&self.copy, self.inputs.as_slice()) {
				(Some(c), _) => c.clone(),
				(None, [one]) => name(one),
				(None, many) if !many.is_empty() => format!("{} items", many.len()),
				_ => String::new(),
			};
			return Card {
				title: note.clone(),
				detail,
				state: if self.state == State::Failed {
					State::Failed
				} else {
					State::Done
				},
				progress: None,
				buttons: if self.copy.is_some() {
					vec![Button::Copy, Button::Close]
				} else {
					vec![Button::Close]
				},
				icon,
				shown: 1.0,
			};
		}
		let doing = self.op.describe(self.kind);
		let title = match self.state {
			State::Waiting | State::Running => doing.clone(),
			State::Done if self.outputs.is_empty() => doing.clone(),
			State::Done if single => self.outputs.first().map(|p| name(p)).unwrap_or_default(),
			State::Done => format!("{} files ready", self.outputs.len()),
			State::Failed => format!("Couldn't {}", lower_first(&doing)),
			State::Cancelled => "Cancelled".into(),
		};
		let detail = match self.state {
			State::Waiting => self
				.waiting_for
				.clone()
				.unwrap_or_else(|| "Waiting for another job to finish".into()),
			State::Running if single => self.inputs.first().map(|p| name(p)).unwrap_or_default(),
			State::Running => format!("{} of {}", (self.done + 1).min(self.total), self.total),
			State::Done if self.outputs.is_empty() => self.error.clone().unwrap_or_default(),
			State::Done => {
				let sizes = hud::size_change(self.bytes_in, self.bytes_out);
				match &self.error {
					Some(e) => format!("{sizes} · {e}"),
					None => sizes,
				}
			}
			State::Failed => self.error.clone().unwrap_or_else(|| "Something went wrong".into()),
			State::Cancelled => "Nothing was kept".into(),
		};
		let retry = self.recipe.is_some() && !self.failed.is_empty();
		let buttons = match self.state {
			State::Waiting | State::Running => vec![Button::Cancel],
			State::Failed if retry => vec![Button::Retry, Button::Close],
			State::Done if retry => vec![Button::Show, Button::Retry, Button::Undo, Button::Close],
			State::Done if self.outputs.len() == 1 => vec![Button::Open, Button::Show, Button::Undo, Button::Close],
			State::Done if !self.outputs.is_empty() => vec![Button::Show, Button::Undo, Button::Close],
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
			detail,
			state: self.state,
			progress,
			buttons,
			icon,
			shown: 1.0,
		}
	}
}

fn lower_first(s: &str) -> String {
	let mut c = s.chars();
	match c.next() {
		Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
		None => String::new(),
	}
}

fn capitalize(s: &str) -> String {
	let mut c = s.chars();
	match c.next() {
		Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
		None => String::new(),
	}
}

/// As many batches run at once as the PC can carry (`Device::slots`); the rest wait their turn.
struct Slots {
	free: Mutex<usize>,
	cv: Condvar,
}

pub struct Jobs {
	pub batches: Arc<Mutex<Vec<Batch>>>,
	slots: Arc<Slots>,
	next: u64,
	pub engines: Engines,
	/// Battery saver is on: heavy batches wait until it's off or they're cancelled.
	pub saver: Arc<AtomicBool>,
}

impl Jobs {
	pub fn new(engines: Engines, slots: usize) -> Jobs {
		Jobs {
			batches: Arc::new(Mutex::new(Vec::new())),
			slots: Arc::new(Slots {
				free: Mutex::new(slots.max(1)),
				cv: Condvar::new(),
			}),
			next: 1,
			engines,
			saver: Arc::new(AtomicBool::new(false)),
		}
	}

	fn push(&mut self, mut b: Batch) -> u64 {
		let id = self.next;
		self.next += 1;
		b.id = id;
		self.batches.lock().unwrap().push(b);
		id
	}

	fn blank(op: Op, kind: Option<Kind>, inputs: &[PathBuf]) -> Batch {
		Batch {
			id: 0,
			op,
			kind,
			inputs: inputs.to_vec(),
			total: 1,
			done: 1,
			progress: None,
			state: State::Done,
			outputs: Vec::new(),
			stamps: Vec::new(),
			bytes_in: 0,
			bytes_out: 0,
			error: None,
			finished: Some(Instant::now()),
			cancel: Arc::new(AtomicBool::new(false)),
			note: None,
			copy: None,
			recipe: None,
			failed: Vec::new(),
			waiting_for: None,
		}
	}

	/// Adds a finished note card, for actions that need no undo.
	pub fn note(&mut self, op: Op, kind: Option<Kind>, inputs: &[PathBuf], text: String) -> u64 {
		let mut b = Self::blank(op, kind, inputs);
		b.note = Some(text);
		self.push(b)
	}

	/// A note that failed, shown in the error colour.
	pub fn problem(&mut self, op: Op, kind: Option<Kind>, inputs: &[PathBuf], text: String) -> u64 {
		super::log::write(&format!("{op:?}: {text}"));
		let mut b = Self::blank(op, kind, inputs);
		b.note = Some(text);
		b.state = State::Failed;
		self.push(b)
	}

	/// A note carrying a value to copy, like a checksum.
	pub fn note_with_copy(
		&mut self,
		op: Op,
		kind: Option<Kind>,
		inputs: &[PathBuf],
		text: String,
		copy: String,
	) -> u64 {
		let mut b = Self::blank(op, kind, inputs);
		b.note = Some(text);
		b.copy = Some(copy);
		self.push(b)
	}

	/// A running card for work the app does itself (checksums), finished later with `finish_note`.
	pub fn begin(&mut self, op: Op, kind: Option<Kind>, inputs: &[PathBuf]) -> (u64, Arc<AtomicBool>) {
		let mut b = Self::blank(op, kind, inputs);
		b.state = State::Running;
		b.done = 0;
		b.finished = None;
		let cancel = b.cancel.clone();
		(self.push(b), cancel)
	}

	pub fn finish_note(batches: &Arc<Mutex<Vec<Batch>>>, id: u64, text: Result<(String, Option<String>), String>) {
		if let Some(b) = batches.lock().unwrap().iter_mut().find(|b| b.id == id) {
			b.finished = Some(Instant::now());
			b.done = 1;
			match text {
				Ok((t, copy)) => {
					b.state = State::Done;
					b.note = Some(t);
					b.copy = copy;
				}
				Err(e) => {
					b.state = if b.cancel.load(Ordering::Relaxed) {
						State::Cancelled
					} else {
						State::Failed
					};
					b.note = Some(e);
				}
			}
		}
	}

	/// Starts the work; `wake` is called from the worker after every change.
	pub fn start(
		&mut self,
		op: Op,
		kind: Option<Kind>,
		inputs: Vec<PathBuf>,
		recipe: Recipe,
		defaults: Defaults,
		wake: impl Fn() + Send + 'static,
	) {
		let mut b = Self::blank(op, kind, &inputs);
		b.state = State::Waiting;
		b.recipe = Some(recipe.clone());
		// one job per file until the plan says otherwise
		b.total = inputs.len();
		b.done = 0;
		b.finished = None;
		let cancel = b.cancel.clone();
		let id = self.push(b);
		let batches = self.batches.clone();
		let slots = self.slots.clone();
		let engines = self.engines.clone();
		let saver = self.saver.clone();
		let worker = std::thread::Builder::new().stack_size(tapwheel_core::run::JOB_STACK);
		let spawned = worker.spawn(move || {
			let update = |f: &mut dyn FnMut(&mut Batch)| {
				if let Some(b) = batches.lock().unwrap().iter_mut().find(|b| b.id == id) {
					f(b);
				}
			};
			let jobs: Vec<Job> = tapwheel_core::plan(op, &recipe, &inputs);
			if jobs.is_empty() {
				super::log::write(&format!("{op:?}: Nothing to do for these files"));
				update(&mut |b| {
					b.note = Some("Nothing to do for these files".into());
					b.state = State::Failed;
					b.finished = Some(Instant::now());
				});
				wake();
				return;
			}
			update(&mut |b| b.total = jobs.len());
			wake();
			if recipe.heavy() && saver.load(Ordering::Relaxed) {
				update(&mut |b| b.waiting_for = Some("Waits while Battery saver is on".into()));
				wake();
				while saver.load(Ordering::Relaxed) && !cancel.load(Ordering::Relaxed) {
					std::thread::sleep(std::time::Duration::from_millis(500));
				}
				update(&mut |b| b.waiting_for = None);
				wake();
			}
			{
				let mut free = slots.free.lock().unwrap();
				while *free == 0 {
					free = slots.cv.wait(free).unwrap();
				}
				*free -= 1;
			}
			update(&mut |b| b.state = State::Running);
			wake();

			let mut failures = Vec::new();
			let mut failed_inputs: Vec<PathBuf> = Vec::new();
			let mut produced: Vec<PathBuf> = Vec::new();
			let mut notes: Vec<String> = Vec::new();
			for job in &jobs {
				if cancel.load(Ordering::Relaxed) {
					break;
				}
				let mut last = Instant::now();
				let result = run(
					job,
					&engines,
					&defaults,
					&mut |p| {
						if last.elapsed().as_millis() >= 80 {
							last = Instant::now();
							update(&mut |b| b.progress = Some(p));
							wake();
						}
					},
					&cancel,
				);
				match result {
					Ok(out) => {
						if let Some(n) = out.note {
							notes.push(n);
						} else {
							notes.extend(out.warning);
							produced.push(out.output.clone());
							update(&mut |b| {
								b.outputs.push(out.output.clone());
								b.stamps.push(out.stamp);
								b.bytes_in += out.bytes_in;
								b.bytes_out += out.bytes_out;
							});
						}
					}
					Err(tapwheel_core::Error::Cancelled) => break,
					Err(e) => {
						let name = job
							.inputs
							.first()
							.and_then(|p| p.file_name())
							.map(|n| n.to_string_lossy().into_owned())
							.unwrap_or_default();
						super::log::write(&format!("{op:?} on {name}: {e}"));
						failed_inputs.extend(job.inputs.iter().cloned());
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
				b.failed = failed_inputs.clone();
				if cancelled {
					b.state = State::Cancelled;
					b.outputs.clear();
				} else if b.outputs.is_empty() && notes.is_empty() {
					b.state = State::Failed;
					b.error = failures.first().cloned();
				} else {
					b.state = State::Done;
					if !notes.is_empty() {
						b.error = Some(notes.join(" · "));
					}
					if !failures.is_empty() {
						let ok = b.done.saturating_sub(failures.len());
						b.error = Some(format!("{ok} done, {} failed", failures.len()));
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
		if let Err(e) = spawned {
			super::log::write(&format!("{op:?}: the job couldn't start: {e}"));
			if let Some(b) = self.batches.lock().unwrap().iter_mut().find(|b| b.id == id) {
				b.state = State::Failed;
				b.error = Some("Windows couldn't start the job; try again".into());
				b.finished = Some(Instant::now());
			}
		}
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
		self.batches
			.lock()
			.unwrap()
			.iter()
			.any(|b| matches!(b.state, State::Running | State::Waiting))
	}

	/// Cards whose time is up: finished ones after a while (failures stay longer so they can be
	/// read), and the oldest finished ones when there are too many.
	pub fn expired(&self, hovered: Option<u64>) -> Vec<u64> {
		let list = self.batches.lock().unwrap();
		let mut out: Vec<u64> = list
			.iter()
			.filter(|b| Some(b.id) != hovered)
			.filter(|b| match (b.state, b.finished) {
				(State::Running, _) | (_, None) => false,
				(State::Failed, Some(t)) => t.elapsed().as_secs() >= 12,
				(_, Some(t)) if b.copy.is_some() => t.elapsed().as_secs() >= 12,
				(_, Some(t)) if b.note.is_some() => t.elapsed().as_millis() >= 3000,
				(State::Cancelled, Some(t)) => t.elapsed().as_millis() >= 1800,
				(_, Some(t)) => t.elapsed().as_secs() >= 8,
			})
			.map(|b| b.id)
			.collect();
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
