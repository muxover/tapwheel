//! Reading and removing photo metadata without re-encoding the picture.
//!
//! JPG, PNG and WebP keep their pixels byte for byte: only the EXIF, XMP and text blocks change.

use std::ops::Range;

/// Where the EXIF (TIFF-structured) block sits inside a file.
pub fn exif_range(bytes: &[u8]) -> Option<Range<usize>> {
	if bytes.starts_with(&[0xff, 0xd8]) {
		jpeg_segments(bytes)
			.into_iter()
			.find(|s| s.marker == 0xe1 && bytes[s.data.clone()].starts_with(b"Exif\0\0"))
			.map(|s| s.data.start + 6..s.data.end)
	} else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
		png_chunks(bytes)
			.into_iter()
			.find(|c| &c.kind == b"eXIf")
			.map(|c| c.data)
	} else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(&b"WEBP"[..]) {
		riff_chunks(bytes).into_iter().find(|c| &c.kind == b"EXIF").map(|c| {
			// some writers keep the JPEG-style "Exif\0\0" prefix
			let start = if bytes[c.data.clone()].starts_with(b"Exif\0\0") {
				c.data.start + 6
			} else {
				c.data.start
			};
			start..c.data.end
		})
	} else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
		Some(0..bytes.len())
	} else {
		None
	}
}

struct Segment {
	marker: u8,
	/// The whole segment, marker included.
	whole: Range<usize>,
	data: Range<usize>,
}

/// JPEG segments up to the start of the picture data.
fn jpeg_segments(b: &[u8]) -> Vec<Segment> {
	let mut out = Vec::new();
	let mut i = 2;
	while i + 4 <= b.len() && b[i] == 0xff {
		let marker = b[i + 1];
		if marker == 0xda || marker == 0xd9 {
			break;
		}
		if (0xd0..=0xd7).contains(&marker) || marker == 0x01 || marker == 0xff {
			i += if marker == 0xff { 1 } else { 2 };
			continue;
		}
		let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
		if len < 2 || i + 2 + len > b.len() {
			break;
		}
		out.push(Segment {
			marker,
			whole: i..i + 2 + len,
			data: i + 4..i + 2 + len,
		});
		i += 2 + len;
	}
	out
}

struct Chunk {
	kind: [u8; 4],
	whole: Range<usize>,
	data: Range<usize>,
}

fn png_chunks(b: &[u8]) -> Vec<Chunk> {
	let mut out = Vec::new();
	let mut i = 8;
	while i + 12 <= b.len() {
		let len = u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]) as usize;
		let end = i + 12 + len;
		if end > b.len() {
			break;
		}
		let kind = [b[i + 4], b[i + 5], b[i + 6], b[i + 7]];
		out.push(Chunk {
			kind,
			whole: i..end,
			data: i + 8..i + 8 + len,
		});
		i = end;
		if &kind == b"IEND" {
			break;
		}
	}
	out
}

fn riff_chunks(b: &[u8]) -> Vec<Chunk> {
	let mut out = Vec::new();
	let mut i = 12;
	while i + 8 <= b.len() {
		let len = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
		let padded = len + (len & 1);
		let end = (i + 8 + padded).min(b.len());
		out.push(Chunk {
			kind: [b[i], b[i + 1], b[i + 2], b[i + 3]],
			whole: i..end,
			data: i + 8..(i + 8 + len).min(b.len()),
		});
		i = end;
	}
	out
}

/// A small reader and patcher for the TIFF structure inside EXIF.
struct Tiff<'a> {
	b: &'a [u8],
	le: bool,
}

const TYPE_SIZES: [usize; 14] = [0, 1, 1, 2, 4, 8, 1, 1, 2, 4, 8, 4, 8, 4];

impl<'a> Tiff<'a> {
	fn new(b: &'a [u8]) -> Option<Self> {
		let le = match b.get(0..2)? {
			b"II" => true,
			b"MM" => false,
			_ => return None,
		};
		Some(Tiff { b, le })
	}

	fn u16(&self, at: usize) -> Option<u16> {
		let s = self.b.get(at..at + 2)?;
		Some(if self.le {
			u16::from_le_bytes([s[0], s[1]])
		} else {
			u16::from_be_bytes([s[0], s[1]])
		})
	}

	fn u32(&self, at: usize) -> Option<u32> {
		let s = self.b.get(at..at + 4)?;
		Some(if self.le {
			u32::from_le_bytes([s[0], s[1], s[2], s[3]])
		} else {
			u32::from_be_bytes([s[0], s[1], s[2], s[3]])
		})
	}

	fn ifd0(&self) -> Option<usize> {
		self.u32(4).map(|o| o as usize)
	}

	/// (tag, entry offset) for each entry of the directory at `ifd`.
	fn entries(&self, ifd: usize) -> Vec<(u16, usize)> {
		let Some(n) = self.u16(ifd) else {
			return Vec::new();
		};
		(0..n as usize)
			.map(|i| ifd + 2 + i * 12)
			.filter(|&e| e + 12 <= self.b.len())
			.filter_map(|e| self.u16(e).map(|t| (t, e)))
			.collect()
	}

	fn find(&self, ifd: usize, tag: u16) -> Option<usize> {
		self.entries(ifd).into_iter().find(|(t, _)| *t == tag).map(|(_, e)| e)
	}

	/// The bytes an entry's value occupies, inline or elsewhere.
	fn value_range(&self, entry: usize) -> Option<Range<usize>> {
		let kind = self.u16(entry + 2)? as usize;
		let count = self.u32(entry + 4)? as usize;
		let size = TYPE_SIZES.get(kind).copied().unwrap_or(1).checked_mul(count)?;
		if size <= 4 {
			Some(entry + 8..entry + 8 + size)
		} else {
			let at = self.u32(entry + 8)? as usize;
			(at + size <= self.b.len()).then_some(at..at + size)
		}
	}
}

const ORIENTATION: u16 = 0x0112;
const GPS_IFD: u16 = 0x8825;

/// EXIF orientation, 1 to 8, where 1 is upright.
pub fn orientation(file: &[u8]) -> u16 {
	let Some(r) = exif_range(file) else { return 1 };
	let Some(t) = Tiff::new(&file[r]) else { return 1 };
	t.ifd0()
		.and_then(|ifd| t.find(ifd, ORIENTATION))
		.and_then(|e| t.u16(e + 8))
		.filter(|o| (1..=8).contains(o))
		.unwrap_or(1)
}

/// FFmpeg filter that turns a picture with this EXIF orientation upright.
pub fn upright_filter(orientation: u16) -> &'static str {
	match orientation {
		2 => "hflip",
		3 => "hflip,vflip",
		4 => "vflip",
		5 => "transpose=cclock_flip",
		6 => "transpose=clock",
		7 => "transpose=clock_flip",
		8 => "transpose=cclock",
		_ => "",
	}
}

fn write_u16(b: &mut [u8], at: usize, v: u16, le: bool) {
	let bytes = if le { v.to_le_bytes() } else { v.to_be_bytes() };
	b[at..at + 2].copy_from_slice(&bytes);
}

/// Copy of an EXIF block, set upright and, if asked, without location.
pub fn tidy_exif(exif: &[u8], drop_location: bool) -> Vec<u8> {
	let mut out = exif.to_vec();
	let Some(t) = Tiff::new(exif) else { return out };
	let le = t.le;
	let Some(ifd) = t.ifd0() else { return out };
	if let Some(e) = t.find(ifd, ORIENTATION) {
		write_u16(&mut out, e + 8, 1, le);
	}
	if drop_location
		&& let Some(e) = t.find(ifd, GPS_IFD)
		&& let Some(gps) = t.u32(e + 8).map(|o| o as usize)
	{
		for (_, entry) in t.entries(gps) {
			if let Some(r) = t.value_range(entry) {
				out[r].fill(0);
			}
			out[entry..entry + 12].fill(0);
		}
		if gps + 2 <= out.len() {
			write_u16(&mut out, gps, 0, le);
		}
	}
	out
}

/// Whether a file carries GPS coordinates.
pub fn has_location(file: &[u8]) -> bool {
	let Some(r) = exif_range(file) else { return false };
	let Some(t) = Tiff::new(&file[r]) else { return false };
	t.ifd0()
		.and_then(|ifd| t.find(ifd, GPS_IFD))
		.and_then(|e| t.u32(e + 8))
		.is_some_and(|gps| !t.entries(gps as usize).is_empty())
}

/// Removes location only, keeping every other detail. `None` when the format isn't handled losslessly.
pub fn remove_location(file: &[u8]) -> Option<Vec<u8>> {
	let r = exif_range(file)?;
	let tidy = {
		// keep the orientation as it was; only the location goes
		let Some(t) = Tiff::new(&file[r.clone()]) else {
			return Some(file.to_vec());
		};
		let o = t.ifd0().and_then(|i| t.find(i, ORIENTATION)).and_then(|e| t.u16(e + 8));
		let mut x = tidy_exif(&file[r.clone()], true);
		if let (Some(o), Some(e)) = (o, t.ifd0().and_then(|i| t.find(i, ORIENTATION))) {
			write_u16(&mut x, e + 8, o, t.le);
		}
		x
	};
	let mut out = file.to_vec();
	out[r].copy_from_slice(&tidy);
	Some(out)
}

/// Removes EXIF, XMP, comments and text, keeping the colour profile and the pixels.
/// Pictures that rely on EXIF to stand upright keep only that one detail.
pub fn remove_all(file: &[u8]) -> Option<Vec<u8>> {
	let turn = orientation(file);
	if file.starts_with(&[0xff, 0xd8]) {
		let mut out = vec![0xff, 0xd8];
		let segs = jpeg_segments(file);
		let body = segs.last().map(|s| s.whole.end).unwrap_or(2);
		if turn != 1 {
			out.extend(jpeg_orientation_app1(turn));
		}
		for s in &segs {
			let data = &file[s.data.clone()];
			let drop = matches!(s.marker, 0xe1 | 0xed | 0xfe)
				|| (s.marker == 0xe0 && !data.starts_with(b"JFIF"))
				|| (0xe3..=0xef).contains(&s.marker) && s.marker != 0xee;
			if !drop {
				out.extend_from_slice(&file[s.whole.clone()]);
			}
		}
		out.extend_from_slice(&file[body..]);
		Some(out)
	} else if file.starts_with(b"\x89PNG\r\n\x1a\n") {
		let mut out = file[..8].to_vec();
		for c in png_chunks(file) {
			if !matches!(&c.kind, b"eXIf" | b"tEXt" | b"iTXt" | b"zTXt" | b"tIME") {
				out.extend_from_slice(&file[c.whole]);
			}
		}
		Some(out)
	} else if file.starts_with(b"RIFF") && file.get(8..12) == Some(&b"WEBP"[..]) {
		let mut body = Vec::new();
		for c in riff_chunks(file) {
			match &c.kind {
				b"EXIF" | b"XMP " => {}
				b"VP8X" => {
					let mut x = file[c.whole].to_vec();
					// clear the EXIF and XMP flags
					if x.len() > 8 {
						x[8] &= !0b0000_1100;
					}
					body.extend(x);
				}
				_ => body.extend_from_slice(&file[c.whole]),
			}
		}
		let mut out = b"RIFF".to_vec();
		out.extend(((body.len() + 4) as u32).to_le_bytes());
		out.extend(b"WEBP");
		out.extend(body);
		Some(out)
	} else {
		None
	}
}

/// A minimal EXIF segment holding only an orientation.
fn jpeg_orientation_app1(o: u16) -> Vec<u8> {
	let mut tiff = b"MM\0*\0\0\0\x08".to_vec();
	tiff.extend(1u16.to_be_bytes());
	tiff.extend(ORIENTATION.to_be_bytes());
	tiff.extend(3u16.to_be_bytes());
	tiff.extend(1u32.to_be_bytes());
	tiff.extend(o.to_be_bytes());
	tiff.extend([0, 0]);
	tiff.extend(0u32.to_be_bytes());
	let mut seg = vec![0xff, 0xe1];
	seg.extend(((tiff.len() + 6 + 2) as u16).to_be_bytes());
	seg.extend(b"Exif\0\0");
	seg.extend(tiff);
	seg
}

/// Puts an EXIF block into a freshly written JPG or PNG, so a converted photo keeps its date and camera.
pub fn insert_exif(file: &[u8], exif: &[u8]) -> Option<Vec<u8>> {
	if file.starts_with(&[0xff, 0xd8]) {
		if exif.len() + 8 > u16::MAX as usize {
			return None;
		}
		let mut out = vec![0xff, 0xd8];
		let segs = jpeg_segments(file);
		// after JFIF if present, as the standard asks
		let after = segs
			.first()
			.filter(|s| s.marker == 0xe0)
			.map(|s| s.whole.end)
			.unwrap_or(2);
		out.extend_from_slice(&file[2..after]);
		out.extend([0xff, 0xe1]);
		out.extend(((exif.len() + 8) as u16).to_be_bytes());
		out.extend(b"Exif\0\0");
		out.extend_from_slice(exif);
		out.extend_from_slice(&file[after..]);
		Some(out)
	} else if file.starts_with(b"\x89PNG\r\n\x1a\n") {
		let chunks = png_chunks(file);
		let idat = chunks.iter().find(|c| &c.kind == b"IDAT")?.whole.start;
		let mut out = file[..idat].to_vec();
		out.extend((exif.len() as u32).to_be_bytes());
		let mut body = b"eXIf".to_vec();
		body.extend_from_slice(exif);
		out.extend_from_slice(&body);
		out.extend(crc32(&body).to_be_bytes());
		out.extend_from_slice(&file[idat..]);
		Some(out)
	} else {
		None
	}
}

/// Every EXIF field a photo carries, as (section, name, value) for the Metadata window, with
/// plain names, the location first and the technical leftovers last.
pub fn fields(file: &[u8]) -> Vec<(String, String, String)> {
	use exif::Tag as T;
	let Some(r) = exif_range(file) else { return Vec::new() };
	let Ok(exif) = exif::Reader::new().read_raw(file[r].to_vec()) else {
		return Vec::new();
	};
	let coords = coordinates(&exif);
	let mut out: Vec<(u8, String, String, String)> = exif
		.fields()
		.filter(|f| f.ifd_num == exif::In::PRIMARY)
		.filter(|f| !matches!(f.tag, T::ExifIFDPointer | T::GPSInfoIFDPointer | T::InteropIFDPointer))
		// the coordinates line says these better
		.filter(|f| {
			coords.is_none()
				|| !matches!(
					f.tag,
					T::GPSLatitude | T::GPSLatitudeRef | T::GPSLongitude | T::GPSLongitudeRef
				)
		})
		.map(|f| {
			let (rank, section) = section_of(f.tag);
			let value = match &f.value {
				exif::Value::Ascii(parts) => parts
					.iter()
					.map(|p| String::from_utf8_lossy(p).trim_end_matches('\0').trim().to_string())
					.collect::<Vec<_>>()
					.join(", "),
				_ => f.display_value().with_unit(&exif).to_string(),
			};
			(
				rank,
				section.to_string(),
				plain_name(f.tag),
				value.chars().take(200).collect::<String>(),
			)
		})
		.filter(|(_, _, _, v)| !v.trim().is_empty())
		.collect();
	if let Some((lat, lon)) = coords {
		out.push((
			0,
			"Location".into(),
			"Coordinates".into(),
			format!("{lat:.5}, {lon:.5}"),
		));
	}
	// stable, so the photo's own order stays within a section; coordinates lead their section
	out.sort_by_key(|(rank, _, name, _)| (*rank, name != "Coordinates"));
	out.into_iter().map(|(_, s, n, v)| (s, n, v)).collect()
}

fn section_of(tag: exif::Tag) -> (u8, &'static str) {
	use exif::Tag as T;
	match tag {
		_ if tag.context() == exif::Context::Gps => (0, "Location"),
		T::Make
		| T::Model
		| T::LensMake
		| T::LensModel
		| T::LensSpecification
		| T::BodySerialNumber
		| T::CameraOwnerName => (1, "Camera"),
		T::ExposureTime
		| T::FNumber
		| T::PhotographicSensitivity
		| T::FocalLength
		| T::FocalLengthIn35mmFilm
		| T::Flash
		| T::WhiteBalance
		| T::ExposureBiasValue
		| T::MeteringMode
		| T::ExposureProgram
		| T::ExposureMode
		| T::DigitalZoomRatio
		| T::SceneCaptureType => (2, "Shot"),
		T::DateTimeOriginal
		| T::DateTimeDigitized
		| T::DateTime
		| T::OffsetTime
		| T::OffsetTimeOriginal
		| T::OffsetTimeDigitized
		| T::ImageDescription
		| T::Artist
		| T::Copyright
		| T::Software
		| T::Orientation
		| T::PixelXDimension
		| T::PixelYDimension
		| T::ImageWidth
		| T::ImageLength => (3, "Picture"),
		_ => (4, "Technical"),
	}
}

fn plain_name(tag: exif::Tag) -> String {
	use exif::Tag as T;
	let known = match tag {
		T::Make => "Maker",
		T::Model => "Model",
		T::LensMake => "Lens maker",
		T::LensModel => "Lens",
		T::LensSpecification => "Lens range",
		T::BodySerialNumber => "Serial number",
		T::CameraOwnerName => "Owner",
		T::ExposureTime => "Shutter",
		T::FNumber => "Aperture",
		T::PhotographicSensitivity => "ISO",
		T::FocalLength => "Focal length",
		T::FocalLengthIn35mmFilm => "35 mm equivalent",
		T::ExposureBiasValue => "Exposure compensation",
		T::MeteringMode => "Metering",
		T::ExposureProgram => "Program",
		T::DateTimeOriginal => "Taken",
		T::DateTimeDigitized => "Digitised",
		T::DateTime => "Changed",
		T::OffsetTimeOriginal => "Time zone",
		T::ImageDescription => "Description",
		T::Artist => "Author",
		T::PixelXDimension | T::ImageWidth => "Width",
		T::PixelYDimension | T::ImageLength => "Height",
		T::GPSAltitude => "Altitude",
		T::GPSImgDirection => "Facing",
		T::GPSSpeed => "Speed",
		T::GPSDateStamp => "Date",
		T::GPSTimeStamp => "Time",
		_ => "",
	};
	if !known.is_empty() {
		return known.into();
	}
	// "FocalPlaneXResolution" reads as "Focal plane x resolution"
	let raw = tag.to_string();
	let raw = raw.strip_prefix("GPS").unwrap_or(&raw);
	let mut out = String::new();
	for (i, c) in raw.chars().enumerate() {
		if c.is_uppercase() && i > 0 {
			out.push(' ');
		}
		out.push(if i == 0 { c } else { c.to_ascii_lowercase() });
	}
	out
}

fn coordinates(exif: &exif::Exif) -> Option<(f64, f64)> {
	let deg = |tag, sign_tag, neg: &str| -> Option<f64> {
		let f = exif.get_field(tag, exif::In::PRIMARY)?;
		let exif::Value::Rational(v) = &f.value else {
			return None;
		};
		let d = v.first()?.to_f64() + v.get(1)?.to_f64() / 60.0 + v.get(2)?.to_f64() / 3600.0;
		let s = exif
			.get_field(sign_tag, exif::In::PRIMARY)
			.map(|f| f.display_value().to_string())
			.unwrap_or_default();
		Some(if s.contains(neg) { -d } else { d })
	};
	Some((
		deg(exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef, "S")?,
		deg(exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef, "W")?,
	))
}

fn crc32(data: &[u8]) -> u32 {
	let mut c = 0xffff_ffffu32;
	for &b in data {
		c ^= b as u32;
		for _ in 0..8 {
			c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
		}
	}
	!c
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;

	/// A tiny JPEG-shaped file with EXIF holding an orientation and a GPS latitude.
	pub fn sample_jpeg(orient: u16) -> Vec<u8> {
		// TIFF: IFD0 at 8 with 2 entries (orientation, GPS pointer), GPS IFD with 1 entry (latitude ref "N")
		let mut t = b"II*\0\x08\0\0\0".to_vec();
		t.extend(2u16.to_le_bytes());
		t.extend(ORIENTATION.to_le_bytes());
		t.extend(3u16.to_le_bytes());
		t.extend(1u32.to_le_bytes());
		t.extend(orient.to_le_bytes());
		t.extend([0, 0]);
		t.extend(GPS_IFD.to_le_bytes());
		t.extend(4u16.to_le_bytes());
		t.extend(1u32.to_le_bytes());
		let gps_at = (t.len() + 4 + 4) as u32;
		t.extend(gps_at.to_le_bytes());
		t.extend(0u32.to_le_bytes());
		t.extend(1u16.to_le_bytes());
		t.extend(1u16.to_le_bytes());
		t.extend(2u16.to_le_bytes());
		t.extend(2u32.to_le_bytes());
		t.extend(b"N\0\0\0");
		t.extend(0u32.to_le_bytes());
		let mut j = vec![0xff, 0xd8, 0xff, 0xe1];
		j.extend(((t.len() + 8) as u16).to_be_bytes());
		j.extend(b"Exif\0\0");
		j.extend(&t);
		j.extend([0xff, 0xfe, 0, 6, b'h', b'i', b'!', b'!']);
		j.extend([0xff, 0xda, 0, 2, 1, 2, 3, 0xff, 0xd9]);
		j
	}

	#[test]
	fn fields_read_like_a_person_wrote_them() {
		use exif::experimental::Writer;
		use exif::{Field, In, Rational, Tag, Value};
		let ascii = |s: &str| Value::Ascii(vec![s.as_bytes().to_vec()]);
		let deg = |d: u32| {
			Value::Rational(vec![
				Rational::from((d, 1)),
				Rational::from((30, 1)),
				Rational::from((0, 1)),
			])
		};
		let tags = [
			Field {
				tag: Tag::Model,
				ifd_num: In::PRIMARY,
				value: ascii("Pixel 9"),
			},
			Field {
				tag: Tag::PhotographicSensitivity,
				ifd_num: In::PRIMARY,
				value: Value::Short(vec![200]),
			},
			Field {
				tag: Tag::GPSLatitudeRef,
				ifd_num: In::PRIMARY,
				value: ascii("S"),
			},
			Field {
				tag: Tag::GPSLatitude,
				ifd_num: In::PRIMARY,
				value: deg(33),
			},
			Field {
				tag: Tag::GPSLongitudeRef,
				ifd_num: In::PRIMARY,
				value: ascii("E"),
			},
			Field {
				tag: Tag::GPSLongitude,
				ifd_num: In::PRIMARY,
				value: deg(151),
			},
		];
		let mut w = Writer::new();
		for f in &tags {
			w.push_field(f);
		}
		let mut tiff = std::io::Cursor::new(Vec::new());
		w.write(&mut tiff, false).unwrap();
		let jpg = insert_exif(&[0xff, 0xd8, 0xff, 0xd9], &tiff.into_inner()).unwrap();
		let got = fields(&jpg);
		assert_eq!(
			got[0],
			("Location".into(), "Coordinates".into(), "-33.50000, 151.50000".into())
		);
		assert!(got.contains(&("Camera".into(), "Model".into(), "Pixel 9".into())));
		assert!(got.contains(&("Shot".into(), "ISO".into(), "200".into())));
		assert!(!got.iter().any(|(_, n, _)| n.contains("atitude")), "{got:?}");
	}

	#[test]
	fn reads_orientation_and_location() {
		let j = sample_jpeg(6);
		assert_eq!(orientation(&j), 6);
		assert_eq!(upright_filter(6), "transpose=clock");
		assert!(has_location(&j));
	}

	#[test]
	fn location_goes_and_the_rest_stays() {
		let j = sample_jpeg(6);
		let out = remove_location(&j).unwrap();
		assert_eq!(out.len(), j.len());
		assert!(!has_location(&out));
		assert_eq!(orientation(&out), 6);
		assert!(out.ends_with(&[0xff, 0xda, 0, 2, 1, 2, 3, 0xff, 0xd9]));
	}

	#[test]
	fn remove_all_keeps_pictures_upright() {
		let j = sample_jpeg(8);
		let out = remove_all(&j).unwrap();
		assert!(!has_location(&out));
		assert_eq!(orientation(&out), 8);
		assert!(!out.windows(4).any(|w| w == b"hi!!"));
		let upright = remove_all(&sample_jpeg(1)).unwrap();
		assert!(exif_range(&upright).is_none());
	}

	#[test]
	fn exif_moves_into_new_files() {
		let src = sample_jpeg(6);
		let exif = tidy_exif(&src[exif_range(&src).unwrap()], false);
		let plain = remove_all(&sample_jpeg(1)).unwrap();
		let out = insert_exif(&plain, &exif).unwrap();
		assert_eq!(orientation(&out), 1);
		assert!(has_location(&out));
		assert_eq!(crc32(b"IEND"), 0xae42_6082);
	}
}
