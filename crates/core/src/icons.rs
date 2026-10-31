//! Icons: Windows ICO and macOS ICNS with every size, PNG sets, favicon packs, and pulling
//! the pictures back out of icon files and programs.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::Error;
use crate::ff::{Run, out, strs};
use crate::image::{decode_png, render_svg_bytes, source, write_png};
use crate::kind::ext_of;
use crate::plan::stem_of;
use crate::run::Engines;

pub const WINDOWS_SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 96, 128, 256];
const MAC_SIZES: &[u32] = &[16, 32, 64, 128, 256, 512, 1024];

/// One square RGBA picture of the icon at `n` pixels, as PNG bytes.
fn square_png(e: &Engines, input: &Path, n: u32) -> Result<Vec<u8>, Error> {
	let ext = ext_of(input);
	if matches!(ext.as_str(), "svg" | "svgz") {
		let data = fs::read(input)?;
		let data = if ext == "svgz" {
			let mut out = Vec::new();
			std::io::Read::read_to_end(&mut flate2::read::GzDecoder::new(&data[..]), &mut out)?;
			out
		} else {
			data
		};
		let (w, h, rgba) = render_svg_bytes(&data, n, Some(n))?;
		return encode_png(w, h, &rgba);
	}
	// icon files: start from their biggest picture
	let big;
	let from = if matches!(ext.as_str(), "ico" | "cur" | "icns") {
		let pics = read_icon_file(input)?;
		let (w, h, rgba) = pics
			.into_iter()
			.max_by_key(|p| p.0)
			.ok_or_else(|| Error::Engine("this icon file is empty".into()))?;
		big = e.scratch_path("icon", "png");
		fs::create_dir_all(&e.scratch)?;
		write_png(&big, w, h, &rgba)?;
		big.clone()
	} else {
		big = PathBuf::new();
		input.to_path_buf()
	};
	let src = source(e, &from)?;
	let tmp = e.scratch_path("square", "png");
	fs::create_dir_all(&e.scratch)?;
	let vf = format!(
		"{}scale={n}:{n}:force_original_aspect_ratio=decrease:flags=lanczos,format=rgba,pad={n}:{n}:(ow-iw)/2:(oh-ih)/2:color=0x00000000",
		if src.upright.is_empty() {
			String::new()
		} else {
			format!("{},", src.upright)
		}
	);
	let mut run = Run::new(&e.ffmpeg);
	if !src.upright.is_empty() {
		run = run.arg("-noautorotate");
	}
	let result = run
		.input(&src.path)
		.args(strs(&[
			"-vf",
			&vf,
			"-frames:v",
			"1",
			"-c:v",
			"png",
			"-f",
			"image2",
			"-update",
			"1",
		]))
		.args(out(&tmp))
		.go(&mut |_| {}, &AtomicBool::new(false));
	if !big.as_os_str().is_empty() {
		let _ = fs::remove_file(&big);
	}
	result?;
	let bytes = fs::read(&tmp)?;
	let _ = fs::remove_file(&tmp);
	Ok(bytes)
}

fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Result<Vec<u8>, Error> {
	let mut out = Vec::new();
	{
		let mut enc = png::Encoder::new(&mut out, w, h);
		enc.set_color(png::ColorType::Rgba);
		enc.set_depth(png::BitDepth::Eight);
		enc.set_compression(png::Compression::High);
		let mut wr = enc.write_header().map_err(|e| Error::Engine(e.to_string()))?;
		wr.write_image_data(rgba).map_err(|e| Error::Engine(e.to_string()))?;
	}
	Ok(out)
}

/// An ICO file holding these PNGs.
pub fn ico_bytes(pngs: &[(u32, Vec<u8>)]) -> Vec<u8> {
	let mut out = Vec::new();
	out.extend(0u16.to_le_bytes());
	out.extend(1u16.to_le_bytes());
	out.extend((pngs.len() as u16).to_le_bytes());
	let mut offset = 6 + 16 * pngs.len();
	for (n, data) in pngs {
		let side = if *n >= 256 { 0 } else { *n as u8 };
		out.extend([side, side, 0, 0]);
		out.extend(1u16.to_le_bytes());
		out.extend(32u16.to_le_bytes());
		out.extend((data.len() as u32).to_le_bytes());
		out.extend((offset as u32).to_le_bytes());
		offset += data.len();
	}
	for (_, data) in pngs {
		out.extend(data);
	}
	out
}

pub fn windows_icon(e: &Engines, inputs: &[PathBuf], part: &Path) -> Result<(), Error> {
	let pngs = WINDOWS_SIZES
		.iter()
		.map(|&n| square_png(e, &inputs[0], n).map(|b| (n, b)))
		.collect::<Result<Vec<_>, _>>()?;
	fs::write(part, ico_bytes(&pngs))?;
	Ok(())
}

pub fn mac_icon(e: &Engines, inputs: &[PathBuf], part: &Path) -> Result<(), Error> {
	use icns::{IconFamily, IconType, Image, PixelFormat};
	let mut family = IconFamily::new();
	for &n in MAC_SIZES {
		let bytes = square_png(e, &inputs[0], n)?;
		let img = Image::read_png(&bytes[..]).map_err(|e| Error::Engine(format!("ICNS: {e}")))?;
		let kind = match n {
			16 => IconType::RGBA32_16x16,
			32 => IconType::RGBA32_32x32,
			64 => IconType::RGBA32_32x32_2x,
			128 => IconType::RGBA32_128x128,
			256 => IconType::RGBA32_256x256,
			512 => IconType::RGBA32_512x512,
			_ => IconType::RGBA32_512x512_2x,
		};
		let img = if img.pixel_format() == PixelFormat::RGBA {
			img
		} else {
			img.convert_to(PixelFormat::RGBA)
		};
		family
			.add_icon_with_type(&img, kind)
			.map_err(|e| Error::Engine(format!("ICNS: {e}")))?;
	}
	let mut f = std::io::BufWriter::new(fs::File::create(part)?);
	family.write(&mut f).map_err(|e| Error::Engine(format!("ICNS: {e}")))?;
	Ok(())
}

pub fn icon_set(e: &Engines, input: &Path, sizes: &[u32], part: &Path) -> Result<(), Error> {
	let stem = stem_of(input);
	if let [one] = sizes {
		fs::write(part, square_png(e, input, *one)?)?;
		return Ok(());
	}
	fs::create_dir_all(part)?;
	for &n in sizes {
		fs::write(part.join(format!("{stem}-{n}.png")), square_png(e, input, n)?)?;
	}
	Ok(())
}

pub fn favicon(e: &Engines, input: &Path, part: &Path) -> Result<(), Error> {
	fs::create_dir_all(part)?;
	let small = [16u32, 32, 48]
		.iter()
		.map(|&n| square_png(e, input, n).map(|b| (n, b)))
		.collect::<Result<Vec<_>, _>>()?;
	fs::write(part.join("favicon.ico"), ico_bytes(&small))?;
	fs::write(part.join("favicon-16x16.png"), &small[0].1)?;
	fs::write(part.join("favicon-32x32.png"), &small[1].1)?;
	fs::write(part.join("apple-touch-icon.png"), square_png(e, input, 180)?)?;
	fs::write(part.join("android-chrome-192x192.png"), square_png(e, input, 192)?)?;
	fs::write(part.join("android-chrome-512x512.png"), square_png(e, input, 512)?)?;
	if matches!(ext_of(input).as_str(), "svg" | "svgz") {
		fs::copy(input, part.join("favicon.svg"))?;
	}
	let manifest = r##"{
  "name": "",
  "short_name": "",
  "icons": [
    { "src": "/android-chrome-192x192.png", "sizes": "192x192", "type": "image/png" },
    { "src": "/android-chrome-512x512.png", "sizes": "512x512", "type": "image/png" }
  ],
  "theme_color": "#ffffff",
  "background_color": "#ffffff",
  "display": "standalone"
}
"##;
	fs::write(part.join("site.webmanifest"), manifest)?;
	let mut html = String::from(
		"<link rel=\"icon\" href=\"/favicon.ico\" sizes=\"48x48\">\n<link rel=\"icon\" type=\"image/png\" sizes=\"32x32\" href=\"/favicon-32x32.png\">\n<link rel=\"icon\" type=\"image/png\" sizes=\"16x16\" href=\"/favicon-16x16.png\">\n<link rel=\"apple-touch-icon\" sizes=\"180x180\" href=\"/apple-touch-icon.png\">\n<link rel=\"manifest\" href=\"/site.webmanifest\">\n",
	);
	if part.join("favicon.svg").exists() {
		html.insert_str(0, "<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">\n");
	}
	fs::write(part.join("paste-into-head.html"), html)?;
	Ok(())
}

/// Every picture inside an ICO, CUR or ICNS file, as RGBA.
pub fn read_icon_file(input: &Path) -> Result<Vec<(u32, u32, Vec<u8>)>, Error> {
	let bytes = fs::read(input)?;
	if ext_of(input) == "icns" {
		let family =
			icns::IconFamily::read(std::io::Cursor::new(&bytes)).map_err(|e| Error::Engine(format!("ICNS: {e}")))?;
		let mut out = Vec::new();
		for kind in family.available_icons() {
			if let Ok(img) = family.get_icon_with_type(kind) {
				let img = img.convert_to(icns::PixelFormat::RGBA);
				out.push((img.width(), img.height(), img.data().to_vec()));
			}
		}
		return Ok(out);
	}
	read_ico(&bytes)
}

fn read_ico(b: &[u8]) -> Result<Vec<(u32, u32, Vec<u8>)>, Error> {
	let bad = || Error::Engine("this icon file is damaged".into());
	let n = u16::from_le_bytes([*b.get(4).ok_or_else(bad)?, *b.get(5).ok_or_else(bad)?]) as usize;
	let mut out = Vec::new();
	for i in 0..n {
		let d = 6 + i * 16;
		let Some(entry) = b.get(d..d + 16) else { break };
		let size = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;
		let at = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;
		let Some(data) = b.get(at..at + size) else { continue };
		if data.starts_with(b"\x89PNG") {
			if let Ok(px) = decode_png(std::io::Cursor::new(data)) {
				out.push(px);
			}
		} else if let Some(px) = read_dib(data) {
			out.push(px);
		}
	}
	// largest first, one per size
	out.sort_by_key(|p| std::cmp::Reverse(p.0));
	out.dedup_by_key(|p| p.0);
	Ok(out)
}

/// A 24- or 32-bit bitmap from inside an ICO, with its transparency mask.
fn read_dib(d: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
	let w = i32::from_le_bytes(d.get(4..8)?.try_into().ok()?) as u32;
	let h = (i32::from_le_bytes(d.get(8..12)?.try_into().ok()?) / 2) as u32;
	let bpp = u16::from_le_bytes(d.get(14..16)?.try_into().ok()?);
	let header = u32::from_le_bytes(d.get(0..4)?.try_into().ok()?) as usize;
	if !(bpp == 32 || bpp == 24) || w == 0 || h == 0 || w > 1024 || h > 1024 {
		return None;
	}
	let stride = ((w * bpp as u32 / 8) as usize).div_ceil(4) * 4;
	let mask_stride = (w as usize).div_ceil(32) * 4;
	let pixels = d.get(header..header + stride * h as usize)?;
	let mask = d.get(header + stride * h as usize..);
	let mut rgba = vec![0u8; (w * h * 4) as usize];
	for y in 0..h as usize {
		let row = &pixels[(h as usize - 1 - y) * stride..];
		for x in 0..w as usize {
			let px = &row[x * bpp as usize / 8..];
			let o = (y * w as usize + x) * 4;
			let mut a = if bpp == 32 { px[3] } else { 255 };
			if bpp == 24
				&& let Some(m) = mask
				&& let Some(byte) = m.get((h as usize - 1 - y) * mask_stride + x / 8)
				&& byte & (0x80 >> (x % 8)) != 0
			{
				a = 0;
			}
			rgba[o..o + 4].copy_from_slice(&[px[2], px[1], px[0], a]);
		}
	}
	Some((w, h, rgba))
}

/// Saves every size inside an icon file, or a program's icon, as PNGs in a folder.
pub fn extract_icons(e: &Engines, input: &Path, part: &Path) -> Result<(), Error> {
	let stem = stem_of(input);
	let pics = match ext_of(input).as_str() {
		"ico" | "cur" | "icns" => read_icon_file(input)?,
		_ => program_icons(input)?,
	};
	if pics.is_empty() {
		return Err(Error::Engine("there's no icon inside".into()));
	}
	fs::create_dir_all(part)?;
	for (w, h, rgba) in &pics {
		write_png(&part.join(format!("{stem}-{w}.png")), *w, *h, rgba)?;
	}
	// programs also get a ready-to-use ICO
	if !matches!(ext_of(input).as_str(), "ico" | "cur" | "icns") {
		let best = part.join(format!("{stem}-{}.png", pics[0].0));
		windows_icon(e, &[best], &part.join(format!("{stem}.ico")))?;
	}
	Ok(())
}

#[cfg(windows)]
fn program_icons(input: &Path) -> Result<Vec<(u32, u32, Vec<u8>)>, Error> {
	crate::winicon::extract(input)
}

#[cfg(not(windows))]
fn program_icons(_input: &Path) -> Result<Vec<(u32, u32, Vec<u8>)>, Error> {
	Err(Error::Engine("program icons can only be read on Windows".into()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ico_round_trips() {
		let red: Vec<u8> = [255, 0, 0, 255].repeat(16 * 16);
		let png = encode_png(16, 16, &red).unwrap();
		let ico = ico_bytes(&[(16, png.clone()), (256, png)]);
		assert_eq!(&ico[0..6], &[0, 0, 1, 0, 2, 0]);
		assert_eq!(ico[6], 16);
		assert_eq!(ico[22], 0, "256 is written as 0");
		let back = read_ico(&ico).unwrap();
		assert_eq!(back.len(), 1, "same picture at both sizes dedups");
		assert_eq!(&back[0].2[..4], &[255, 0, 0, 255]);
	}

	#[test]
	fn reads_old_bitmap_icons() {
		// 2x2, 32-bit, bottom-up, with a mask
		let mut d = Vec::new();
		d.extend(40u32.to_le_bytes());
		d.extend(2i32.to_le_bytes());
		d.extend(4i32.to_le_bytes());
		d.extend(1u16.to_le_bytes());
		d.extend(32u16.to_le_bytes());
		d.extend([0u8; 24]);
		// bottom row then top row, BGRA
		d.extend([0, 0, 255, 255, 0, 255, 0, 255]);
		d.extend([255, 0, 0, 255, 0, 0, 0, 0]);
		d.extend([0u8; 8]);
		let (w, h, rgba) = read_dib(&d).unwrap();
		assert_eq!((w, h), (2, 2));
		assert_eq!(&rgba[0..4], &[0, 0, 255, 255], "top-left is blue");
		assert_eq!(&rgba[8..12], &[255, 0, 0, 255], "bottom-left is red");
	}
}
