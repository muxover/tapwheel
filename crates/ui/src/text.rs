use ab_glyph::{Font, FontVec, OutlineCurve, PxScale, ScaleFont, VariableFont, point};
use tiny_skia::{Path, PathBuilder, Pixmap};

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

	/// One line bent along the circle of radius `r` around (cx, cy), centred on `angle`, with the
	/// letters' middle on the circle. `flip` runs it the other way so text below the centre reads
	/// left to right instead of upside down.
	#[allow(clippy::too_many_arguments)]
	pub fn arc(
		&self,
		text: &str,
		size: f32,
		w: Weight,
		cx: f32,
		cy: f32,
		r: f32,
		angle: f32,
		flip: bool,
	) -> Option<Path> {
		let font = self.font(w).as_scaled(PxScale::from(size));
		let (sx, sy) = (font.h_scale_factor(), font.v_scale_factor());
		let total = self.width(text, size, w);
		let cap = font
			.font
			.outline(font.glyph_id('H'))
			.map(|o| o.bounds.max.y * sy)
			.unwrap_or(size * 0.7);
		let base = if flip { r + cap / 2.0 } else { r - cap / 2.0 };
		let mut pb = PathBuilder::new();
		let mut pen = 0.0;
		let mut prev = None;
		for c in text.chars() {
			let id = font.glyph_id(c);
			if let Some(p) = prev {
				pen += font.kern(p, id);
			}
			let adv = font.h_advance(id);
			let along = pen + adv / 2.0 - total / 2.0;
			let a = if flip {
				angle - along / base
			} else {
				angle + along / base
			};
			let (s, co) = a.sin_cos();
			let (t, up) = if flip {
				((s, -co), (-co, -s))
			} else {
				((-s, co), (co, s))
			};
			let (ox, oy) = (cx + co * base, cy + s * base);
			let at = |p: ab_glyph::Point| {
				let (u, v) = (p.x * sx - adv / 2.0, p.y * sy);
				(ox + u * t.0 + v * up.0, oy + u * t.1 + v * up.1)
			};
			if let Some(o) = font.font.outline(id) {
				let mut last: Option<ab_glyph::Point> = None;
				for curve in &o.curves {
					let (start, end) = match *curve {
						OutlineCurve::Line(a, b) => (a, b),
						OutlineCurve::Quad(a, _, b) => (a, b),
						OutlineCurve::Cubic(a, _, _, b) => (a, b),
					};
					// the outline is one list of curves; a gap starts the next contour
					if last.is_none_or(|l| (l.x - start.x).abs() > 0.01 || (l.y - start.y).abs() > 0.01) {
						if last.is_some() {
							pb.close();
						}
						let (x, y) = at(start);
						pb.move_to(x, y);
					}
					match *curve {
						OutlineCurve::Line(_, b) => {
							let (x, y) = at(b);
							pb.line_to(x, y);
						}
						OutlineCurve::Quad(_, c1, b) => {
							let ((x1, y1), (x, y)) = (at(c1), at(b));
							pb.quad_to(x1, y1, x, y);
						}
						OutlineCurve::Cubic(_, c1, c2, b) => {
							let ((x1, y1), (x2, y2), (x, y)) = (at(c1), at(c2), at(b));
							pb.cubic_to(x1, y1, x2, y2, x, y);
						}
					}
					last = Some(end);
				}
				if last.is_some() {
					pb.close();
				}
			}
			pen += adv;
			prev = Some(id);
		}
		pb.finish()
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
