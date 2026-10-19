use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::Error;
use crate::format::Format;
use crate::kind::ext_of;

struct Entry {
	path: PathBuf,
	name: String,
	dir: bool,
}

/// Every file and folder under the inputs, named relative to the inputs' parent.
fn walk(inputs: &[PathBuf]) -> io::Result<Vec<Entry>> {
	let mut out = Vec::new();
	for input in inputs {
		let base = input.parent().unwrap_or(Path::new(""));
		walk_one(input, base, &mut out)?;
	}
	Ok(out)
}

fn walk_one(path: &Path, base: &Path, out: &mut Vec<Entry>) -> io::Result<()> {
	let rel = path.strip_prefix(base).unwrap_or(path);
	let name = rel
		.components()
		.map(|c| c.as_os_str().to_string_lossy())
		.collect::<Vec<_>>()
		.join("/");
	let meta = fs::symlink_metadata(path)?;
	if meta.is_dir() {
		out.push(Entry {
			path: path.to_path_buf(),
			name: name.clone(),
			dir: true,
		});
		let mut children: Vec<_> = fs::read_dir(path)?.collect::<Result<_, _>>()?;
		children.sort_by_key(|e| e.file_name());
		for child in children {
			walk_one(&child.path(), base, out)?;
		}
	} else if meta.is_file() {
		out.push(Entry {
			path: path.to_path_buf(),
			name,
			dir: false,
		});
	}
	Ok(())
}

fn check(cancel: &AtomicBool) -> Result<(), Error> {
	if cancel.load(Ordering::Relaxed) {
		Err(Error::Cancelled)
	} else {
		Ok(())
	}
}

pub fn pack(inputs: &[PathBuf], format: Format, output: &Path, cancel: &AtomicBool) -> Result<(), Error> {
	pack_with(inputs, format, None, output, cancel)
}

/// Packs files and folders. With a password, ZIP uses AES-256 and 7Z encrypts names too.
pub fn pack_with(
	inputs: &[PathBuf],
	format: Format,
	password: Option<&str>,
	output: &Path,
	cancel: &AtomicBool,
) -> Result<(), Error> {
	let entries = walk(inputs)?;
	match format {
		Format::Zip => {
			let mut zip = zip::ZipWriter::new(BufWriter::new(File::create(output)?));
			let mut opts = zip::write::SimpleFileOptions::default()
				.compression_method(zip::CompressionMethod::Deflated)
				.large_file(true);
			if let Some(pw) = password {
				opts = opts.with_aes_encryption(zip::AesMode::Aes256, pw);
			}
			for e in &entries {
				check(cancel)?;
				if e.dir {
					zip.add_directory(&e.name, opts).map_err(zip_err)?;
				} else {
					zip.start_file(&e.name, opts).map_err(zip_err)?;
					io::copy(&mut BufReader::new(File::open(&e.path)?), &mut zip)?;
				}
			}
			zip.finish().map_err(zip_err)?.flush()?;
		}
		Format::SevenZ => {
			let mut w = sevenz_rust2::ArchiveWriter::create(output).map_err(sz_err)?;
			if let Some(pw) = password {
				w.set_content_methods(vec![
					sevenz_rust2::encoder_options::AesEncoderOptions::new(pw.into()).into(),
					sevenz_rust2::EncoderMethod::LZMA2.into(),
				]);
				w.set_encrypt_header(true);
			}
			for e in &entries {
				check(cancel)?;
				let entry = sevenz_rust2::ArchiveEntry::from_path(&e.path, e.name.clone());
				if e.dir {
					w.push_archive_entry::<&[u8]>(entry, None).map_err(sz_err)?;
				} else {
					w.push_archive_entry(entry, Some(BufReader::new(File::open(&e.path)?)))
						.map_err(sz_err)?;
				}
			}
			w.finish()?;
		}
		Format::Tar => write_tar(BufWriter::new(File::create(output)?), &entries, cancel)?.flush()?,
		Format::Tgz => {
			let gz =
				flate2::write::GzEncoder::new(BufWriter::new(File::create(output)?), flate2::Compression::default());
			write_tar(gz, &entries, cancel)?.finish()?.flush()?;
		}
		Format::Txz => {
			let xz = xz2::write::XzEncoder::new(BufWriter::new(File::create(output)?), 6);
			write_tar(xz, &entries, cancel)?.finish()?.flush()?;
		}
		Format::Tbz2 => {
			let bz = bzip2::write::BzEncoder::new(BufWriter::new(File::create(output)?), bzip2::Compression::default());
			write_tar(bz, &entries, cancel)?.finish()?.flush()?;
		}
		Format::Tzst => {
			let z = zstd::Encoder::new(BufWriter::new(File::create(output)?), 9)?;
			write_tar(z, &entries, cancel)?.finish()?.flush()?;
		}
		_ => {
			return Err(Error::Engine(format!("{} is not an archive", format.label())));
		}
	}
	Ok(())
}

fn write_tar<W: Write>(w: W, entries: &[Entry], cancel: &AtomicBool) -> Result<W, Error> {
	let mut tar = tar::Builder::new(w);
	tar.follow_symlinks(false);
	for e in entries {
		check(cancel)?;
		if e.dir {
			tar.append_dir(&e.name, &e.path)?;
		} else {
			tar.append_path_with_name(&e.path, &e.name)?;
		}
	}
	Ok(tar.into_inner()?)
}

/// Joins an entry name onto `dest`, refusing anything that would land outside it.
fn safe_join(dest: &Path, name: &str) -> Option<PathBuf> {
	let rel = Path::new(name.trim_start_matches(['/', '\\']));
	let mut out = dest.to_path_buf();
	for c in rel.components() {
		match c {
			Component::Normal(p) => out.push(p),
			Component::CurDir => {}
			_ => return None,
		}
	}
	(out != dest).then_some(out)
}

/// Unpacks everything, or only the entries named in `only` (a folder brings what's inside it).
pub fn extract(input: &Path, dest: &Path, only: &[String], cancel: &AtomicBool) -> Result<(), Error> {
	let wanted = |name: &str| {
		let name = name.trim_end_matches('/');
		only.is_empty()
			|| only.iter().any(|o| {
				let o = o.trim_end_matches('/');
				name == o || name.starts_with(&format!("{o}/"))
			})
	};
	fs::create_dir_all(dest)?;
	match ext_of(input).as_str() {
		"zip" | "jar" | "apk" | "cbz" | "nupkg" | "vsix" | "whl" => {
			let mut zip = zip::ZipArchive::new(BufReader::new(File::open(input)?)).map_err(zip_err)?;
			for i in 0..zip.len() {
				check(cancel)?;
				let mut f = zip.by_index(i).map_err(|e| match e {
					zip::result::ZipError::UnsupportedArchive(m) if m.contains("assword") => {
						Error::Engine("this archive needs a password".into())
					}
					other => zip_err(other),
				})?;
				let name = f.name().map_err(zip_err)?.into_owned();
				if !wanted(&name) {
					continue;
				}
				let Some(path) = safe_join(dest, &name) else {
					continue;
				};
				if f.is_dir() {
					fs::create_dir_all(&path)?;
				} else {
					if let Some(p) = path.parent() {
						fs::create_dir_all(p)?;
					}
					io::copy(&mut f, &mut BufWriter::new(File::create(&path)?))?;
				}
			}
		}
		"7z" => {
			sevenz_rust2::decompress_file_with_extract_fn(input, dest, |entry, reader, _| {
				if cancel.load(Ordering::Relaxed) {
					return Err(sevenz_rust2::Error::Other("cancelled".into()));
				}
				if !wanted(entry.name()) {
					return Ok(true);
				}
				let Some(path) = safe_join(dest, entry.name()) else {
					return Ok(true);
				};
				if entry.is_directory() {
					fs::create_dir_all(&path)?;
				} else {
					if let Some(p) = path.parent() {
						fs::create_dir_all(p)?;
					}
					io::copy(reader, &mut BufWriter::new(File::create(&path)?))?;
				}
				Ok(true)
			})
			.map_err(|e| {
				if cancel.load(Ordering::Relaxed) {
					Error::Cancelled
				} else {
					sz_err(e)
				}
			})?;
		}
		"tar" => untar(BufReader::new(File::open(input)?), dest, &wanted, cancel)?,
		"tgz" => untar(
			flate2::read::MultiGzDecoder::new(BufReader::new(File::open(input)?)),
			dest,
			&wanted,
			cancel,
		)?,
		"txz" => untar(
			xz2::read::XzDecoder::new_multi_decoder(BufReader::new(File::open(input)?)),
			dest,
			&wanted,
			cancel,
		)?,
		"tbz" | "tbz2" => untar(
			bzip2::read::MultiBzDecoder::new(BufReader::new(File::open(input)?)),
			dest,
			&wanted,
			cancel,
		)?,
		"tzst" => untar(
			zstd::Decoder::new(BufReader::new(File::open(input)?))?,
			dest,
			&wanted,
			cancel,
		)?,
		"gz" | "xz" | "bz2" | "zst" => {
			let name = crate::plan::stem_of(input);
			let mut r = single_stream(input)?;
			io::copy(&mut r, &mut BufWriter::new(File::create(dest.join(name))?))?;
		}
		other => return Err(Error::Engine(format!("can't open .{other} archives"))),
	}
	Ok(())
}

fn untar<R: Read>(r: R, dest: &Path, wanted: &dyn Fn(&str) -> bool, cancel: &AtomicBool) -> Result<(), Error> {
	let mut tar = tar::Archive::new(r);
	for entry in tar.entries()? {
		check(cancel)?;
		let mut entry = entry?;
		if !wanted(&entry.path()?.to_string_lossy()) {
			continue;
		}
		// unpack_in refuses paths that escape `dest`
		entry.unpack_in(dest)?;
	}
	Ok(())
}

/// Archive to archive: unpack into a scratch folder, then pack its contents.
pub fn repack(input: &Path, format: Format, output: &Path, scratch: &Path, cancel: &AtomicBool) -> Result<(), Error> {
	extract(input, scratch, &[], cancel)?;
	let mut items: Vec<PathBuf> = fs::read_dir(scratch)?
		.map(|e| e.map(|e| e.path()))
		.collect::<Result<_, _>>()?;
	items.sort();
	pack(&items, format, output, cancel)
}

/// A reader that undoes one compressed stream (`.gz`, `.xz`, `.bz2`, `.zst`).
fn single_stream(input: &Path) -> Result<Box<dyn Read>, Error> {
	let f = BufReader::new(File::open(input)?);
	Ok(match ext_of(input).as_str() {
		"gz" | "tgz" => Box::new(flate2::read::MultiGzDecoder::new(f)),
		"xz" | "txz" => Box::new(xz2::read::XzDecoder::new_multi_decoder(f)),
		"bz2" | "tbz" | "tbz2" => Box::new(bzip2::read::MultiBzDecoder::new(f)),
		"zst" | "tzst" => Box::new(zstd::Decoder::new(f)?),
		_ => Box::new(f),
	})
}

/// One line of an archive's table of contents.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Item {
	pub name: String,
	pub size: u64,
	pub dir: bool,
	/// Seconds since 1970, when the archive records it.
	pub modified: Option<i64>,
}

/// Everything inside an archive, without unpacking it.
pub fn list(input: &Path) -> Result<Vec<Item>, Error> {
	let mut out = Vec::new();
	match ext_of(input).as_str() {
		"zip" | "jar" | "apk" | "cbz" | "nupkg" | "vsix" | "whl" => {
			let mut zip = zip::ZipArchive::new(BufReader::new(File::open(input)?)).map_err(zip_err)?;
			for i in 0..zip.len() {
				let f = zip.by_index_raw(i).map_err(zip_err)?;
				let modified = f.last_modified().and_then(|d| {
					let t = time::PrimitiveDateTime::try_from(d).ok()?;
					Some(t.assume_utc().unix_timestamp())
				});
				out.push(Item {
					name: f.name().map_err(zip_err)?.into_owned(),
					size: f.size(),
					dir: f.is_dir(),
					modified,
				});
			}
		}
		"7z" => {
			let archive = sevenz_rust2::Archive::open(input).map_err(sz_err)?;
			for e in &archive.files {
				out.push(Item {
					name: e.name().to_string(),
					size: e.size(),
					dir: e.is_directory(),
					modified: e
						.has_last_modified_date
						.then(|| std::time::SystemTime::from(e.last_modified_date()))
						.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
						.map(|d| d.as_secs() as i64),
				});
			}
		}
		"tar" | "tgz" | "txz" | "tbz" | "tbz2" | "tzst" => {
			let mut tar = tar::Archive::new(single_stream(input)?);
			for entry in tar.entries()? {
				let entry = entry?;
				let h = entry.header();
				out.push(Item {
					name: entry.path()?.to_string_lossy().into_owned(),
					size: h.size().unwrap_or(0),
					dir: h.entry_type().is_dir(),
					modified: h.mtime().ok().map(|t| t as i64),
				});
			}
		}
		"gz" | "xz" | "bz2" | "zst" => out.push(Item {
			name: crate::plan::stem_of(input),
			size: 0,
			dir: false,
			modified: None,
		}),
		other => return Err(Error::Engine(format!("can't open .{other} archives"))),
	}
	Ok(out)
}

/// Reads every file in the archive to its end, so damage anywhere shows up. Returns the file count.
pub fn test(input: &Path, cancel: &AtomicBool) -> Result<usize, Error> {
	let mut n = 0;
	let mut sink = io::sink();
	match ext_of(input).as_str() {
		"zip" | "jar" | "apk" | "cbz" | "nupkg" | "vsix" | "whl" => {
			let mut zip = zip::ZipArchive::new(BufReader::new(File::open(input)?)).map_err(zip_err)?;
			for i in 0..zip.len() {
				check(cancel)?;
				let mut f = zip.by_index(i).map_err(zip_err)?;
				let name = f.name().map(|n| n.into_owned()).unwrap_or_default();
				// reading to the end checks the CRC
				io::copy(&mut f, &mut sink).map_err(|e| Error::Engine(format!("{name} is damaged: {e}")))?;
				n += 1;
			}
		}
		"7z" => {
			sevenz_rust2::decompress_file_with_extract_fn(input, Path::new("."), |entry, reader, _| {
				if cancel.load(Ordering::Relaxed) {
					return Err(sevenz_rust2::Error::Other("cancelled".into()));
				}
				if !entry.is_directory() {
					io::copy(reader, &mut io::sink())?;
					n += 1;
				}
				Ok(true)
			})
			.map_err(|e| {
				if cancel.load(Ordering::Relaxed) {
					Error::Cancelled
				} else {
					sz_err(e)
				}
			})?;
		}
		"tar" | "tgz" | "txz" | "tbz" | "tbz2" | "tzst" => {
			let mut tar = tar::Archive::new(single_stream(input)?);
			for entry in tar.entries()? {
				check(cancel)?;
				let mut entry = entry?;
				if entry.header().entry_type().is_dir() {
					continue;
				}
				io::copy(&mut entry, &mut sink)?;
				n += 1;
			}
		}
		"gz" | "xz" | "bz2" | "zst" => {
			io::copy(&mut single_stream(input)?, &mut sink)?;
			n = 1;
		}
		other => return Err(Error::Engine(format!("can't open .{other} archives"))),
	}
	Ok(n)
}

/// Cuts a file into numbered parts (`name.001`, `name.002`, ...) that 7-Zip joins back.
pub fn split(input: &Path, part_mb: u64, dir: &Path, cancel: &AtomicBool) -> Result<usize, Error> {
	fs::create_dir_all(dir)?;
	let name = input
		.file_name()
		.map(|n| n.to_string_lossy().into_owned())
		.unwrap_or_default();
	let size = part_mb.max(1) * 1024 * 1024;
	let mut r = BufReader::new(File::open(input)?);
	let mut n = 0;
	loop {
		check(cancel)?;
		let mut chunk = (&mut r).take(size);
		let path = dir.join(format!("{name}.{:03}", n + 1));
		let mut w = BufWriter::new(File::create(&path)?);
		let copied = io::copy(&mut chunk, &mut w)?;
		w.flush()?;
		if copied == 0 {
			drop(w);
			fs::remove_file(&path)?;
			break;
		}
		n += 1;
		if copied < size {
			break;
		}
	}
	Ok(n)
}

fn zip_err(e: zip::result::ZipError) -> Error {
	Error::Engine(format!("ZIP: {e}"))
}

fn sz_err(e: sevenz_rust2::Error) -> Error {
	Error::Engine(format!("7Z: {e}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn concatenated_streams_come_out_whole() {
		let dir = std::env::temp_dir().join(format!("tw-multi-{}", std::process::id()));
		let _ = fs::remove_dir_all(&dir);
		fs::create_dir_all(&dir).unwrap();
		let parts: [&[u8]; 2] = [b"part one\n", b"part two\n"];
		let mut files = Vec::new();
		for ext in ["gz", "bz2", "xz"] {
			let mut all = Vec::new();
			for p in parts {
				let mut w: Box<dyn Write> = match ext {
					"gz" => Box::new(flate2::write::GzEncoder::new(&mut all, flate2::Compression::default())),
					"bz2" => Box::new(bzip2::write::BzEncoder::new(&mut all, bzip2::Compression::default())),
					_ => Box::new(xz2::write::XzEncoder::new(&mut all, 6)),
				};
				w.write_all(p).unwrap();
				drop(w);
			}
			let f = dir.join(format!("log.txt.{ext}"));
			fs::write(&f, all).unwrap();
			files.push(f);
		}
		for f in files {
			let out = dir.join(format!("out-{}", ext_of(&f)));
			extract(&f, &out, &[], &AtomicBool::new(false)).unwrap();
			assert_eq!(fs::read(out.join("log.txt")).unwrap(), b"part one\npart two\n", "{f:?}");
			assert!(test(&f, &AtomicBool::new(false)).is_ok(), "{f:?}");
		}
		let _ = fs::remove_dir_all(dir);
	}

	fn fixture() -> (PathBuf, PathBuf) {
		// tests run in parallel and Windows clocks are coarse, so a counter keeps folders apart
		static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
		let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
		let root = std::env::temp_dir().join(format!("tw-arc-{}-{n}", std::process::id()));
		let _ = fs::remove_dir_all(&root);
		let folder = root.join("photos");
		fs::create_dir_all(folder.join("trip")).unwrap();
		fs::write(folder.join("a.txt"), b"alpha").unwrap();
		fs::write(folder.join("trip").join("b.txt"), vec![7u8; 10_000]).unwrap();
		(root, folder)
	}

	#[test]
	fn every_format_round_trips() {
		let (root, folder) = fixture();
		let never = AtomicBool::new(false);
		for f in [
			Format::Zip,
			Format::SevenZ,
			Format::Tar,
			Format::Tgz,
			Format::Txz,
			Format::Tbz2,
			Format::Tzst,
		] {
			let out = root.join(format!("out.{}", f.ext()));
			pack(std::slice::from_ref(&folder), f, &out, &never).unwrap();
			let dest = root.join(format!("x-{}", f.ext()));
			extract(&out, &dest, &[], &never).unwrap();
			assert_eq!(fs::read(dest.join("photos/a.txt")).unwrap(), b"alpha", "{f:?}");
			assert_eq!(fs::read(dest.join("photos/trip/b.txt")).unwrap().len(), 10_000, "{f:?}");
			// only what's picked: a folder brings its contents, nothing else comes along
			let some = root.join(format!("some-{}", f.ext()));
			extract(&out, &some, &["photos/trip".into()], &never).unwrap();
			assert!(some.join("photos/trip/b.txt").exists(), "{f:?}");
			assert!(!some.join("photos/a.txt").exists(), "{f:?}");
		}
		fs::remove_dir_all(root).unwrap();
	}

	#[test]
	fn repack_zip_to_7z() {
		let (root, folder) = fixture();
		let never = AtomicBool::new(false);
		let zip = root.join("in.zip");
		pack(&[folder], Format::Zip, &zip, &never).unwrap();
		let out = root.join("in.7z");
		repack(&zip, Format::SevenZ, &out, &root.join("scratch"), &never).unwrap();
		let dest = root.join("check");
		extract(&out, &dest, &[], &never).unwrap();
		assert_eq!(fs::read(dest.join("photos/a.txt")).unwrap(), b"alpha");
		fs::remove_dir_all(root).unwrap();
	}

	#[test]
	fn passwords_list_test_and_split() {
		let (root, folder) = fixture();
		let never = AtomicBool::new(false);
		let zip = root.join("locked.zip");
		pack_with(
			std::slice::from_ref(&folder),
			Format::Zip,
			Some("hunter2"),
			&zip,
			&never,
		)
		.unwrap();
		let names: Vec<String> = list(&zip).unwrap().into_iter().map(|i| i.name).collect();
		assert!(names.contains(&"photos/a.txt".to_string()));
		let err = extract(&zip, &root.join("nope"), &[], &never).unwrap_err();
		assert!(err.to_string().contains("password"), "{err}");

		let sz = root.join("locked.7z");
		pack_with(
			std::slice::from_ref(&folder),
			Format::SevenZ,
			Some("hunter2"),
			&sz,
			&never,
		)
		.unwrap();
		assert!(list(&sz).is_err(), "7z hides names behind the password");

		let plain = root.join("plain.tar.zst");
		pack(std::slice::from_ref(&folder), Format::Tzst, &plain, &never).unwrap();
		assert_eq!(test(&plain, &never).unwrap(), 2);

		let parts = root.join("parts");
		let big = root.join("big.bin");
		fs::write(&big, vec![1u8; 2 * 1024 * 1024 + 10]).unwrap();
		assert_eq!(split(&big, 1, &parts, &never).unwrap(), 3);
		assert_eq!(fs::metadata(parts.join("big.bin.003")).unwrap().len(), 10);
		fs::remove_dir_all(root).unwrap();
	}

	#[test]
	fn zip_slip_is_skipped() {
		let dest = Path::new("/safe/out");
		assert_eq!(safe_join(dest, "../evil.txt"), None);
		assert_eq!(safe_join(dest, "a/../../evil"), None);
		assert_eq!(
			safe_join(dest, "/abs/ok.txt"),
			Some(PathBuf::from("/safe/out/abs/ok.txt"))
		);
		assert_eq!(safe_join(dest, "./x/y"), Some(PathBuf::from("/safe/out/x/y")));
	}

	#[test]
	fn cancel_stops_packing() {
		let (root, folder) = fixture();
		let stop = AtomicBool::new(true);
		let err = pack(&[folder], Format::Zip, &root.join("c.zip"), &stop).unwrap_err();
		assert!(matches!(err, Error::Cancelled));
		fs::remove_dir_all(root).unwrap();
	}
}
