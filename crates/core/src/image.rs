//! Picture recipes. FFmpeg does the decoding and encoding; formats it can't read are turned into
//! a PNG first, and metadata is carried across by hand so converted photos keep their details.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::Error;
use crate::ff::{self, Run, image_args, join_vf, out, strs, turn_filter};
use crate::format::Format;
use crate::kind::{ext_of, is_raw};
use crate::metadata;
use crate::plan::{image_format_for, same_image};
use crate::recipe::{Layout, Recipe};
use crate::run::Engines;
use crate::settings::Defaults;

type Progress<'a> = &'a mut dyn FnMut(f32);

/// A picture FFmpeg can read, made from the input if needed.
pub struct Source {
	pub path: PathBuf,
	temp: bool,
	/// Filter that stands the picture upright, from its EXIF.
	pub upright: &'static str,
	/// The original's EXIF, to carry into the result.
	pub exif: Option<Vec<u8>>,
}

impl Drop for Source {
	fn drop(&mut self) {
		if self.temp {
			let _ = fs::remove_file(&self.path);
		}
	}
}

pub fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<(), Error> {
	let file = std::io::BufWriter::new(fs::File::create(path)?);
	let mut enc = png::Encoder::new(file, w, h);
	enc.set_color(png::ColorType::Rgba);
	enc.set_depth(png::BitDepth::Eight);
	enc.set_compression(png::Compression::Fast);
	let mut w = enc.write_header().map_err(|e| Error::Engine(e.to_string()))?;
	w.write_image_data(rgba).map_err(|e| Error::Engine(e.to_string()))?;
	w.finish().map_err(|e| Error::Engine(e.to_string()))?;
	Ok(())
}

pub fn read_png(path: &Path) -> Result<(u32, u32, Vec<u8>), Error> {
	let mut d = png::Decoder::new(std::io::BufReader::new(fs::File::open(path)?));
	d.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
	let mut r = d.read_info().map_err(|e| Error::Engine(e.to_string()))?;
	let mut buf = vec![
		0;
		r.output_buffer_size()
			.ok_or_else(|| Error::Engine("picture too large".into()))?
	];
	let info = r.next_frame(&mut buf).map_err(|e| Error::Engine(e.to_string()))?;
	let (w, h) = (info.width, info.height);
	let rgba = match info.color_type {
		png::ColorType::Rgba => buf[..(w * h * 4) as usize].to_vec(),
		png::ColorType::GrayscaleAlpha => buf[..(w * h * 2) as usize]
			.as_chunks::<2>()
			.0
			.iter()
			.flat_map(|p| [p[0], p[0], p[0], p[1]])
			.collect(),
		other => return Err(Error::Engine(format!("unexpected PNG layout {other:?}"))),
	};
	Ok((w, h, rgba))
}

fn temp_png(e: &Engines, tag: &str) -> Result<PathBuf, Error> {
	fs::create_dir_all(&e.scratch)?;
	static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
	let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	Ok(e.scratch.join(format!("{tag}-{}-{n}.png", std::process::id())))
}

/// Renders an SVG with resvg, making its longest side at least `min_edge`.
pub fn render_svg(path: &Path, min_edge: u32) -> Result<(u32, u32, Vec<u8>), Error> {
	let data = fs::read(path)?;
	let data = if ext_of(path) == "svgz" {
		let mut out = Vec::new();
		std::io::Read::read_to_end(&mut flate2::read::GzDecoder::new(&data[..]), &mut out)?;
		out
	} else {
		data
	};
	render_svg_bytes(&data, min_edge, None)
}

/// Renders SVG bytes at `exact` size (square box) or at least `min_edge` on the longest side.
pub fn render_svg_bytes(data: &[u8], min_edge: u32, exact: Option<u32>) -> Result<(u32, u32, Vec<u8>), Error> {
	let mut opt = resvg::usvg::Options::default();
	opt.fontdb_mut().load_system_fonts();
	let tree = resvg::usvg::Tree::from_data(data, &opt).map_err(|e| Error::Engine(format!("SVG: {e}")))?;
	let size = tree.size();
	let (sw, sh) = (size.width().max(1.0), size.height().max(1.0));
	let scale = match exact {
		Some(n) => n as f32 / sw.max(sh),
		None => (min_edge as f32 / sw.max(sh)).max(1.0),
	};
	let (w, h) = match exact {
		Some(n) => (n, n),
		None => ((sw * scale).round() as u32, (sh * scale).round() as u32),
	};
	let mut pm = tiny_skia::Pixmap::new(w.max(1), h.max(1)).ok_or_else(|| Error::Engine("SVG is too large".into()))?;
	let (dx, dy) = ((w as f32 - sw * scale) / 2.0, (h as f32 - sh * scale) / 2.0);
	let t = tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, dx, dy);
	resvg::render(&tree, t, &mut pm.as_mut());
	Ok((w, h, unpremultiply(pm.data())))
}

pub fn unpremultiply(data: &[u8]) -> Vec<u8> {
	data.as_chunks::<4>()
		.0
		.iter()
		.flat_map(|p| {
			let a = p[3] as u32;
			if a == 0 || a == 255 {
				[p[0], p[1], p[2], p[3]]
			} else {
				let f = |c: u8| ((c as u32 * 255 + a / 2) / a).min(255) as u8;
				[f(p[0]), f(p[1]), f(p[2]), p[3]]
			}
		})
		.collect()
}

fn decode_special(e: &Engines, input: &Path, ext: &str) -> Result<Option<PathBuf>, Error> {
	if !matches!(ext, "svg" | "svgz" | "jxl" | "psd") && !is_raw(ext) {
		return Ok(None);
	}
	let what = match ext {
		"psd" => "This PSD can't be read".to_string(),
		e if is_raw(e) => "This camera file can't be read".to_string(),
		other => format!("This .{other} file can't be read"),
	};
	let (w, h, rgba) = crate::caught(&what, || decode_rgba_special(input, ext))?;
	let tmp = temp_png(e, "src")?;
	write_png(&tmp, w, h, &rgba)?;
	Ok(Some(tmp))
}

/// Pictures FFmpeg can't open, decoded by their own libraries.
fn decode_rgba_special(input: &Path, ext: &str) -> Result<(u32, u32, Vec<u8>), Error> {
	Ok(match ext {
		"svg" | "svgz" => render_svg(input, 1024)?,
		"jxl" => {
			let img = jxl_oxide::JxlImage::builder()
				.open(input)
				.map_err(|e| Error::Engine(format!("JPEG XL: {e}")))?;
			let render = img
				.render_frame(0)
				.map_err(|e| Error::Engine(format!("JPEG XL: {e}")))?;
			let fb = render.image_all_channels();
			let (w, h, ch) = (fb.width() as u32, fb.height() as u32, fb.channels());
			let px = fb.buf();
			let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
			let rgba: Vec<u8> = px
				.chunks_exact(ch)
				.flat_map(|p| match ch {
					1 => [to8(p[0]), to8(p[0]), to8(p[0]), 255],
					2 => [to8(p[0]), to8(p[0]), to8(p[0]), to8(p[1])],
					3 => [to8(p[0]), to8(p[1]), to8(p[2]), 255],
					_ => [to8(p[0]), to8(p[1]), to8(p[2]), to8(p[3])],
				})
				.collect();
			(w, h, rgba)
		}
		"psd" => {
			let bytes = fs::read(input)?;
			let psd = psd::Psd::from_bytes(&bytes).map_err(|e| Error::Engine(format!("PSD: {e}")))?;
			(psd.width(), psd.height(), psd.rgba())
		}
		ext if is_raw(ext) => {
			let mut pipe =
				imagepipe::Pipeline::new_from_file(input).map_err(|e| Error::Engine(format!("camera file: {e}")))?;
			let img = pipe
				.output_8bit(None)
				.map_err(|e| Error::Engine(format!("camera file: {e}")))?;
			let rgba = img
				.data
				.as_chunks::<3>()
				.0
				.iter()
				.flat_map(|p| [p[0], p[1], p[2], 255])
				.collect();
			(img.width as u32, img.height as u32, rgba)
		}
		other => return Err(Error::Engine(format!("can't decode .{other} here"))),
	})
}

/// Makes the input readable by FFmpeg and works out how to stand it upright.
/// Width and height read straight from a PNG, JPEG, GIF, WebP or BMP header, without starting
/// FFmpeg. None for anything else or anything odd, and the caller asks FFmpeg instead.
pub fn header_size(path: &Path) -> Option<(u32, u32)> {
	use std::io::{Read, Seek, SeekFrom};
	let mut f = fs::File::open(path).ok()?;
	let mut head = [0u8; 32];
	f.read_exact(&mut head).ok()?;
	let be16 = |b: &[u8]| u16::from_be_bytes([b[0], b[1]]) as u32;
	let le16 = |b: &[u8]| u16::from_le_bytes([b[0], b[1]]) as u32;
	let be32 = |b: &[u8]| u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
	let le32 = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
	let size = if head.starts_with(b"\x89PNG\r\n\x1a\n") && &head[12..16] == b"IHDR" {
		(be32(&head[16..20]), be32(&head[20..24]))
	} else if head.starts_with(b"GIF8") {
		(le16(&head[6..8]), le16(&head[8..10]))
	} else if head.starts_with(b"BM") {
		(le32(&head[18..22]), (le32(&head[22..26]) as i32).unsigned_abs())
	} else if head.starts_with(b"RIFF") && &head[8..12] == b"WEBP" {
		match &head[12..16] {
			b"VP8 " => {
				let mut b = [0u8; 30];
				f.seek(SeekFrom::Start(0)).ok()?;
				f.read_exact(&mut b).ok()?;
				(le16(&b[26..28]) & 0x3fff, le16(&b[28..30]) & 0x3fff)
			}
			b"VP8L" => {
				let b = &head[21..25];
				let bits = le32(b);
				((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1)
			}
			b"VP8X" => {
				let w = 1 + (head[24] as u32 | (head[25] as u32) << 8 | (head[26] as u32) << 16);
				let h = 1 + (head[27] as u32 | (head[28] as u32) << 8 | (head[29] as u32) << 16);
				(w, h)
			}
			_ => return None,
		}
	} else if head.starts_with(&[0xff, 0xd8]) {
		// walk the JPEG's segments to the frame header; EXIF and the like are skipped over
		let mut at = 2u64;
		loop {
			let mut m = [0u8; 4];
			f.seek(SeekFrom::Start(at)).ok()?;
			f.read_exact(&mut m).ok()?;
			if m[0] != 0xff {
				return None;
			}
			let marker = m[1];
			if marker == 0xd8 || marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
				at += 2;
				continue;
			}
			let len = be16(&m[2..4]) as u64;
			let sof = matches!(marker, 0xc0..=0xcf) && !matches!(marker, 0xc4 | 0xc8 | 0xcc);
			if sof {
				let mut b = [0u8; 5];
				f.read_exact(&mut b).ok()?;
				break (be16(&b[3..5]), be16(&b[1..3]));
			}
			if marker == 0xda || len < 2 {
				return None;
			}
			at += 2 + len;
		}
	} else {
		return None;
	};
	(size.0 > 0 && size.1 > 0).then_some(size)
}

pub fn source(e: &Engines, input: &Path) -> Result<Source, Error> {
	let ext = ext_of(input);
	if let Some(tmp) = decode_special(e, input, &ext)? {
		return Ok(Source {
			path: tmp,
			temp: true,
			upright: "",
			exif: None,
		});
	}
	let (upright, exif) = if matches!(
		ext.as_str(),
		"jpg" | "jpeg" | "jfif" | "jpe" | "png" | "webp" | "tif" | "tiff"
	) {
		let bytes = fs::read(input)?;
		let o = metadata::orientation(&bytes);
		let exif = metadata::exif_range(&bytes)
			.filter(|r| r.len() < 60_000 && !matches!(ext.as_str(), "tif" | "tiff"))
			.map(|r| bytes[r].to_vec());
		(metadata::upright_filter(o), exif)
	} else {
		("", None)
	};
	Ok(Source {
		path: input.to_path_buf(),
		temp: false,
		upright,
		exif,
	})
}

/// Size of the picture as it will be shown.
pub fn size_of(e: &Engines, src: &Source) -> Result<(u32, u32), Error> {
	let p = ff::probe(&e.ffprobe, &src.path)?;
	let (w, h) = p.shown_size();
	Ok(
		if matches!(
			src.upright,
			"transpose=clock" | "transpose=cclock" | "transpose=clock_flip" | "transpose=cclock_flip"
		) {
			(h, w)
		} else {
			(w, h)
		},
	)
}

/// Writes one picture: upright, `filter`, then encoded. Keeps the original's details unless `strip`.
#[allow(clippy::too_many_arguments)]
pub fn write(
	e: &Engines,
	src: &Source,
	filter: &str,
	to: Format,
	quality: u8,
	strip: bool,
	part: &Path,
	c: &AtomicBool,
) -> Result<(), Error> {
	let mut run = Run::new(&e.ffmpeg);
	if !src.upright.is_empty() {
		run = run.arg("-noautorotate");
	}
	let vf = join_vf(&[src.upright, filter]);
	run.input(&src.path)
		.args(image_args(to, &vf, quality, true))
		.args(out(part))
		.go(&mut |_| {}, c)?;
	if !strip
		&& let Some(exif) = &src.exif
		&& matches!(to, Format::Jpg | Format::Png)
	{
		let bytes = fs::read(part)?;
		if let Some(with) = metadata::insert_exif(&bytes, &metadata::tidy_exif(exif, false)) {
			fs::write(part, with)?;
		}
	}
	Ok(())
}

/// The format a recipe writes for this input when it keeps the input's own format.
fn keep(input: &Path) -> Format {
	same_image(&ext_of(input))
}

pub fn run(
	e: &Engines,
	d: &Defaults,
	r: &Recipe,
	input: &Path,
	part: &Path,
	_pr: Progress,
	c: &AtomicBool,
) -> Result<(), Error> {
	let src = source(e, input)?;
	let strip = d.strip_on_convert;
	match r {
		Recipe::Convert { to: Format::Ico } => crate::icons::windows_icon(e, &[input.to_path_buf()], part),
		Recipe::Convert { to: Format::Icns } => crate::icons::mac_icon(e, &[input.to_path_buf()], part),
		Recipe::Convert { to: Format::Svg } => trace(e, &src, part),
		Recipe::Convert {
			to: to @ (Format::Mp4 | Format::Webm),
		} => animated_to_video(e, input, *to, part, c),
		Recipe::Convert { to: Format::Webp } if ext_of(input) == "gif" => animated_webp(e, input, d, part, c),
		Recipe::Convert { to } => write(e, &src, "", *to, d.image_quality, strip, part, c),
		Recipe::CompressImage {
			quality,
			max_edge,
			target_kb,
		} => compress(e, &src, input, *quality, *max_edge, *target_kb, part, c),
		Recipe::ResizeImage { width, height, percent } => {
			let vf = match (percent, width, height) {
				(Some(p), _, _) => format!("scale=trunc(iw*{p}/100):trunc(ih*{p}/100):flags=lanczos"),
				(None, Some(w), Some(h)) => {
					format!("scale='min({w},iw)':'min({h},ih)':force_original_aspect_ratio=decrease:flags=lanczos")
				}
				(None, Some(w), None) => format!("scale='min({w},iw)':-1:flags=lanczos"),
				(None, None, Some(h)) => format!("scale=-1:'min({h},ih)':flags=lanczos"),
				_ => String::new(),
			};
			write(e, &src, &vf, keep(input), d.image_quality, strip, part, c)
		}
		Recipe::RotateImage { turn } => write(
			e,
			&src,
			turn_filter(*turn),
			keep(input),
			d.image_quality,
			strip,
			part,
			c,
		),
		Recipe::CropImage { x, y, w, h } => {
			let vf = format!("crop={w}:{h}:{x}:{y}");
			write(e, &src, &vf, keep(input), d.image_quality, strip, part, c)
		}
		Recipe::StripMetadata { location_only } => {
			let bytes = fs::read(input)?;
			let lossless = matches!(ext_of(input).as_str(), "jpg" | "jpeg" | "jfif" | "jpe" | "png" | "webp");
			if !lossless {
				// other formats keep details in places we can't edit in place, so they're rewritten clean
				return write(e, &src, "", keep(input), d.image_quality_best.max(92), true, part, c);
			}
			let clean = if *location_only {
				metadata::remove_location(&bytes).unwrap_or_else(|| bytes.clone())
			} else {
				metadata::remove_all(&bytes).unwrap_or_else(|| bytes.clone())
			};
			Ok(fs::write(part, clean)?)
		}
		Recipe::RemoveBackground => crate::bg::remove(e, &src, part, c),
		Recipe::Trace => trace(e, &src, part),
		Recipe::Rendered { file, to, .. } => {
			let to = to.unwrap_or_else(|| keep(input));
			if to == Format::Png {
				fs::copy(file, part)?;
				Ok(())
			} else {
				let rendered = Source {
					path: file.clone(),
					temp: false,
					upright: "",
					exif: src.exif.clone(),
				};
				write(e, &rendered, "", to, d.image_quality, strip, part, c)
			}
		}
		_ => Err(Error::Engine("this tool doesn't fit this picture".into())),
	}
}

#[allow(clippy::too_many_arguments)]
fn compress(
	e: &Engines,
	src: &Source,
	input: &Path,
	quality: u8,
	max_edge: Option<u32>,
	target_kb: Option<u32>,
	part: &Path,
	c: &AtomicBool,
) -> Result<(), Error> {
	let to = match keep(input) {
		Format::Tiff | Format::Bmp | Format::Tga | Format::Qoi => Format::Jpg,
		f => f,
	};
	let fit = |edge: u32| {
		format!("scale='min({edge},iw)':'min({edge},ih)':force_original_aspect_ratio=decrease:flags=lanczos")
	};
	let mut edge = max_edge;
	if to == Format::Png {
		// palette compression keeps PNG sharp and transparent
		let mut q = quality;
		loop {
			let flat = temp_png(e, "pq")?;
			write(
				e,
				src,
				&edge.map(fit).unwrap_or_default(),
				Format::Png,
				100,
				true,
				&flat,
				c,
			)?;
			let result = crate::pngquant::compress(&flat, part, q);
			let _ = fs::remove_file(&flat);
			result?;
			let kb = fs::metadata(part)?.len() / 1024;
			match target_kb {
				Some(t) if kb > t as u64 && q > 20 => q = q.saturating_sub(20),
				Some(t) if kb > t as u64 => {
					let (w, h) = size_of(e, src)?;
					let now = edge.unwrap_or(w.max(h));
					if now < 64 {
						return Ok(());
					}
					edge = Some((now as f32 * 0.8) as u32);
				}
				_ => return Ok(()),
			}
		}
	}
	let Some(target) = target_kb else {
		return write(e, src, &edge.map(fit).unwrap_or_default(), to, quality, false, part, c);
	};
	// find the best quality that fits the size, shrinking the picture if even low quality is too big
	let target = target as u64 * 1024;
	for _ in 0..6 {
		let (mut lo, mut hi, mut best) = (25u8, 95u8, None);
		while lo <= hi {
			let q = lo + (hi - lo) / 2;
			write(e, src, &edge.map(fit).unwrap_or_default(), to, q, false, part, c)?;
			if fs::metadata(part)?.len() <= target {
				best = Some(q);
				lo = q + 1;
			} else {
				hi = q.saturating_sub(1);
				if q == 25 {
					break;
				}
			}
		}
		if let Some(q) = best {
			return write(e, src, &edge.map(fit).unwrap_or_default(), to, q, false, part, c);
		}
		let (w, h) = size_of(e, src)?;
		let now = edge.unwrap_or(w.max(h));
		edge = Some(((now as f32) * 0.75) as u32);
		if edge.unwrap_or(0) < 64 {
			break;
		}
	}
	Err(Error::Engine("couldn't get it that small".into()))
}

fn trace(e: &Engines, src: &Source, part: &Path) -> Result<(), Error> {
	let (w, h) = size_of(e, src)?;
	let tmp = temp_png(e, "trace")?;
	let flat = Source {
		path: src.path.clone(),
		temp: false,
		upright: src.upright,
		exif: None,
	};
	// big photos make huge vectors; trace at a sensible size
	let edge = w.max(h).min(1600);
	let fit = format!("scale='min({edge},iw)':'min({edge},ih)':force_original_aspect_ratio=decrease");
	write(e, &flat, &fit, Format::Png, 100, true, &tmp, &AtomicBool::new(false))?;
	let (tw, th, rgba) = read_png(&tmp)?;
	let _ = fs::remove_file(&tmp);
	let img = vtracer::ColorImage {
		pixels: rgba,
		width: tw as usize,
		height: th as usize,
	};
	let cfg = vtracer::Config {
		filter_speckle: 4,
		color_precision: 6,
		layer_difference: 16,
		..vtracer::Config::default()
	};
	let svg = vtracer::convert(img, cfg).map_err(|e| Error::Engine(format!("trace: {e}")))?;
	fs::write(part, svg.to_string())?;
	Ok(())
}

fn animated_to_video(e: &Engines, input: &Path, to: Format, part: &Path, c: &AtomicBool) -> Result<(), Error> {
	let mut a = strs(&["-map", "0:v:0", "-vf", ff::EVEN]);
	a.extend(ff::video_codec_args(to, 20, false, 128));
	Run::new(&e.ffmpeg)
		.input(input)
		.args(a)
		.args(out(part))
		.go(&mut |_| {}, c)
		.map(drop)
}

fn animated_webp(e: &Engines, input: &Path, d: &Defaults, part: &Path, c: &AtomicBool) -> Result<(), Error> {
	let a = vec![
		"-map".to_string(),
		"0:v:0".into(),
		"-c:v".into(),
		"libwebp_anim".into(),
		"-quality".into(),
		d.image_quality.to_string(),
		"-loop".into(),
		"0".into(),
		"-f".into(),
		"webp".into(),
	];
	Run::new(&e.ffmpeg)
		.input(input)
		.args(a)
		.args(out(part))
		.go(&mut |_| {}, c)
		.map(drop)
}

/// Lays several pictures out on one canvas, or into a GIF.
pub fn combine(
	e: &Engines,
	inputs: &[PathBuf],
	layout: Layout,
	gap: u32,
	background: &str,
	part: &Path,
	c: &AtomicBool,
) -> Result<(), Error> {
	let mut pics = Vec::new();
	for i in inputs {
		let src = source(e, i)?;
		let tmp = temp_png(e, "pic")?;
		// pictures meet at the size of the smallest, so one huge photo doesn't dwarf the rest
		write(
			e,
			&src,
			"scale='min(2400,iw)':'min(2400,ih)':force_original_aspect_ratio=decrease",
			Format::Png,
			100,
			true,
			&tmp,
			c,
		)?;
		let px = read_png(&tmp)?;
		let _ = fs::remove_file(&tmp);
		pics.push(px);
	}
	if layout == Layout::Gif {
		return gif_from(e, &pics, part, c);
	}
	let n = pics.len();
	let (cols, rows) = match layout {
		Layout::Row => (n, 1),
		Layout::Column => (1, n),
		_ => {
			let cols = (n as f32).sqrt().ceil() as usize;
			(cols, n.div_ceil(cols))
		}
	};
	// every cell gets the same height (rows) or width (columns) for a tidy look
	let cell_h = pics.iter().map(|p| p.1).min().unwrap_or(1).min(1600);
	let cell_w = pics.iter().map(|p| p.0).min().unwrap_or(1).min(1600);
	let scaled: Vec<(f32, f32)> = pics
		.iter()
		.map(|(w, h, _)| match layout {
			Layout::Column => (cell_w as f32, *h as f32 * cell_w as f32 / *w as f32),
			_ => (*w as f32 * cell_h as f32 / *h as f32, cell_h as f32),
		})
		.collect();
	let col_w: Vec<f32> = (0..cols)
		.map(|ci| {
			(0..rows)
				.filter_map(|ri| scaled.get(ri * cols + ci).map(|s| s.0))
				.fold(0.0, f32::max)
		})
		.collect();
	let row_h: Vec<f32> = (0..rows)
		.map(|ri| {
			(0..cols)
				.filter_map(|ci| scaled.get(ri * cols + ci).map(|s| s.1))
				.fold(0.0, f32::max)
		})
		.collect();
	let g = gap as f32;
	let total_w = col_w.iter().sum::<f32>() + g * (cols as f32 + 1.0);
	let total_h = row_h.iter().sum::<f32>() + g * (rows as f32 + 1.0);
	let mut canvas = tiny_skia::Pixmap::new(total_w.ceil() as u32, total_h.ceil() as u32)
		.ok_or_else(|| Error::Engine("the combined picture would be too large".into()))?;
	if let Some(col) = parse_colour(background) {
		canvas.fill(col);
	}
	for (i, (w, h, rgba)) in pics.iter().enumerate() {
		let (ri, ci) = (i / cols, i % cols);
		let x = g + col_w[..ci].iter().sum::<f32>() + g * ci as f32 + (col_w[ci] - scaled[i].0) / 2.0;
		let y = g + row_h[..ri].iter().sum::<f32>() + g * ri as f32 + (row_h[ri] - scaled[i].1) / 2.0;
		let Some(pm) = pixmap_from(*w, *h, rgba) else { continue };
		let s = scaled[i].0 / *w as f32;
		let paint = tiny_skia::PixmapPaint {
			quality: tiny_skia::FilterQuality::Bicubic,
			..Default::default()
		};
		canvas.draw_pixmap(
			0,
			0,
			pm.as_ref(),
			&paint,
			tiny_skia::Transform::from_row(s, 0.0, 0.0, s, x, y),
			None,
		);
	}
	write_png(part, canvas.width(), canvas.height(), &unpremultiply(canvas.data()))
}

pub fn pixmap_from(w: u32, h: u32, rgba: &[u8]) -> Option<tiny_skia::Pixmap> {
	let mut pm = tiny_skia::Pixmap::new(w, h)?;
	for (dst, s) in pm.pixels_mut().iter_mut().zip(rgba.as_chunks::<4>().0.iter()) {
		*dst = tiny_skia::ColorU8::from_rgba(s[0], s[1], s[2], s[3]).premultiply();
	}
	Some(pm)
}

pub fn parse_colour(s: &str) -> Option<tiny_skia::Color> {
	let h = s.trim().trim_start_matches('#');
	let v = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
	match h.len() {
		6 => Some(tiny_skia::Color::from_rgba8(v(0)?, v(2)?, v(4)?, 255)),
		8 => Some(tiny_skia::Color::from_rgba8(v(0)?, v(2)?, v(4)?, v(6)?)),
		_ => None,
	}
}

fn gif_from(e: &Engines, pics: &[(u32, u32, Vec<u8>)], part: &Path, c: &AtomicBool) -> Result<(), Error> {
	let dir = e.scratch.join(format!("gif-{}", std::process::id()));
	fs::create_dir_all(&dir)?;
	let (w, h) = (pics[0].0.min(960), pics[0].1 * pics[0].0.min(960) / pics[0].0.max(1));
	for (i, (pw, ph, rgba)) in pics.iter().enumerate() {
		write_png(&dir.join(format!("{i:04}.png")), *pw, *ph, rgba)?;
	}
	let vf = format!(
		"scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=white,split[a][b];[a]palettegen[p];[b][p]paletteuse"
	);
	let result = Run::new(&e.ffmpeg)
		.args(strs(&["-framerate", "1"]))
		.input(&dir.join("%04d.png"))
		.args(strs(&["-vf", &vf, "-loop", "0", "-f", "gif"]))
		.args(out(part))
		.go(&mut |_| {}, c);
	let _ = fs::remove_dir_all(&dir);
	result.map(drop)
}

/// A preview the editor windows can show: the picture itself when the browser reads it, otherwise a PNG copy.
pub fn preview(e: &Engines, input: &Path) -> Result<PathBuf, Error> {
	let ext = ext_of(input);
	if matches!(
		ext.as_str(),
		"jpg" | "jpeg" | "jfif" | "png" | "webp" | "avif" | "gif" | "bmp" | "svg"
	) {
		let src = source(e, input)?;
		if src.upright.is_empty() {
			return Ok(input.to_path_buf());
		}
	}
	let src = source(e, input)?;
	let tmp = temp_png(e, "preview")?;
	write(e, &src, "", Format::Png, 100, true, &tmp, &AtomicBool::new(false))?;
	Ok(tmp)
}

/// What a picture's format would be after a recipe that keeps the format.
pub fn kept_format(input: &Path) -> Format {
	image_format_for(&ext_of(input)).unwrap_or_else(|| keep(input))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sizes_come_from_the_header() {
		let dir = std::env::temp_dir().join(format!("tw-head-{}", std::process::id()));
		fs::create_dir_all(&dir).unwrap();
		let png = dir.join("a.png");
		write_png(&png, 37, 21, &vec![200u8; 37 * 21 * 4]).unwrap();
		assert_eq!(header_size(&png), Some((37, 21)));
		// a JPEG frame header behind a big APP1 block, as phone photos have
		let mut jpg = vec![0xff, 0xd8, 0xff, 0xe1, 0x40, 0x02];
		jpg.extend(vec![0u8; 0x4000]);
		jpg.extend([0xff, 0xc0, 0x00, 0x11, 0x08, 0x02, 0x37, 0x04, 0xd2, 0x03]);
		jpg.extend(vec![0u8; 12]);
		let j = dir.join("a.jpg");
		fs::write(&j, &jpg).unwrap();
		assert_eq!(header_size(&j), Some((1234, 567)));
		let txt = dir.join("a.txt");
		fs::write(&txt, b"not a picture at all, just words here").unwrap();
		assert_eq!(header_size(&txt), None);
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn a_broken_psd_is_an_error_not_a_crash() {
		let dir = std::env::temp_dir().join(format!("tw-psd-{}", std::process::id()));
		fs::create_dir_all(&dir).unwrap();
		// a valid header whose colour-mode section claims far more bytes than the file has
		let mut psd = b"8BPS\x00\x01\x00\x00\x00\x00\x00\x00\x00\x03".to_vec();
		psd.extend([0, 0, 0, 16, 0, 0, 0, 16, 0, 8, 0, 3]);
		psd.extend([0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0x40, 0]);
		psd.extend([0, 0]);
		let path = dir.join("cut.psd");
		fs::write(&path, &psd).unwrap();
		let e = Engines::in_dir(&dir, dir.join("scratch"));
		let err = decode_special(&e, &path, "psd").unwrap_err();
		assert_eq!(err.to_string(), "This PSD can't be read");
		let _ = fs::remove_dir_all(dir);
	}
}
