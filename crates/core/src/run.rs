use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

use crate::Error;
use crate::format::Format;
use crate::kind::{Kind, ext_of, kind_of};
use crate::plan::{Job, part_path, stem_of, unique_path};
use crate::recipe::Recipe;
use crate::settings::Defaults;
use crate::{archive, code, codeshot, icons, image, media, pngquant, speech, subs, text};

/// Where the bundled engines live.
#[derive(Clone, Debug)]
pub struct Engines {
	pub ffmpeg: PathBuf,
	pub ffprobe: PathBuf,
	/// Scratch space for intermediate files, cleaned as jobs finish.
	pub scratch: PathBuf,
	/// The background-removal model and the runtime that runs it.
	pub model: PathBuf,
	pub onnxruntime: PathBuf,
	/// Where expanded model weights are kept between runs.
	pub cache: PathBuf,
}

impl Engines {
	/// A path in the scratch folder that no other job, in this run of the app, is given.
	pub fn scratch_path(&self, tag: &str, ext: &str) -> PathBuf {
		static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
		let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let name = format!("{tag}-{}-{n}", std::process::id());
		self.scratch
			.join(if ext.is_empty() { name } else { format!("{name}.{ext}") })
	}

	/// Engines in a folder laid out like the app's `engines/`.
	pub fn in_dir(dir: &Path, scratch: PathBuf) -> Engines {
		let exe = |n: &str| {
			dir.join(if cfg!(windows) {
				format!("{n}.exe")
			} else {
				n.to_string()
			})
		};
		Engines {
			ffmpeg: exe("ffmpeg"),
			ffprobe: exe("ffprobe"),
			cache: scratch.join("cache"),
			scratch,
			model: dir.join("background.onnx"),
			onnxruntime: dir.join(if cfg!(windows) {
				"onnxruntime.dll"
			} else {
				"libonnxruntime.so"
			}),
		}
	}
}

#[derive(Debug)]
pub struct Outcome {
	/// Empty when the job only reports, like testing an archive.
	pub output: PathBuf,
	pub bytes_in: u64,
	pub bytes_out: u64,
	/// A short report shown on the card instead of sizes.
	pub note: Option<String>,
	/// Something the card says next to the sizes, like what an extraction had to skip.
	pub warning: Option<String>,
	/// How the output looked when the job finished, so Undo can tell if it changed since.
	pub stamp: Option<Stamp>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
	pub size: u64,
	pub modified: Option<SystemTime>,
}

pub fn stamp(path: &Path) -> Stamp {
	let meta = fs::metadata(path).ok();
	Stamp {
		size: size_of(path),
		modified: meta.and_then(|m| m.modified().ok()),
	}
}

/// Whether `path` is still exactly what TapWheel wrote, so removing it loses nothing of yours.
pub fn unchanged(path: &Path, was: Stamp) -> bool {
	path.exists() && stamp(path) == was
}

/// `.tw-part` files of jobs still running, so a crash's leftovers can be cleared on the next start.
static JOURNAL: Mutex<()> = Mutex::new(());

fn journal(e: &Engines) -> PathBuf {
	e.cache.join("parts.txt")
}

fn note_part(e: &Engines, part: &Path, running: bool) {
	let _lock = JOURNAL.lock().unwrap_or_else(|p| p.into_inner());
	let path = journal(e);
	let mut lines: Vec<String> = fs::read_to_string(&path)
		.unwrap_or_default()
		.lines()
		.map(str::to_string)
		.collect();
	let me = part.to_string_lossy().into_owned();
	lines.retain(|l| *l != me);
	if running {
		lines.push(me);
	}
	let _ = fs::create_dir_all(&e.cache);
	let _ = fs::write(&path, lines.join("\n"));
}

/// Removes `.tw-part` files a crash left behind, and returns how many there were.
pub fn clear_leftovers(e: &Engines) -> usize {
	let _lock = JOURNAL.lock().unwrap_or_else(|p| p.into_inner());
	let path = journal(e);
	let Ok(list) = fs::read_to_string(&path) else { return 0 };
	let mut n = 0;
	for line in list.lines().filter(|l| l.ends_with(".tw-part")) {
		let p = Path::new(line);
		if p.exists() {
			remove(p);
			n += 1;
		}
	}
	let _ = fs::remove_file(&path);
	n
}

/// Deletes what an earlier run left in the scratch folder. Everything there is named
/// `<what>-<process id>-<n>` (frame strips, frames, clips, previews, renders, job scratch), so
/// anything carrying another process's id is from a run that crashed or lost power; only one
/// TapWheel runs at a time.
pub fn clear_scratch(e: &Engines) -> usize {
	let me = std::process::id().to_string();
	let mut n = 0;
	for dir in [e.scratch.join("web"), e.scratch.clone()] {
		let Ok(entries) = fs::read_dir(&dir) else { continue };
		for entry in entries.flatten() {
			let name = entry.file_name().to_string_lossy().into_owned();
			let owner = name.split('-').nth(1).unwrap_or("");
			if !owner.is_empty() && owner.bytes().all(|b| b.is_ascii_digit()) && owner != me {
				remove(&entry.path());
				n += 1;
			}
		}
	}
	n
}

/// Free bytes on the drive holding `dir`, where the system says.
pub fn free_space(dir: &Path) -> Option<u64> {
	#[cfg(windows)]
	{
		use std::os::windows::ffi::OsStrExt;
		let wide: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
		let mut free = 0u64;
		let ok = unsafe {
			windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
				wide.as_ptr(),
				&mut free,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
			)
		};
		(ok != 0).then_some(free)
	}
	#[cfg(unix)]
	{
		use std::os::unix::ffi::OsStrExt;
		let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).ok()?;
		let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
		let ok = unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0;
		ok.then(|| st.f_bavail as u64 * st.f_frsize as u64)
	}
	#[cfg(not(any(windows, unix)))]
	{
		let _ = dir;
		None
	}
}

fn readable(path: &Path) -> std::io::Result<()> {
	if path.is_dir() {
		fs::read_dir(path).map(|_| ())
	} else {
		fs::File::open(path).map(|_| ())
	}
}

/// Says what's wrong before any work starts: a file that can't be read, or too little room.
pub fn preflight(job: &Job, bytes_in: u64) -> Result<(), Error> {
	for input in &job.inputs {
		if let Err(e) = readable(input) {
			let name = input.file_name().unwrap_or_default().to_string_lossy();
			return Err(Error::Engine(format!("{name} can't be read: {e}")));
		}
	}
	let dir = job.output.parent().unwrap_or(Path::new("."));
	let need = bytes_in + 64 * 1024 * 1024;
	if let Some(free) = free_space(dir)
		&& free < need
	{
		return Err(Error::Engine(format!(
			"there isn't enough free space: about {} needed, {} free",
			mb(need),
			mb(free)
		)));
	}
	Ok(())
}

fn mb(bytes: u64) -> String {
	format!("{} MB", bytes.div_ceil(1024 * 1024))
}

pub fn size_of(path: &Path) -> u64 {
	match fs::symlink_metadata(path) {
		Ok(m) if m.is_dir() => fs::read_dir(path)
			.map(|rd| rd.flatten().map(|e| size_of(&e.path())).sum())
			.unwrap_or(0),
		Ok(m) => m.len(),
		Err(_) => 0,
	}
}

/// Stack for the threads that run jobs: parsers and formatters recurse, and the default 2 MB
/// runs out on deeply nested files before the depth checks would.
pub const JOB_STACK: usize = 64 * 1024 * 1024;

/// What a job's card says when its engine panicked; the log has the details.
const BUG: &str = "TapWheel hit a bug with this file, so nothing was saved (details are in the log)";

/// Runs a job into a `.tw-part` file, then gives it its final name.
/// The final name is re-checked at the end, so a file that appeared meanwhile is never replaced.
pub fn run(
	job: &Job,
	engines: &Engines,
	defaults: &Defaults,
	progress: &mut dyn FnMut(f32),
	cancel: &AtomicBool,
) -> Result<Outcome, Error> {
	let bytes_in = job.inputs.iter().map(|p| size_of(p)).sum();
	if job.output.as_os_str().is_empty() {
		let note = crate::caught(BUG, || report(job, cancel))?;
		return Ok(Outcome {
			output: PathBuf::new(),
			bytes_in,
			bytes_out: 0,
			note: Some(note),
			warning: None,
			stamp: None,
		});
	}
	preflight(job, bytes_in)?;
	let part = part_path(&job.output);
	note_part(engines, &part, true);
	let done = crate::caught(BUG, || execute(job, &part, engines, defaults, progress, cancel));
	let warning = match done {
		Ok(w) => w,
		Err(e) => {
			remove(&part);
			note_part(engines, &part, false);
			return Err(e);
		}
	};
	// the check and the rename happen together, so a job finishing at the same moment with the
	// same name can't slip in between and be replaced
	static NAMING: Mutex<()> = Mutex::new(());
	let naming = NAMING.lock().unwrap_or_else(|p| p.into_inner());
	let output = if job.output.exists() {
		let dir = job.output.parent().unwrap_or(Path::new(""));
		let ext = if part.is_dir() {
			String::new()
		} else {
			let name = job
				.output
				.file_name()
				.unwrap_or_default()
				.to_string_lossy()
				.into_owned();
			let stem = stem_of(&job.output);
			name.strip_prefix(&format!("{stem}.")).unwrap_or("").to_string()
		};
		unique_path(dir, &stem_of(&job.output), &ext)
	} else {
		job.output.clone()
	};
	let renamed = fs::rename(&part, &output);
	drop(naming);
	if let Err(e) = renamed {
		remove(&part);
		note_part(engines, &part, false);
		return Err(e.into());
	}
	note_part(engines, &part, false);
	keep_date(job, &output);
	Ok(Outcome {
		bytes_out: size_of(&output),
		stamp: Some(stamp(&output)),
		output,
		bytes_in,
		note: None,
		warning,
	})
}

/// A converted photo keeps the date it was taken, so it sorts next to the original.
fn keep_date(job: &Job, output: &Path) {
	let [input] = &job.inputs[..] else { return };
	if output.is_dir() {
		return;
	}
	if let Ok(time) = fs::metadata(input).and_then(|m| m.modified())
		&& let Ok(f) = fs::File::options().write(true).open(output)
	{
		let _ = f.set_modified(time);
	}
}

fn copy_all(from: &Path, to: &Path) -> Result<(), Error> {
	if from.is_dir() {
		fs::create_dir_all(to)?;
		for entry in fs::read_dir(from)? {
			let entry = entry?;
			copy_all(&entry.path(), &to.join(entry.file_name()))?;
		}
	} else {
		fs::copy(from, to)?;
	}
	Ok(())
}

fn remove(path: &Path) {
	let _ = if path.is_dir() {
		fs::remove_dir_all(path)
	} else {
		fs::remove_file(path)
	};
}

fn report(job: &Job, cancel: &AtomicBool) -> Result<String, Error> {
	match &job.recipe {
		Recipe::Test => {
			let n = archive::test(&job.inputs[0], cancel)?;
			Ok(format!("All {n} files are fine"))
		}
		_ => Err(Error::Engine("nothing to report".into())),
	}
}

fn execute(
	job: &Job,
	part: &Path,
	e: &Engines,
	d: &Defaults,
	progress: &mut dyn FnMut(f32),
	cancel: &AtomicBool,
) -> Result<Option<String>, Error> {
	let input = &job.inputs[0];
	if let Recipe::Extract { only } = &job.recipe {
		return archive::extract(input, part, only, cancel);
	}
	let kind = kind_of(input);
	let ext = ext_of(input);
	let write = |s: String| -> Result<(), Error> { Ok(fs::write(part, s)?) };
	let done = match &job.recipe {
		Recipe::Duplicate => copy_all(input, part),
		// whole-selection recipes
		Recipe::Pack { format } => archive::pack(&job.inputs, *format, part, cancel),
		Recipe::Convert { to } if to.is_archive() && kind != Kind::Archive => {
			archive::pack(&job.inputs, *to, part, cancel)
		}
		Recipe::Protect { password, format } => archive::pack_with(&job.inputs, *format, Some(password), part, cancel),
		Recipe::Join => media::join(e, d, &job.inputs, part, progress, cancel),
		Recipe::Merge => write(text::merge(&job.inputs)?),
		Recipe::Combine {
			layout,
			gap,
			background,
		} => image::combine(e, &job.inputs, *layout, *gap, background, part, cancel),

		// archives
		Recipe::Convert { to } if kind == Kind::Archive => {
			let scratch = e.scratch_path("repack", "");
			let result = archive::repack(input, *to, part, &scratch, cancel);
			let _ = fs::remove_dir_all(&scratch);
			result
		}
		Recipe::SplitArchive { part_mb } => archive::split(input, *part_mb, part, cancel).map(drop),

		// icons, from icon files, pictures or programs
		Recipe::WindowsIcon => icons::windows_icon(e, &job.inputs, part),
		Recipe::MacIcon => icons::mac_icon(e, &job.inputs, part),
		Recipe::IconSet { sizes } => icons::icon_set(e, input, sizes, part),
		Recipe::Favicon => icons::favicon(e, input, part),
		Recipe::ExtractIcons => icons::extract_icons(e, input, part),
		Recipe::Convert { to: Format::Ico } => icons::windows_icon(e, &job.inputs, part),
		Recipe::Convert { to: Format::Icns } => icons::mac_icon(e, &job.inputs, part),
		Recipe::Convert { .. } if kind == Kind::Icon => image::run(e, d, &job.recipe, input, part, progress, cancel),

		// code and text
		Recipe::Convert { to: Format::Png } if kind == Kind::Code => {
			let style = codeshot::Style {
				dark: d.code_theme != "light",
				background: d.code_background.clone(),
				title: input
					.file_name()
					.map(|n| n.to_string_lossy().into_owned())
					.unwrap_or_default(),
				..codeshot::Style::default()
			};
			codeshot::render_png(&text::read(input)?, &ext, &style, part)
		}
		Recipe::Convert { to: Format::Html } if kind == Kind::Code => {
			let name = input
				.file_name()
				.map(|n| n.to_string_lossy().into_owned())
				.unwrap_or_default();
			write(code::to_html(
				&text::read_within(input, text::DATA_LIMIT)?,
				&ext,
				&name,
			)?)
		}
		Recipe::Convert { to: Format::Md } if kind == Kind::Code => {
			write(code::markdown_block(&text::read(input)?, &ext))
		}
		Recipe::Convert { to: Format::Txt } if kind == Kind::Code => write(text::read(input)?),
		Recipe::Convert { to } if matches!(kind, Kind::Text | Kind::Code) => write(text::convert(input, *to)?),
		Recipe::ToUtf8
		| Recipe::LineEndings { .. }
		| Recipe::CleanUp
		| Recipe::SortLines { .. }
		| Recipe::Dedupe
		| Recipe::ChangeCase { .. }
		| Recipe::Indent { .. }
		| Recipe::Pretty
		| Recipe::Minify => Ok(fs::write(part, text::tool(input, &job.recipe)?)?),
		Recipe::SplitText { lines } => text::split(input, *lines, part).map(drop),
		Recipe::ReadAloud { voice, rate } => {
			let body = text::read_within(input, text::SPEECH_LIMIT)?;
			let body = match ext.as_str() {
				"srt" | "vtt" | "ass" | "ssa" => subs::convert(&body, Format::Txt),
				"md" | "markdown" | "html" | "htm" | "rtf" => text::convert(input, Format::Txt)?,
				_ => body,
			};
			let wav = e.scratch_path("speech", "wav");
			fs::create_dir_all(&e.scratch)?;
			speech::speak(&body, voice.as_deref(), *rate, &wav)?;
			let r = media::wav_to_mp3(e, &wav, part, progress, cancel);
			let _ = fs::remove_file(&wav);
			r
		}

		// pictures
		Recipe::Convert { .. }
		| Recipe::CompressImage { .. }
		| Recipe::ResizeImage { .. }
		| Recipe::RotateImage { .. }
		| Recipe::CropImage { .. }
		| Recipe::StripMetadata { .. }
		| Recipe::RemoveBackground
		| Recipe::Trace
		| Recipe::Rendered { .. }
			if matches!(kind, Kind::Image | Kind::Icon) =>
		{
			image::run(e, d, &job.recipe, input, part, progress, cancel)
		}
		Recipe::Rendered { file, .. } => {
			fs::copy(file, part)?;
			Ok(())
		}

		// video and sound
		Recipe::Convert { to } => media::convert(e, d, input, *to, part, progress, cancel),
		Recipe::CompressImage { quality, .. } if ext == "png" => pngquant::compress(input, part, *quality),
		_ => media::run(e, d, &job.recipe, input, part, progress, cancel),
	};
	done.map(|()| None)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ops::Op;

	fn dir(tag: &str) -> PathBuf {
		let d = std::env::temp_dir().join(format!("tw-run-{tag}-{}", std::process::id()));
		let _ = fs::remove_dir_all(&d);
		fs::create_dir_all(&d).unwrap();
		d
	}

	fn engines(d: &Path) -> Engines {
		let mut e = Engines::in_dir(d, d.join("scratch"));
		e.cache = d.join("cache");
		e
	}

	#[test]
	fn undo_only_takes_what_is_unchanged() {
		let d = dir("undo");
		let input = d.join("notes.txt");
		fs::write(&input, "b\na\n").unwrap();
		let job = crate::plan(
			Op::SortLines,
			&Recipe::SortLines {
				by: crate::recipe::SortBy::Az,
			},
			&[input],
		)
		.remove(0);
		let out = run(
			&job,
			&engines(&d),
			&Defaults::default(),
			&mut |_| {},
			&AtomicBool::new(false),
		)
		.unwrap();
		let was = out.stamp.unwrap();
		assert!(unchanged(&out.output, was));
		fs::write(&out.output, "a\nb\nc\n").unwrap();
		assert!(!unchanged(&out.output, was), "edited since");
		assert!(
			!fs::read_to_string(journal(&engines(&d)))
				.unwrap_or_default()
				.contains(".tw-part")
		);
	}

	#[test]
	fn a_crash_leaves_nothing_after_the_next_start() {
		let d = dir("crash");
		let e = engines(&d);
		let part = d.join("photo-small.jpg.tw-part");
		fs::write(&part, "half a file").unwrap();
		note_part(&e, &part, true);
		assert_eq!(clear_leftovers(&e), 1);
		assert!(!part.exists());
		assert_eq!(clear_leftovers(&e), 0);
	}

	#[test]
	fn preflight_names_what_is_wrong() {
		let d = dir("pre");
		let gone = d.join("gone.txt");
		let job = crate::plan(Op::CleanUp, &Recipe::CleanUp, std::slice::from_ref(&gone)).remove(0);
		let err = preflight(&job, 0).unwrap_err().to_string();
		assert!(err.contains("gone.txt can't be read"), "{err}");
		assert!(free_space(&d).is_some_and(|f| f > 0));
	}

	#[test]
	fn read_aloud_refuses_a_book_before_speaking() {
		let d = dir("speech");
		let input = d.join("novel.txt");
		fs::write(&input, "word ".repeat(80_000)).unwrap();
		let job = crate::plan(
			Op::ReadAloud,
			&Recipe::ReadAloud { voice: None, rate: 1.0 },
			std::slice::from_ref(&input),
		)
		.remove(0);
		let err = run(
			&job,
			&engines(&d),
			&Defaults::default(),
			&mut |_| {},
			&AtomicBool::new(false),
		)
		.unwrap_err();
		assert!(err.to_string().starts_with("This file is too big"), "{err}");
	}

	/// Runs both jobs at the same moment and returns what each made.
	fn side_by_side(a: Job, b: Job, e: &Engines) -> (Result<Outcome, Error>, Result<Outcome, Error>) {
		let go = std::sync::Barrier::new(2);
		std::thread::scope(|s| {
			let one = |job: Job| {
				let go = &go;
				s.spawn(move || {
					go.wait();
					run(&job, e, &Defaults::default(), &mut |_| {}, &AtomicBool::new(false))
				})
			};
			let (x, y) = (one(a), one(b));
			(x.join().unwrap(), y.join().unwrap())
		})
	}

	#[test]
	fn two_jobs_aiming_at_one_name_each_keep_their_own() {
		for round in 0..4 {
			let d = dir(&format!("same-name-{round}"));
			let e = engines(&d);
			let rows = |tag: &str| {
				(0..20_000)
					.map(|i| format!("{{\"{tag}\":{i}}}"))
					.collect::<Vec<_>>()
					.join(",")
			};
			let (a, b) = (d.join("a.json"), d.join("a.jsonl"));
			fs::write(&a, format!("[{}]", rows("from_json"))).unwrap();
			fs::write(&b, rows("from_lines").replace(',', "\n")).unwrap();
			let to = Recipe::Convert { to: Format::Csv };
			let ja = crate::plan(Op::Convert(Format::Csv), &to, std::slice::from_ref(&a)).remove(0);
			let jb = crate::plan(Op::Convert(Format::Csv), &to, std::slice::from_ref(&b)).remove(0);
			assert_eq!(ja.output, jb.output, "both aim at a.csv");
			let (ra, rb) = side_by_side(ja, jb, &e);
			let (oa, ob) = (ra.unwrap().output, rb.unwrap().output);
			assert_ne!(oa, ob);
			assert!(fs::read_to_string(&oa).unwrap().starts_with("from_json"));
			assert!(fs::read_to_string(&ob).unwrap().starts_with("from_lines"));
		}
	}

	#[test]
	fn two_archive_conversions_at_once_stay_apart() {
		let d = dir("repack");
		let e = engines(&d);
		let mut zips = Vec::new();
		for tag in ["x", "y"] {
			let src = d.join(format!("{tag}-files"));
			fs::create_dir_all(&src).unwrap();
			for i in 0..200 {
				fs::write(src.join(format!("{tag}{i}.txt")), tag.repeat(20_000)).unwrap();
			}
			let zip = d.join(format!("{tag}.zip"));
			archive::pack(&[src], Format::Zip, &zip, &AtomicBool::new(false)).unwrap();
			zips.push(zip);
		}
		let to = Recipe::Convert { to: Format::Tar };
		let mut jobs = crate::plan(Op::Convert(Format::Tar), &to, &zips);
		let (rx, ry) = side_by_side(jobs.remove(0), jobs.remove(0), &e);
		for (out, tag) in [(rx.unwrap().output, "x"), (ry.unwrap().output, "y")] {
			let names: Vec<String> = archive::list(&out).unwrap().into_iter().map(|i| i.name).collect();
			assert_eq!(
				names.iter().filter(|n| n.ends_with(".txt")).count(),
				200,
				"{tag}: {names:?}"
			);
			assert!(
				names
					.iter()
					.all(|n| !n.ends_with(".txt") || n.contains(&format!("{tag}-files/{tag}"))),
				"{tag}: {names:?}"
			);
		}
	}

	#[test]
	fn a_crashed_runs_scratch_goes_at_the_next_start() {
		let d = dir("scratch-leftovers");
		let e = engines(&d);
		fs::create_dir_all(e.scratch.join("web")).unwrap();
		let old = [
			e.scratch.join("strip-1-0.png"),
			e.scratch.join("clip-1-3.mp4"),
			e.scratch.join("web").join("render-1-0.png"),
		];
		for f in &old {
			fs::write(f, b"x").unwrap();
		}
		fs::create_dir_all(e.scratch.join("repack-1-2").join("inside")).unwrap();
		let ours = e.scratch_path("still", "jpg");
		fs::write(&ours, b"x").unwrap();
		let other = e.scratch.join("notes.txt");
		fs::write(&other, b"x").unwrap();
		assert_eq!(clear_scratch(&e), 4);
		assert!(old.iter().all(|f| !f.exists()));
		assert!(!e.scratch.join("repack-1-2").exists());
		assert!(ours.exists() && other.exists() && e.scratch.join("web").exists());
	}

	#[test]
	fn scratch_names_are_never_shared() {
		let e = engines(Path::new("/x"));
		assert_ne!(e.scratch_path("pass", ""), e.scratch_path("pass", ""));
	}
}
