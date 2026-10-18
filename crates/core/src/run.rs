use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

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
		let note = report(job, cancel)?;
		return Ok(Outcome {
			output: PathBuf::new(),
			bytes_in,
			bytes_out: 0,
			note: Some(note),
		});
	}
	let part = part_path(&job.output);
	if let Err(e) = execute(job, &part, engines, defaults, progress, cancel) {
		remove(&part);
		return Err(e);
	}
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
	if let Err(e) = fs::rename(&part, &output) {
		remove(&part);
		return Err(e.into());
	}
	keep_date(job, &output);
	Ok(Outcome {
		bytes_out: size_of(&output),
		output,
		bytes_in,
		note: None,
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
) -> Result<(), Error> {
	let input = &job.inputs[0];
	let kind = kind_of(input);
	let ext = ext_of(input);
	let write = |s: String| -> Result<(), Error> { Ok(fs::write(part, s)?) };
	match &job.recipe {
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
			let scratch = e
				.scratch
				.join(format!("repack-{}-{}", std::process::id(), stem_of(part).len()));
			let result = archive::repack(input, *to, part, &scratch, cancel);
			let _ = fs::remove_dir_all(&scratch);
			result
		}
		Recipe::Extract => archive::extract(input, part, cancel),
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
			write(code::to_html(&text::read(input)?, &ext, &name)?)
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
			let body = text::read(input)?;
			let body = match ext.as_str() {
				"srt" | "vtt" | "ass" | "ssa" => subs::convert(&body, Format::Txt),
				"md" | "markdown" | "html" | "htm" | "rtf" => text::convert(input, Format::Txt)?,
				_ => body,
			};
			let wav = e.scratch.join(format!("speech-{}.wav", std::process::id()));
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
	}
}
