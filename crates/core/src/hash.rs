//! Checksums for the Actions wheel.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::Error;
use crate::recipe::HashKind;

fn digest<D: Digest>(path: &Path) -> Result<String, Error> {
	let mut r = BufReader::with_capacity(1 << 20, File::open(path)?);
	let mut h = D::new();
	let mut buf = vec![0u8; 1 << 20];
	loop {
		let n = r.read(&mut buf)?;
		if n == 0 {
			break;
		}
		h.update(&buf[..n]);
	}
	Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn hash(path: &Path, kind: HashKind) -> Result<String, Error> {
	match kind {
		HashKind::Sha256 => digest::<Sha256>(path),
		HashKind::Sha1 => digest::<Sha1>(path),
		HashKind::Md5 => digest::<Md5>(path),
	}
}

/// Which kind of checksum a pasted value looks like, by its length.
pub fn kind_of(value: &str) -> Option<HashKind> {
	let v = value.trim();
	if !v.chars().all(|c| c.is_ascii_hexdigit()) {
		return None;
	}
	match v.len() {
		64 => Some(HashKind::Sha256),
		40 => Some(HashKind::Sha1),
		32 => Some(HashKind::Md5),
		_ => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn known_digests() {
		let p = std::env::temp_dir().join(format!("tw-hash-{}.txt", std::process::id()));
		std::fs::write(&p, b"abc").unwrap();
		assert_eq!(
			hash(&p, HashKind::Sha256).unwrap(),
			"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
		);
		assert_eq!(hash(&p, HashKind::Md5).unwrap(), "900150983cd24fb0d6963f7d28e17f72");
		assert_eq!(
			hash(&p, HashKind::Sha1).unwrap(),
			"a9993e364706816aba3e25717850c26c9cd0d89d"
		);
		std::fs::remove_file(p).unwrap();
		assert_eq!(kind_of(" 900150983CD24FB0D6963F7D28E17F72\n"), Some(HashKind::Md5));
		assert_eq!(kind_of("hello"), None);
	}
}
