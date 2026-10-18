use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::Error;
use crate::kind::ext_of;
use crate::ops::Format;

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
	let entries = walk(inputs)?;
	match format {
		Format::Zip => {
			let mut zip = zip::ZipWriter::new(BufWriter::new(File::create(output)?));
			let opts = zip::write::SimpleFileOptions::default()
				.compression_method(zip::CompressionMethod::Deflated)
				.large_file(true);
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
		Format::Tar | Format::Tgz => {
			let file = BufWriter::new(File::create(output)?);
			if format == Format::Tgz {
				let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
				write_tar(gz, &entries, cancel)?.finish()?.flush()?;
			} else {
				write_tar(file, &entries, cancel)?.flush()?;
			}
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

pub fn extract(input: &Path, dest: &Path, cancel: &AtomicBool) -> Result<(), Error> {
	fs::create_dir_all(dest)?;
	match ext_of(input).as_str() {
		"zip" => {
			let mut zip = zip::ZipArchive::new(BufReader::new(File::open(input)?)).map_err(zip_err)?;
			for i in 0..zip.len() {
				check(cancel)?;
				let mut f = zip.by_index(i).map_err(zip_err)?;
				let name = f.name().map_err(zip_err)?.into_owned();
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
		"tar" => untar(BufReader::new(File::open(input)?), dest, cancel)?,
		"tgz" => untar(
			flate2::read::GzDecoder::new(BufReader::new(File::open(input)?)),
			dest,
			cancel,
		)?,
		"gz" => {
			let name = crate::plan::stem_of(input);
			let mut gz = flate2::read::GzDecoder::new(BufReader::new(File::open(input)?));
			io::copy(&mut gz, &mut BufWriter::new(File::create(dest.join(name))?))?;
		}
		other => return Err(Error::Engine(format!("can't open .{other} archives"))),
	}
	Ok(())
}

fn untar<R: Read>(r: R, dest: &Path, cancel: &AtomicBool) -> Result<(), Error> {
	let mut tar = tar::Archive::new(r);
	for entry in tar.entries()? {
		check(cancel)?;
		let mut entry = entry?;
		// unpack_in refuses paths that escape `dest`
		entry.unpack_in(dest)?;
	}
	Ok(())
}

/// Archive to archive: unpack into a scratch folder, then pack its contents.
pub fn repack(input: &Path, format: Format, output: &Path, scratch: &Path, cancel: &AtomicBool) -> Result<(), Error> {
	extract(input, scratch, cancel)?;
	let mut items: Vec<PathBuf> = fs::read_dir(scratch)?
		.map(|e| e.map(|e| e.path()))
		.collect::<Result<_, _>>()?;
	items.sort();
	pack(&items, format, output, cancel)
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
		for f in [Format::Zip, Format::SevenZ, Format::Tar, Format::Tgz] {
			let out = root.join(format!("out.{}", f.ext()));
			pack(std::slice::from_ref(&folder), f, &out, &never).unwrap();
			let dest = root.join(format!("x-{}", f.ext()));
			extract(&out, &dest, &never).unwrap();
			assert_eq!(fs::read(dest.join("photos/a.txt")).unwrap(), b"alpha", "{f:?}");
			assert_eq!(fs::read(dest.join("photos/trip/b.txt")).unwrap().len(), 10_000, "{f:?}");
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
		extract(&out, &dest, &never).unwrap();
		assert_eq!(fs::read(dest.join("photos/a.txt")).unwrap(), b"alpha");
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
