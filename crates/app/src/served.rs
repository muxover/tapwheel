//! Byte ranges of shared files for the pages, read straight from disk. A response never holds
//! more than one chunk, whatever the file's size and whatever the request asks for: wry copies
//! a body into a memory stream and WebView2 copies it again, so a whole 2.7 GB video was 5.4 GB.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The most one response carries.
pub const CHUNK: u64 = 4 * 1024 * 1024;

pub struct Part {
	/// 200 for a whole small file, 206 for a range.
	pub status: u16,
	/// `bytes start-end/len`, for a 206.
	pub content_range: Option<String>,
	pub body: Vec<u8>,
}

/// The bytes a `Range` header asks for, cut to one chunk; `None` when there's no usable range.
fn wanted(range: Option<&str>, len: u64) -> Option<(u64, u64)> {
	let (a, b) = range?.strip_prefix("bytes=")?.split_once('-')?;
	let (start, end) = if a.is_empty() {
		// the last n bytes
		let n: u64 = b.parse().ok()?;
		(len.saturating_sub(n), len.checked_sub(1)?)
	} else {
		let start: u64 = a.parse().ok()?;
		(start, b.parse().unwrap_or(u64::MAX).min(len.checked_sub(1)?))
	};
	(start <= end).then_some((start, end.min(start + CHUNK - 1)))
}

/// A request without a range gets the whole file only when that fits in one chunk; otherwise
/// the first chunk, marked as a range, so the page asks for the rest piece by piece.
pub fn read(path: &Path, range: Option<&str>) -> std::io::Result<Part> {
	let mut f = File::open(path)?;
	let len = f.metadata()?.len();
	let (start, end, status) = match wanted(range, len) {
		Some((s, e)) => (s, e, 206),
		None if range.is_some() && len > 0 => {
			return Ok(Part {
				status: 416,
				content_range: Some(format!("bytes */{len}")),
				body: Vec::new(),
			});
		}
		None if len <= CHUNK => (0, len.saturating_sub(1), 200),
		None => (0, CHUNK - 1, 206),
	};
	let mut body = vec![0; if len == 0 { 0 } else { (end - start + 1) as usize }];
	f.seek(SeekFrom::Start(start))?;
	f.read_exact(&mut body)?;
	Ok(Part {
		status,
		content_range: (status == 206).then(|| format!("bytes {start}-{end}/{len}")),
		body,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn big_file(tag: &str, len: u64) -> std::path::PathBuf {
		let p = std::env::temp_dir().join(format!("tw-served-{tag}-{}", std::process::id()));
		let f = File::create(&p).unwrap();
		// sparse where the file system allows, so the test takes no real space
		f.set_len(len).unwrap();
		p
	}

	#[test]
	fn a_file_over_4_gb_is_served_a_chunk_at_a_time() {
		let len = 5 * 1024 * 1024 * 1024;
		let p = big_file("huge", len);
		let whole = read(&p, None).unwrap();
		assert_eq!(whole.status, 206);
		assert_eq!(whole.body.len() as u64, CHUNK);
		assert_eq!(whole.content_range.unwrap(), format!("bytes 0-{}/{len}", CHUNK - 1));
		let open = read(&p, Some("bytes=0-")).unwrap();
		assert_eq!(open.body.len() as u64, CHUNK);
		let far = read(&p, Some("bytes=4294967296-")).unwrap();
		assert_eq!(far.status, 206);
		assert_eq!(
			far.content_range.unwrap(),
			format!("bytes 4294967296-{}/{len}", 4294967296 + CHUNK - 1)
		);
		let tail = read(&p, Some("bytes=-100")).unwrap();
		assert_eq!(tail.body.len(), 100);
		assert_eq!(
			tail.content_range.unwrap(),
			format!("bytes {}-{}/{len}", len - 100, len - 1)
		);
		let _ = std::fs::remove_file(p);
	}

	#[test]
	fn small_files_and_odd_ranges() {
		let p = big_file("small", 10);
		let whole = read(&p, None).unwrap();
		assert_eq!((whole.status, whole.body.len()), (200, 10));
		assert!(whole.content_range.is_none());
		assert_eq!(read(&p, Some("bytes=2-4")).unwrap().body.len(), 3);
		assert_eq!(
			read(&p, Some("bytes=5-100")).unwrap().content_range.unwrap(),
			"bytes 5-9/10"
		);
		assert_eq!(read(&p, Some("bytes=20-")).unwrap().status, 416);
		let empty = big_file("empty", 0);
		assert_eq!(read(&empty, None).unwrap().body.len(), 0);
		let _ = std::fs::remove_file(p);
		let _ = std::fs::remove_file(empty);
	}
}
