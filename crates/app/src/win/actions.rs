//! The Actions wheel and the tools that finish instantly: they work on the UI thread or a short
//! thread of their own, and report on a card.

use std::path::PathBuf;

use tapwheel_core::kind::{ext_of, kind_of};
use tapwheel_core::ops::selection_kind;
use tapwheel_core::recipe::HashKind;
use tapwheel_core::{Kind, Op};

use super::app::{App, file_name};
use super::jobs::Jobs;
use super::{share, shell};

impl App {
	pub fn action(&mut self, op: Op, preset: Option<&str>, files: &[PathBuf]) {
		let kind = selection_kind(files);
		let plural = |n: usize, one: &str, many: &str| if n == 1 { one.to_string() } else { format!("{n} {many}") };
		match op {
			Op::Copy => {
				if shell::copy_files(self.main, files) {
					self.jobs.note(
						op,
						kind,
						files,
						format!("Copied {}, ready to paste", plural(files.len(), "1 item", "items")),
					);
				} else {
					self.busy_clipboard(op, kind, files);
				}
			}
			Op::CopyPath => {
				let which = preset.unwrap_or("path");
				let text = tapwheel_core::files::clip_text(files, which);
				let what = match which {
					"name" => plural(files.len(), "the name", "names"),
					"stem" => plural(files.len(), "the base name", "base names"),
					"ext" => plural(files.len(), "the extension", "extensions"),
					"uri" => plural(files.len(), "the file URL", "file URLs"),
					"quoted" => plural(files.len(), "the quoted path", "quoted paths"),
					_ => plural(files.len(), "the path", "paths"),
				};
				if shell::copy_text(self.main, &text) {
					self.jobs.note(op, kind, files, format!("Copied {what}"));
				} else {
					self.busy_clipboard(op, kind, files);
				}
			}
			Op::MoveTo => self.move_to(preset, files),
			Op::Share => {
				if let Err(e) = share::share(files) {
					super::log::write(&format!("share: {e}"));
					self.jobs
						.problem(op, kind, files, format!("Sharing didn't open: {}", e.message()));
				}
			}
			Op::Float => {
				super::float::add(self.theme, files);
			}
			Op::Hash => self.checksum(preset, files),
			Op::Properties => {
				if !shell::properties(self.main, &files[0]) {
					self.jobs.problem(op, kind, files, "Properties didn't open".into());
				}
			}
			Op::OpenWith => shell::open_with(&files[0]),
			Op::Terminal => {
				let dir = files
					.iter()
					.find(|p| kind_of(p) == Kind::Folder)
					.cloned()
					.or_else(|| files[0].parent().map(PathBuf::from))
					.unwrap_or_default();
				if shell::terminal(&dir) {
					self.jobs
						.note(op, kind, files, format!("Opened a terminal in {}", file_name(&dir)));
				} else {
					self.jobs
						.problem(op, kind, files, "No terminal could be started".into());
				}
			}
			Op::CopyMarkdown => {
				let mut out = String::new();
				for f in files {
					if let Ok(text) = tapwheel_core::text::read(f) {
						if files.len() > 1 {
							out.push_str(&format!("**{}**\n\n", file_name(f)));
						}
						out.push_str(&tapwheel_core::code::markdown_block(&text, &ext_of(f)));
						out.push('\n');
					}
				}
				if shell::copy_text(self.main, out.trim_end()) {
					self.jobs
						.note(op, kind, files, "Copied as a Markdown code block".into());
				} else {
					self.busy_clipboard(op, kind, files);
				}
			}
			Op::Stats => {
				let report: Vec<String> = files
					.iter()
					.filter_map(|f| tapwheel_core::text::stats(f).ok())
					.collect();
				match report.as_slice() {
					[] => {
						self.jobs
							.problem(op, kind, files, "These files couldn't be read as text".into());
					}
					[one] => {
						self.jobs.note_with_copy(op, kind, files, "Counted".into(), one.clone());
					}
					many => {
						self.jobs.note_with_copy(
							op,
							kind,
							files,
							format!("Counted {} files", many.len()),
							many.join("\n"),
						);
					}
				}
			}
			_ => {}
		}
		self.refresh_hud();
	}

	fn busy_clipboard(&mut self, op: Op, kind: Option<Kind>, files: &[PathBuf]) {
		self.jobs.problem(
			op,
			kind,
			files,
			"Another app is holding the clipboard; try again".into(),
		);
	}

	fn move_to(&mut self, preset: Option<&str>, files: &[PathBuf]) {
		let kind = selection_kind(files);
		let dest = match preset
			.and_then(|p| p.strip_prefix("recent"))
			.and_then(|n| n.parse::<usize>().ok())
		{
			Some(i) => self.settings.recent_folders.get(i).cloned(),
			None => {
				let start = self
					.settings
					.recent_folders
					.first()
					.cloned()
					.or_else(|| files[0].parent().map(PathBuf::from));
				share::pick_folder(self.main as isize, start.as_deref())
			}
		};
		let Some(dest) = dest else { return };
		if files.iter().all(|f| f.parent() == Some(dest.as_path())) {
			self.jobs
				.note(Op::MoveTo, kind, files, format!("Already in {}", file_name(&dest)));
			return;
		}
		if shell::move_to(files, &dest) {
			self.settings.remember_folder(&dest);
			self.save();
			let moved: Vec<PathBuf> = files
				.iter()
				.map(|f| dest.join(f.file_name().unwrap_or_default()))
				.collect();
			self.jobs
				.note(Op::MoveTo, kind, &moved, format!("Moved to {}", file_name(&dest)));
		} else {
			self.jobs
				.problem(Op::MoveTo, kind, files, "The move didn't finish".into());
		}
	}

	fn checksum(&mut self, preset: Option<&str>, files: &[PathBuf]) {
		let kind = selection_kind(files);
		let files: Vec<PathBuf> = files.iter().filter(|f| f.is_file()).cloned().collect();
		if files.is_empty() {
			self.jobs
				.problem(Op::Hash, kind, &files, "Checksums need files, not folders".into());
			return;
		}
		let expected = if preset == Some("check") {
			match shell::clipboard_text(self.main).and_then(|t| tapwheel_core::hash::kind_of(&t).map(|k| (k, t))) {
				Some(found) => Some(found),
				None => {
					self.jobs.problem(
						Op::Hash,
						kind,
						&files,
						"Copy the expected checksum first, then compare".into(),
					);
					return;
				}
			}
		} else {
			None
		};
		let algo = match (preset, &expected) {
			(_, Some((k, _))) => *k,
			(Some("md5"), _) => HashKind::Md5,
			(Some("sha1"), _) => HashKind::Sha1,
			_ => HashKind::Sha256,
		};
		let label = match algo {
			HashKind::Sha256 => "SHA-256",
			HashKind::Sha1 => "SHA-1",
			HashKind::Md5 => "MD5",
		};
		let (id, cancel) = self.jobs.begin(Op::Hash, kind, &files);
		let batches = self.jobs.batches.clone();
		let wake = self.wake();
		std::thread::spawn(move || {
			let mut lines = Vec::new();
			let mut failed = None;
			for f in &files {
				if cancel.load(std::sync::atomic::Ordering::Relaxed) {
					failed = Some("Cancelled".to_string());
					break;
				}
				match tapwheel_core::hash::hash(f, algo) {
					Ok(h) => lines.push((file_name(f), h)),
					Err(e) => {
						failed = Some(format!("{}: {e}", file_name(f)));
						break;
					}
				}
			}
			let result = match (failed, expected) {
				(Some(e), _) => Err(e),
				(None, Some((_, want))) => {
					let want = want.trim().to_lowercase();
					match lines.iter().find(|(_, h)| *h == want) {
						Some((name, _)) => Ok((format!("{label} matches {name}"), None)),
						None => Err(format!("{label} doesn't match; the file may be damaged or different")),
					}
				}
				(None, None) => {
					let copy = if lines.len() == 1 {
						lines[0].1.clone()
					} else {
						lines
							.iter()
							.map(|(n, h)| format!("{h}  {n}"))
							.collect::<Vec<_>>()
							.join("\n")
					};
					Ok((format!("{label} ready"), Some(copy)))
				}
			};
			Jobs::finish_note(&batches, id, result);
			wake();
		});
	}
}
