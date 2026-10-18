use ab_glyph::{Font, FontVec, PxScale, ScaleFont, VariableFont, point};
use tiny_skia::Pixmap;

use crate::theme::Rgba;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weight {
	Regular,
	Semibold,
}

pub struct Fonts {
	regular: FontVec,
	semibold: FontVec,
}

impl Fonts {
	/// `semibold` may be the same variable font as `regular`; its weight axis is set to 600.
	pub fn new(regular: Vec<u8>, semibold: Vec<u8>) -> Option<Fonts> {
		let mut regular = FontVec::try_from_vec(regular).ok()?;
		let mut semibold = FontVec::try_from_vec(semibold).ok()?;
		regular.set_variation(b"wght", 400.0);
		semibold.set_variation(b"wght", 600.0);
		Some(Fonts { regular, semibold })
	}

	/// A bundled font for tests on machines without Segoe UI.
	#[cfg(test)]
	pub fn fallback() -> Fonts {
		let f = include_bytes!("../../core/assets/JetBrainsMono-Regular.ttf").to_vec();
		Fonts::new(f.clone(), f).expect("bundled font")
	}

	fn font(&self, w: Weight) -> &FontVec {
		match w {
			Weight::Regular => &self.regular,
			Weight::Semibold => &self.semibold,
		}
	}

	pub fn width(&self, text: &str, size: f32, w: Weight) -> f32 {
		let font = self.font(w).as_scaled(PxScale::from(size));
		let mut x = 0.0;
		let mut prev = None;
		for c in text.chars() {
			let id = font.glyph_id(c);
			if let Some(p) = prev {
				x += font.kern(p, id);
			}
			x += font.h_advance(id);
			prev = Some(id);
		}
		x
	}

	/// Distance from the top of the line to the baseline.
	pub fn ascent(&self, size: f32, w: Weight) -> f32 {
		self.font(w).as_scaled(PxScale::from(size)).ascent()
	}

	pub fn line_height(&self, size: f32, w: Weight) -> f32 {
		let f = self.font(w).as_scaled(PxScale::from(size));
		f.ascent() - f.descent()
	}

	/// Draws one line with its top-left at (x, y). Text wider than `max` fades out
	/// over its last pixels instead of being cut with an ellipsis.
	pub fn draw(
		&self,
		pm: &mut Pixmap,
		text: &str,
		x: f32,
		y: f32,
		size: f32,
		w: Weight,
		color: Rgba,
		max: Option<f32>,
	) {
		let font = self.font(w).as_scaled(PxScale::from(size));
		let baseline = y + font.ascent();
		let full = self.width(text, size, w);
		let limit = max.filter(|m| full > *m);
		let fade = (size * 2.0).min(limit.unwrap_or(0.0) / 2.0);
		let width = pm.width() as i32;
		let height = pm.height() as i32;
		let data = pm.data_mut();
		let mut pen = x;
		let mut prev = None;
		for c in text.chars() {
			let id = font.glyph_id(c);
			if let Some(p) = prev {
				pen += font.kern(p, id);
			}
			let glyph = id.with_scale_and_position(size, point(pen, baseline));
			pen += font.h_advance(id);
			prev = Some(id);
			if limit.is_some_and(|m| pen - font.h_advance(id) > x + m) {
				break;
			}
			let Some(outline) = font.outline_glyph(glyph) else {
				continue;
			};
			let b = outline.px_bounds();
			outline.draw(|gx, gy, cov| {
				let px = b.min.x as i32 + gx as i32;
				let py = b.min.y as i32 + gy as i32;
				if px < 0 || py < 0 || px >= width || py >= height {
					return;
				}
				let mut a = cov.clamp(0.0, 1.0) * color.a;
				if let Some(m) = limit {
					let right = x + m;
					let fx = px as f32;
					if fx >= right {
						return;
					}
					if fx > right - fade {
						a *= (right - fx) / fade;
					}
				}
				blend(data, ((py * width + px) * 4) as usize, color, a);
			});
		}
	}

	pub fn draw_centered(
		&self,
		pm: &mut Pixmap,
		text: &str,
		cx: f32,
		y: f32,
		size: f32,
		w: Weight,
		color: Rgba,
		max: Option<f32>,
	) {
		let width = self.width(text, size, w);
		let shown = max.map_or(width, |m| width.min(m));
		self.draw(pm, text, cx - shown / 2.0, y, size, w, color, max);
	}
}

/// Source-over onto premultiplied RGBA.
fn blend(data: &mut [u8], i: usize, c: Rgba, a: f32) {
	if a <= 0.0 {
		return;
	}
	let inv = 1.0 - a;
	let px = &mut data[i..i + 4];
	px[0] = (c.r * a * 255.0 + px[0] as f32 * inv).round().min(255.0) as u8;
	px[1] = (c.g * a * 255.0 + px[1] as f32 * inv).round().min(255.0) as u8;
	px[2] = (c.b * a * 255.0 + px[2] as f32 * inv).round().min(255.0) as u8;
	px[3] = (a * 255.0 + px[3] as f32 * inv).round().min(255.0) as u8;
}
