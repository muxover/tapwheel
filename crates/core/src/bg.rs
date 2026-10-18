//! Background removal with a segmentation model run by ONNX Runtime, fully offline.
//!
//! The runtime library and the model ship next to the app in `engines/` and load on first use,
//! so nothing is downloaded and the app starts as fast without them.

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, OnceLock};

use crate::Error;
use crate::ff::decode_rgba;
use crate::image::{Source, size_of, write_png};
use crate::run::Engines;

/// The model's square input size.
const SIDE: u32 = 1024;
/// ISNet reads pixels from -0.5 to 0.5.
const MEAN: [f32; 3] = [0.5, 0.5, 0.5];
const STD: [f32; 3] = [1.0, 1.0, 1.0];

/// The model ships with half-precision weights; this writes the full-precision copy it runs from,
/// once, next to a copy of the graph. Returns the graph to load.
fn expand(e: &Engines) -> Result<std::path::PathBuf, Error> {
	let half = e.model.with_extension("f16");
	let dir = e.cache.join("model");
	let graph = dir.join("background.onnx");
	let weights = dir.join("background.weights");
	let want = std::fs::metadata(&half)?.len() * 2;
	let ready = std::fs::metadata(&weights).map(|m| m.len() == want).unwrap_or(false)
		&& std::fs::metadata(&graph).map(|m| m.len()).ok() == std::fs::metadata(&e.model).map(|m| m.len()).ok();
	if ready {
		return Ok(graph);
	}
	std::fs::create_dir_all(&dir)?;
	let raw = std::fs::read(&half)?;
	let mut full = Vec::with_capacity(raw.len() * 2);
	for pair in raw.as_chunks::<2>().0.iter() {
		let h = half::f16::from_le_bytes([pair[0], pair[1]]);
		full.extend_from_slice(&h.to_f32().to_le_bytes());
	}
	let part = weights.with_extension("weights.tw-part");
	std::fs::write(&part, full)?;
	std::fs::rename(&part, &weights)?;
	std::fs::copy(&e.model, &graph)?;
	Ok(graph)
}

static SESSION: OnceLock<Mutex<Option<ort::session::Session>>> = OnceLock::new();

fn session(e: &Engines) -> Result<std::sync::MutexGuard<'static, Option<ort::session::Session>>, Error> {
	let cell = SESSION.get_or_init(|| Mutex::new(None));
	let mut guard = cell
		.lock()
		.map_err(|_| Error::Engine("the background model failed earlier".into()))?;
	if guard.is_none() {
		if !e.model.exists() || !e.model.with_extension("f16").exists() || !e.onnxruntime.exists() {
			return Err(Error::Engine(
				"the background model is missing; reinstall TapWheel".into(),
			));
		}
		fn fail(w: &'static str) -> impl Fn(ort::Error) -> Error {
			move |x| Error::Engine(format!("background model {w}: {x}"))
		}
		ort::init_from(e.onnxruntime.to_string_lossy().into_owned())
			.with_name("tapwheel")
			.commit()
			.map_err(fail("runtime"))?;
		let threads = std::thread::available_parallelism()
			.map(|n| n.get())
			.unwrap_or(4)
			.clamp(1, 8);
		let s = ort::session::Session::builder()
			.map_err(fail("setup"))?
			.with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
			.map_err(fail("setup"))?
			.with_intra_threads(threads)
			.map_err(fail("setup"))?
			.commit_from_file(expand(e)?)
			.map_err(fail("load"))?;
		*guard = Some(s);
	}
	Ok(guard)
}

/// Loads the model ahead of time, so the first cut-out doesn't wait for it. Safe to call more
/// than once; only the first call does any work.
pub fn warm(e: &Engines) {
	drop(session(e));
}

/// Bilinear resize of one channel (or RGBA when `ch` is 4) to `w` × `h`.
fn resize(src: &[f32], sw: u32, sh: u32, ch: usize, w: u32, h: u32) -> Vec<f32> {
	let mut out = vec![0.0; (w * h) as usize * ch];
	let fx = sw as f32 / w as f32;
	let fy = sh as f32 / h as f32;
	for y in 0..h {
		let sy = ((y as f32 + 0.5) * fy - 0.5).clamp(0.0, (sh - 1) as f32);
		let (y0, ty) = (sy.floor() as u32, sy.fract());
		let y1 = (y0 + 1).min(sh - 1);
		for x in 0..w {
			let sx = ((x as f32 + 0.5) * fx - 0.5).clamp(0.0, (sw - 1) as f32);
			let (x0, tx) = (sx.floor() as u32, sx.fract());
			let x1 = (x0 + 1).min(sw - 1);
			for c in 0..ch {
				let p = |xx: u32, yy: u32| src[((yy * sw + xx) as usize) * ch + c];
				let top = p(x0, y0) * (1.0 - tx) + p(x1, y0) * tx;
				let bottom = p(x0, y1) * (1.0 - tx) + p(x1, y1) * tx;
				out[((y * w + x) as usize) * ch + c] = top * (1.0 - ty) + bottom * ty;
			}
		}
	}
	out
}

/// Averages down big pictures before the bilinear step, so fine detail doesn't alias.
fn shrink_rgba(rgba: &[u8], w: u32, h: u32) -> (Vec<f32>, u32, u32) {
	let mut f: Vec<f32> = rgba.iter().map(|&v| v as f32 / 255.0).collect();
	let (mut w, mut h) = (w, h);
	while w >= SIDE * 2 && h >= SIDE * 2 {
		let (nw, nh) = (w / 2, h / 2);
		let mut half = vec![0.0; (nw * nh * 4) as usize];
		for y in 0..nh {
			for x in 0..nw {
				for c in 0..4 {
					let p = |xx: u32, yy: u32| f[((yy * w + xx) * 4) as usize + c];
					half[((y * nw + x) * 4) as usize + c] =
						(p(2 * x, 2 * y) + p(2 * x + 1, 2 * y) + p(2 * x, 2 * y + 1) + p(2 * x + 1, 2 * y + 1)) / 4.0;
				}
			}
		}
		f = half;
		w = nw;
		h = nh;
	}
	(f, w, h)
}

/// The subject's mask at the picture's own size, 0 to 1.
pub fn mask(e: &Engines, rgba: &[u8], w: u32, h: u32) -> Result<Vec<f32>, Error> {
	let (small, sw, sh) = shrink_rgba(rgba, w, h);
	let input = resize(&small, sw, sh, 4, SIDE, SIDE);
	let plane = (SIDE * SIDE) as usize;
	let mut chw = vec![0.0f32; 3 * plane];
	for i in 0..plane {
		for c in 0..3 {
			chw[c * plane + i] = (input[i * 4 + c] - MEAN[c]) / STD[c];
		}
	}
	let mut guard = session(e)?;
	let s = guard.as_mut().expect("loaded above");
	let fail = |x: ort::Error| Error::Engine(format!("background model: {x}"));
	let tensor = ort::value::Tensor::from_array(([1usize, 3, SIDE as usize, SIDE as usize], chw)).map_err(fail)?;
	let outputs = s.run(ort::inputs![tensor]).map_err(fail)?;
	// the first output is the finest prediction
	let (_, raw) = outputs[0].try_extract_tensor::<f32>().map_err(fail)?;
	let raw = &raw[..plane];
	// logits from BiRefNet-style models, already 0..1 from others
	let logits = raw.iter().any(|v| *v < -0.01 || *v > 1.01);
	let mut m: Vec<f32> = raw
		.iter()
		.map(|&v| if logits { 1.0 / (1.0 + (-v).exp()) } else { v })
		.collect();
	let (lo, hi) = m
		.iter()
		.fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
	if hi - lo > 1e-6 {
		for v in &mut m {
			*v = (*v - lo) / (hi - lo);
		}
	}
	Ok(resize(&m, SIDE, SIDE, 1, w, h))
}

/// Applies a mask as transparency, with a gentle curve so edges stay soft but the background goes fully clear.
pub fn cut_out(rgba: &[u8], mask: &[f32]) -> Vec<u8> {
	rgba.as_chunks::<4>()
		.0
		.iter()
		.zip(mask)
		.flat_map(|(p, &m)| {
			let a = ((m - 0.06) / 0.88).clamp(0.0, 1.0);
			let a = a * a * (3.0 - 2.0 * a);
			[p[0], p[1], p[2], (p[3] as f32 * a).round() as u8]
		})
		.collect()
}

pub fn remove(e: &Engines, src: &Source, part: &Path, c: &AtomicBool) -> Result<(), Error> {
	let (w, h) = size_of(e, src)?;
	let upright_src = if src.upright.is_empty() {
		decode_rgba(&e.ffmpeg, &src.path, (w, h))?
	} else {
		// decode through a PNG so the EXIF turn is applied
		let tmp = e.scratch.join(format!("bg-{}.png", std::process::id()));
		crate::image::write(e, src, "", crate::format::Format::Png, 100, true, &tmp, c)?;
		let (_, _, px) = crate::image::read_png(&tmp)?;
		let _ = std::fs::remove_file(&tmp);
		px
	};
	if c.load(std::sync::atomic::Ordering::Relaxed) {
		return Err(Error::Cancelled);
	}
	let m = mask(e, &upright_src, w, h)?;
	write_png(part, w, h, &cut_out(&upright_src, &m))
}

/// The cut-out of `input` as a PNG in the scratch folder, upright and at full size, for the
/// touch-up window to edit. Its alpha is the mask the window starts from.
pub fn cutout_file(e: &Engines, input: &Path) -> Result<std::path::PathBuf, Error> {
	let src = crate::image::source(e, input)?;
	let _ = std::fs::create_dir_all(&e.scratch);
	static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
	let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
	let out = e.scratch.join(format!("cutout-{}-{n}.png", std::process::id()));
	remove(e, &src, &out, &AtomicBool::new(false))?;
	Ok(out)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn resize_keeps_flat_colour() {
		let src = vec![0.5f32; 4 * 4];
		let out = resize(&src, 4, 4, 1, 9, 7);
		assert!(out.iter().all(|v| (v - 0.5).abs() < 1e-6));
	}

	#[test]
	fn cut_out_clears_the_background() {
		let px = [10, 20, 30, 255, 10, 20, 30, 255, 10, 20, 30, 255];
		let out = cut_out(&px, &[0.0, 0.5, 1.0]);
		assert_eq!(out[3], 0);
		assert!(out[7] > 100 && out[7] < 160);
		assert_eq!(out[11], 255);
	}
}
