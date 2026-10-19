//! Everyday actions that need no engine: copying names, renaming in bulk, and saying what a
//! file really is.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Error;
use crate::kind::{Kind, ext_of, kind_of};
use crate::plan::stem_of;
use crate::recipe::Case;
use crate::run::Engines;
use crate::{archive, ff};

/// What Copy path puts on the clipboard for the ring choice `which`, one line per item.
pub fn clip_text(paths: &[PathBuf], which: &str) -> String {
	paths
		.iter()
		.map(|p| match which {
			"name" => name_of(p),
			"stem" => stem_of(p),
			"ext" => p
				.extension()
				.map(|e| e.to_string_lossy().into_owned())
				.unwrap_or_default(),
			"quoted" => format!("\"{}\"", p.display()),
			"uri" => file_uri(p),
			_ => p.display().to_string(),
		})
		.collect::<Vec<_>>()
		.join("\r\n")
}

fn name_of(p: &Path) -> String {
	p.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default()
}

/// `C:\My files\a b.txt` becomes `file:///C:/My%20files/a%20b.txt`; network paths keep their server.
pub fn file_uri(p: &Path) -> String {
	let s = p.to_string_lossy().replace('\\', "/");
	let (head, rest) = match s.strip_prefix("//") {
		Some(unc) => ("file://", unc.to_string()),
		None if s.starts_with('/') => ("file://", s.clone()),
		None => ("file:///", s.clone()),
	};
	let mut out = String::from(head);
	for b in rest.bytes() {
		match b {
			b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => out.push(b as char),
			_ => out.push_str(&format!("%{b:02X}")),
		}
	}
	out
}

/// How Rename builds new names. Every part is optional; the extension is always kept.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenameRule {
	/// Replaces the name before the other parts apply; numbering is added when there are several files.
	pub base: Option<String>,
	pub find: String,
	pub replace: String,
	pub case: Option<Case>,
	pub prefix: String,
	pub suffix: String,
	pub number: bool,
	pub start: u32,
	pub pad: u8,
	/// Puts the day the file last changed in front, as 2026-10-18, so names sort by date.
	pub date: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Renamed {
	pub from: PathBuf,
	pub to: PathBuf,
	/// Why this one can't be renamed; nothing is renamed while any has a problem.
	pub problem: Option<String>,
}

pub(crate) const RESERVED: &[&str] = &[
	"CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2",
	"LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The new name for every file, with any problem found before anything changes.
pub fn rename_plan(paths: &[PathBuf], rule: &RenameRule) -> Vec<Renamed> {
	let many = paths.len() > 1;
	let width = rule.pad.max(1) as usize;
	let mut out: Vec<Renamed> = paths
		.iter()
		.enumerate()
		.map(|(i, p)| {
			let ext = if p.is_dir() { String::new() } else { ext_of(p) };
			let mut stem = rule
				.base
				.clone()
				.filter(|b| !b.is_empty())
				.unwrap_or_else(|| stem_of(p));
			if !rule.find.is_empty() {
				stem = stem.replace(&rule.find, &rule.replace);
			}
			stem = match rule.case {
				Some(Case::Upper) => stem.to_uppercase(),
				Some(Case::Lower) => stem.to_lowercase(),
				Some(Case::Title) => crate::text::title_case(&stem),
				Some(Case::Sentence) => crate::text::sentence_case(&stem),
				None => stem,
			};
			stem = format!("{}{stem}{}", rule.prefix, rule.suffix);
			if rule.date
				&& let Some(day) = std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(day_of)
			{
				stem = format!("{day} {stem}");
			}
			if rule.number || (many && rule.base.as_deref().is_some_and(|b| !b.is_empty())) {
				let n = rule.start as usize + i;
				stem = format!("{stem} {n:0width$}");
			}
			// everything after the stem, as it was spelled: "tar.gz" stays whole
			let ext = if ext.is_empty() {
				String::new()
			} else {
				let name = p
					.file_name()
					.map(|n| n.to_string_lossy().into_owned())
					.unwrap_or_default();
				name.strip_prefix(&stem_of(p))
					.map(|rest| rest.trim_start_matches('.').to_string())
					.unwrap_or_default()
			};
			let name = if ext.is_empty() { stem } else { format!("{stem}.{ext}") };
			Renamed {
				from: p.clone(),
				to: p.with_file_name(&name),
				problem: name_problem(&name),
			}
		})
		.collect();
	let sources: HashSet<String> = paths.iter().map(|p| key(p)).collect();
	let mut seen = HashSet::new();
	for r in &mut out {
		if r.problem.is_some() {
			continue;
		}
		let k = key(&r.to);
		if !seen.insert(k.clone()) {
			r.problem = Some("another file gets this name too".into());
		} else if r.to.exists() && !sources.contains(&k) {
			r.problem = Some("a file with this name is already there".into());
		}
	}
	out
}

/// A time as its calendar day, 2026-10-18, in UTC.
fn day_of(t: std::time::SystemTime) -> Option<String> {
	let days = (t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() / 86_400) as i64;
	// Howard Hinnant's civil_from_days
	let z = days + 719_468;
	let era = z.div_euclid(146_097);
	let doe = z - era * 146_097;
	let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let d = doy - (153 * mp + 2) / 5 + 1;
	let m = if mp < 10 { mp + 3 } else { mp - 9 };
	let y = yoe + era * 400 + i64::from(m <= 2);
	Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// Windows compares names without case.
fn key(p: &Path) -> String {
	p.to_string_lossy().to_lowercase()
}

fn name_problem(name: &str) -> Option<String> {
	let stem = name.split('.').next().unwrap_or("");
	if name.trim().is_empty() || stem.trim().is_empty() {
		Some("the name is empty".into())
	} else if let Some(c) = name.chars().find(|c| r#"<>:"/\|?*"#.contains(*c) || c.is_control()) {
		Some(format!("names can't contain {c}"))
	} else if name.ends_with('.') || name.ends_with(' ') {
		Some("names can't end with a dot or a space".into())
	} else if RESERVED.contains(&stem.to_uppercase().as_str()) {
		Some(format!("Windows keeps {stem} for itself"))
	} else if name.chars().count() > 255 {
		Some("the name is too long".into())
	} else {
		None
	}
}

/// Renames everything in the plan, or nothing: names that swap go through a temporary name, and
/// a failure halfway puts back what was already renamed. Returns the pairs that changed.
pub fn rename_all(plan: &[Renamed]) -> Result<Vec<(PathBuf, PathBuf)>, Error> {
	if let Some(r) = plan.iter().find(|r| r.problem.is_some()) {
		return Err(Error::Engine(format!(
			"{}: {}",
			name_of(&r.from),
			r.problem.as_deref().unwrap_or_default()
		)));
	}
	let moving: Vec<&Renamed> = plan.iter().filter(|r| r.from != r.to).collect();
	let sources: HashSet<String> = moving.iter().map(|r| key(&r.from)).collect();
	// a file may have appeared under a new name since the preview; renaming over it would replace it
	if let Some(r) = moving.iter().find(|r| r.to.exists() && !sources.contains(&key(&r.to))) {
		return Err(Error::Engine(format!(
			"{} appeared since the preview, so nothing was renamed",
			name_of(&r.to)
		)));
	}
	let mut parked: Vec<(PathBuf, PathBuf, PathBuf)> = Vec::new();
	for (i, r) in moving.iter().enumerate() {
		let tmp = r.from.with_file_name(format!(".tw-rename-{}-{i}", std::process::id()));
		if let Err(e) = fs::rename(&r.from, &tmp) {
			let lost = put_back(&[], &parked);
			return Err(failed(&r.from, &e, lost));
		}
		parked.push((r.from.clone(), tmp, r.to.clone()));
	}
	let mut done: Vec<(PathBuf, PathBuf)> = Vec::new();
	for (from, tmp, to) in &parked {
		// every source is parked by now, so anything at `to` belongs to someone else
		let result = if to.exists() {
			Err(std::io::Error::new(
				std::io::ErrorKind::AlreadyExists,
				"a file with that name appeared",
			))
		} else {
			fs::rename(tmp, to)
		};
		if let Err(e) = result {
			let lost = put_back(&done, &parked);
			return Err(failed(from, &e, lost));
		}
		done.push((from.clone(), to.clone()));
	}
	Ok(done)
}

/// Undoes a rename halfway through. Returns the files that couldn't go back, by the name they
/// were left with.
fn put_back(done: &[(PathBuf, PathBuf)], parked: &[(PathBuf, PathBuf, PathBuf)]) -> Vec<String> {
	let mut lost = Vec::new();
	for (from, to) in done.iter().rev() {
		if fs::rename(to, from).is_err() {
			lost.push(name_of(to));
		}
	}
	for (from, tmp, _) in parked {
		if tmp.exists() && fs::rename(tmp, from).is_err() {
			lost.push(name_of(tmp));
		}
	}
	lost
}

fn failed(file: &Path, e: &std::io::Error, lost: Vec<String>) -> Error {
	let mut text = format!("{} couldn't be renamed: {e}", name_of(file));
	if lost.is_empty() {
		text.push_str("; every other name was put back");
	} else {
		text.push_str(&format!("; these couldn't be put back: {}", lost.join(", ")));
	}
	Error::Engine(text)
}

/// What the File info window shows.
#[derive(Clone, Debug, Serialize)]
pub struct Info {
	pub name: String,
	pub folder: String,
	pub size: u64,
	pub kind: String,
	/// What the name says it is, like "JPG".
	pub named: String,
	/// What the first bytes say it is, when they say anything.
	pub actual: Option<String>,
	/// False when the name claims one known type and the bytes say another.
	pub matches: bool,
	pub created: Option<i64>,
	pub modified: Option<i64>,
	/// Files inside a folder or archive, and their total size unpacked.
	pub items: Option<u64>,
	pub unpacked: Option<u64>,
	pub width: Option<u32>,
	pub height: Option<u32>,
	pub duration: Option<f64>,
	pub video: Option<String>,
	pub audio: Option<String>,
	pub fps: Option<f64>,
	pub lines: Option<u64>,
	pub words: Option<u64>,
}

fn secs(t: std::io::Result<std::time::SystemTime>) -> Option<i64> {
	t.ok()?
		.duration_since(std::time::UNIX_EPOCH)
		.ok()
		.map(|d| d.as_secs() as i64)
}

pub fn info(e: &Engines, p: &Path) -> Result<Info, Error> {
	let meta = fs::metadata(p)?;
	let kind = kind_of(p);
	let mut head = [0u8; 64];
	let actual = if meta.is_file() {
		use std::io::Read;
		let n = fs::File::open(p)?.read(&mut head)?;
		sniff(&head[..n]).map(str::to_string)
	} else {
		None
	};
	let mut out = Info {
		name: name_of(p),
		folder: p.parent().map(|d| d.display().to_string()).unwrap_or_default(),
		size: crate::run::size_of(p),
		kind: format!("{kind:?}").to_lowercase(),
		named: ext_of(p).to_uppercase(),
		matches: actual
			.as_deref()
			.is_none_or(|a| named_as(&ext_of(p)).is_none_or(|n| n == a)),
		actual,
		created: secs(meta.created()),
		modified: secs(meta.modified()),
		items: None,
		unpacked: None,
		width: None,
		height: None,
		duration: None,
		video: None,
		audio: None,
		fps: None,
		lines: None,
		words: None,
	};
	match kind {
		Kind::Folder => {
			let mut n = 0;
			count(p, &mut n);
			out.items = Some(n);
		}
		Kind::Archive => {
			if let Ok(list) = archive::list(p) {
				let files: Vec<_> = list.iter().filter(|i| !i.dir).collect();
				out.items = Some(files.len() as u64);
				out.unpacked = Some(files.iter().map(|i| i.size).sum());
			}
		}
		Kind::Image | Kind::Video | Kind::Audio | Kind::Icon => {
			if let Ok(pr) = ff::probe(&e.ffprobe, p) {
				let (w, h) = pr.shown_size();
				if w > 0 {
					out.width = Some(w);
					out.height = Some(h);
				}
				if kind != Kind::Image && pr.duration > 0.0 {
					out.duration = Some(pr.duration);
				}
				if kind == Kind::Video || kind == Kind::Audio {
					out.video = pr.video_codec.clone().filter(|_| kind == Kind::Video);
					out.audio = pr.audio_codec.clone();
					out.fps = (pr.fps > 0.0 && kind == Kind::Video).then_some(pr.fps);
				}
			}
		}
		Kind::Text | Kind::Code if meta.len() < 64 * 1024 * 1024 => {
			if let Ok(text) = crate::text::read(p) {
				out.lines = Some(text.lines().count() as u64);
				out.words = Some(text.split_whitespace().count() as u64);
			}
		}
		_ => {}
	}
	Ok(out)
}

fn count(dir: &Path, n: &mut u64) {
	if let Ok(rd) = fs::read_dir(dir) {
		for e in rd.flatten() {
			let p = e.path();
			if p.is_dir() {
				count(&p, n);
			} else {
				*n += 1;
			}
		}
	}
}

/// Which `sniff` answer a known extension promises.
fn named_as(ext: &str) -> Option<&'static str> {
	const NAMES: &[(&str, &[&str])] = &[
		("PNG", &["png", "apng"]),
		("JPG", &["jpg", "jpeg", "jfif", "jpe"]),
		("GIF", &["gif"]),
		("WebP", &["webp"]),
		("WAV", &["wav"]),
		("AVI", &["avi"]),
		("HEIC", &["heic", "heif", "hif"]),
		("AVIF", &["avif"]),
		("MOV", &["mov", "qt"]),
		("M4A", &["m4a", "m4b"]),
		("MP4", &["mp4", "m4v"]),
		("MKV or WebM", &["mkv", "mka", "webm"]),
		("MP3", &["mp3"]),
		("FLAC", &["flac"]),
		("OGG", &["ogg", "oga", "ogv", "opus"]),
		("BMP", &["bmp", "dib"]),
		("ICO", &["ico", "cur"]),
		("ICNS", &["icns"]),
		("TIFF", &["tif", "tiff"]),
		("PSD", &["psd"]),
		("PDF", &["pdf"]),
		(
			"ZIP",
			&[
				"zip", "docx", "xlsx", "pptx", "odt", "ods", "odp", "epub", "jar", "apk", "msix", "appx", "vsix",
			],
		),
		("7Z", &["7z"]),
		("RAR", &["rar"]),
		("GZIP", &["gz", "tgz"]),
		("XZ", &["xz", "txz"]),
		("ZSTD", &["zst"]),
		("BZIP2", &["bz2"]),
		("Windows program", &["exe", "dll", "scr", "sys"]),
	];
	NAMES.iter().find(|(_, exts)| exts.contains(&ext)).map(|(n, _)| *n)
}

/// The format a file's first bytes give away, whatever its name says.
pub fn sniff(b: &[u8]) -> Option<&'static str> {
	let at = |i: usize, sig: &[u8]| b.get(i..i + sig.len()) == Some(sig);
	Some(match () {
		_ if at(0, b"\x89PNG\r\n\x1a\n") => "PNG",
		_ if at(0, &[0xff, 0xd8, 0xff]) => "JPG",
		_ if at(0, b"GIF87a") || at(0, b"GIF89a") => "GIF",
		_ if at(0, b"RIFF") && at(8, b"WEBP") => "WebP",
		_ if at(0, b"RIFF") && at(8, b"WAVE") => "WAV",
		_ if at(0, b"RIFF") && at(8, b"AVI ") => "AVI",
		_ if at(4, b"ftypheic") || at(4, b"ftypheix") || at(4, b"ftypmif1") => "HEIC",
		_ if at(4, b"ftypavif") => "AVIF",
		_ if at(4, b"ftypqt") => "MOV",
		_ if at(4, b"ftypM4A") => "M4A",
		_ if at(4, b"ftyp") => "MP4",
		_ if at(0, &[0x1a, 0x45, 0xdf, 0xa3]) => "MKV or WebM",
		_ if at(0, b"ID3") || (b.len() > 1 && b[0] == 0xff && b[1] & 0xe0 == 0xe0) => "MP3",
		_ if at(0, b"fLaC") => "FLAC",
		_ if at(0, b"OggS") => "OGG",
		_ if at(0, b"BM") => "BMP",
		_ if at(0, &[0, 0, 1, 0]) => "ICO",
		_ if at(0, b"icns") => "ICNS",
		_ if at(0, b"II*\0") || at(0, b"MM\0*") => "TIFF",
		_ if at(0, b"8BPS") => "PSD",
		_ if at(0, b"%PDF") => "PDF",
		_ if at(0, b"PK\x03\x04") || at(0, b"PK\x05\x06") => "ZIP",
		_ if at(0, b"7z\xbc\xaf\x27\x1c") => "7Z",
		_ if at(0, b"Rar!\x1a\x07") => "RAR",
		_ if at(0, &[0x1f, 0x8b]) => "GZIP",
		_ if at(0, &[0xfd, b'7', b'z', b'X', b'Z', 0]) => "XZ",
		_ if at(0, &[0x28, 0xb5, 0x2f, 0xfd]) => "ZSTD",
		_ if at(0, b"BZh") => "BZIP2",
		_ if at(0, b"MZ") => "Windows program",
		_ if at(0, b"<svg") || at(0, b"<?xml") => "XML or SVG",
		_ => return None,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn dir(tag: &str) -> PathBuf {
		let d = std::env::temp_dir().join(format!("tw-files-{tag}-{}", std::process::id()));
		let _ = fs::remove_dir_all(&d);
		fs::create_dir_all(&d).unwrap();
		d
	}

	#[test]
	fn rename_keeps_a_double_extension() {
		let d = dir("tgz");
		let p = d.join("backup.tar.gz");
		fs::write(&p, "x").unwrap();
		let rule = RenameRule {
			prefix: "x-".into(),
			..RenameRule::default()
		};
		assert_eq!(rename_plan(&[p], &rule)[0].to, d.join("x-backup.tar.gz"));
	}

	#[test]
	fn rename_never_replaces_a_file_that_appeared_after_the_preview() {
		let d = dir("appeared");
		let a = d.join("a.txt");
		fs::write(&a, "mine").unwrap();
		let rule = RenameRule {
			base: Some("b".into()),
			..RenameRule::default()
		};
		let plan = rename_plan(std::slice::from_ref(&a), &rule);
		assert_eq!(plan[0].to, d.join("b.txt"));
		fs::write(d.join("b.txt"), "someone else's").unwrap();
		let err = rename_all(&plan).unwrap_err().to_string();
		assert!(err.contains("b.txt appeared"), "{err}");
		assert_eq!(fs::read_to_string(d.join("b.txt")).unwrap(), "someone else's");
		assert_eq!(fs::read_to_string(&a).unwrap(), "mine");
	}

	#[test]
	fn days_are_calendar_days() {
		let at = |secs: u64| day_of(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)).unwrap();
		assert_eq!(at(0), "1970-01-01");
		assert_eq!(at(951_782_400), "2000-02-29");
		assert_eq!(at(1_792_310_400), "2026-10-18");
	}

	#[test]
	fn copy_path_choices() {
		let p = vec![PathBuf::from("/home/jax/My files/a b.txt")];
		assert_eq!(clip_text(&p, "name"), "a b.txt");
		assert_eq!(clip_text(&p, "stem"), "a b");
		assert_eq!(clip_text(&p, "quoted"), "\"/home/jax/My files/a b.txt\"");
		assert_eq!(clip_text(&p, "uri"), "file:///home/jax/My%20files/a%20b.txt");
		assert_eq!(
			file_uri(Path::new(r"C:\Users\jax\café.png")),
			"file:///C:/Users/jax/caf%C3%A9.png"
		);
		assert_eq!(file_uri(Path::new(r"\\nas\photos\x.jpg")), "file://nas/photos/x.jpg");
	}

	#[test]
	fn rename_numbers_cases_and_swaps() {
		let d = dir("rename");
		let a = d.join("b.JPG");
		let b = d.join("a.jpg");
		fs::write(&a, "1").unwrap();
		fs::write(&b, "2").unwrap();
		let rule = RenameRule {
			base: Some("Trip".into()),
			start: 1,
			pad: 2,
			..Default::default()
		};
		let plan = rename_plan(&[a.clone(), b.clone()], &rule);
		assert_eq!(plan[0].to, d.join("Trip 01.JPG"));
		assert_eq!(plan[1].to, d.join("Trip 02.jpg"));
		assert!(plan.iter().all(|r| r.problem.is_none()));
		rename_all(&plan).unwrap();
		assert_eq!(fs::read_to_string(d.join("Trip 01.JPG")).unwrap(), "1");

		// two files trading names works through a temporary name
		let (x, y) = (d.join("Trip 01.JPG"), d.join("Trip 02.jpg"));
		let swap = vec![
			Renamed {
				from: x.clone(),
				to: d.join("Trip 02.JPG"),
				problem: None,
			},
			Renamed {
				from: y.clone(),
				to: d.join("Trip 01.jpg"),
				problem: None,
			},
		];
		rename_all(&swap).unwrap();
		assert_eq!(fs::read_to_string(d.join("Trip 02.JPG")).unwrap(), "1");

		let up = RenameRule {
			case: Some(Case::Upper),
			find: "trip".into(),
			replace: "x".into(),
			..Default::default()
		};
		assert_eq!(rename_plan(&[d.join("trip day.txt")], &up)[0].to, d.join("X DAY.txt"));
	}

	#[test]
	fn rename_refuses_before_changing_anything() {
		let d = dir("refuse");
		let (a, b, c) = (d.join("a.txt"), d.join("b.txt"), d.join("taken.txt"));
		for p in [&a, &b, &c] {
			fs::write(p, "x").unwrap();
		}
		let same = RenameRule {
			base: Some("taken".into()),
			number: false,
			..Default::default()
		};
		let plan = rename_plan(std::slice::from_ref(&a), &same);
		assert_eq!(
			plan[0].problem.as_deref(),
			Some("a file with this name is already there")
		);
		assert!(rename_all(&plan).is_err());
		assert!(a.exists(), "nothing renamed");
		let clash = RenameRule {
			suffix: "?".into(),
			..Default::default()
		};
		assert!(
			rename_plan(std::slice::from_ref(&b), &clash)[0]
				.problem
				.as_deref()
				.unwrap()
				.contains("can't contain ?")
		);
		let con = RenameRule {
			base: Some("CON".into()),
			..Default::default()
		};
		assert!(rename_plan(&[b], &con)[0].problem.is_some());
	}

	#[test]
	fn the_first_bytes_tell_the_real_type() {
		assert_eq!(sniff(b"\x89PNG\r\n\x1a\n...."), Some("PNG"));
		assert_eq!(sniff(b"\0\0\0\x18ftypheic...."), Some("HEIC"));
		assert_eq!(sniff(b"PK\x03\x04...."), Some("ZIP"));
		assert_eq!(sniff(b"hello"), None);
		let d = dir("info");
		let p = d.join("photo.jpg");
		fs::write(&p, b"\x89PNG\r\n\x1a\nnot really a jpg").unwrap();
		let e = Engines::in_dir(&d, d.join("s"));
		let i = info(&e, &p).unwrap();
		assert_eq!(i.named, "JPG");
		assert_eq!(i.actual.as_deref(), Some("PNG"));
		assert!(!i.matches);
		let doc = d.join("report.docx");
		fs::write(&doc, b"PK\x03\x04 office files are zips").unwrap();
		assert!(info(&e, &doc).unwrap().matches);
	}
}
