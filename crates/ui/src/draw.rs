use std::f32::consts::PI;

use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};

use crate::theme::Rgba;

pub fn paint(c: Rgba) -> Paint<'static> {
	let mut p = Paint::default();
	p.set_color(c.skia());
	p.anti_alias = true;
	p
}

pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
	let r = r.min(w / 2.0).min(h / 2.0);
	if r <= 0.0 {
		return Some(PathBuilder::from_rect(Rect::from_xywh(x, y, w, h)?));
	}
	// cubic approximation of a quarter circle
	let k = r * 0.552_284_8;
	let mut pb = PathBuilder::new();
	pb.move_to(x + r, y);
	pb.line_to(x + w - r, y);
	pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
	pb.line_to(x + w, y + h - r);
	pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
	pb.line_to(x + r, y + h);
	pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
	pb.line_to(x, y + r);
	pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
	pb.close();
	pb.finish()
}

pub fn fill_rrect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, c: Rgba) {
	if let Some(p) = rounded_rect(x, y, w, h, r) {
		pm.fill_path(&p, &paint(c), FillRule::Winding, Transform::identity(), None);
	}
}

pub fn stroke_rrect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, width: f32, c: Rgba) {
	let half = width / 2.0;
	if let Some(p) = rounded_rect(x + half, y + half, w - width, h - width, (r - half).max(0.0)) {
		let s = Stroke {
			width,
			..Stroke::default()
		};
		pm.stroke_path(&p, &paint(c), &s, Transform::identity(), None);
	}
}

pub fn circle(cx: f32, cy: f32, r: f32) -> Option<Path> {
	PathBuilder::from_circle(cx, cy, r)
}

pub fn fill_circle(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, c: Rgba) {
	if let Some(p) = circle(cx, cy, r) {
		pm.fill_path(&p, &paint(c), FillRule::Winding, Transform::identity(), None);
	}
}

pub fn stroke_circle(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, width: f32, c: Rgba) {
	if let Some(p) = circle(cx, cy, r) {
		let s = Stroke {
			width,
			..Stroke::default()
		};
		pm.stroke_path(&p, &paint(c), &s, Transform::identity(), None);
	}
}

/// A ring segment between two radii, angles in radians, clockwise from `a0` to `a1`.
pub fn ring_segment(cx: f32, cy: f32, r_in: f32, r_out: f32, a0: f32, a1: f32) -> Option<Path> {
	let steps = (((a1 - a0).abs() / (PI / 90.0)).ceil() as usize).max(2);
	let mut pb = PathBuilder::new();
	for i in 0..=steps {
		let a = a0 + (a1 - a0) * i as f32 / steps as f32;
		let (x, y) = (cx + r_out * a.cos(), cy + r_out * a.sin());
		if i == 0 {
			pb.move_to(x, y);
		} else {
			pb.line_to(x, y);
		}
	}
	for i in (0..=steps).rev() {
		let a = a0 + (a1 - a0) * i as f32 / steps as f32;
		pb.line_to(cx + r_in * a.cos(), cy + r_in * a.sin());
	}
	pb.close();
	pb.finish()
}

pub fn fill_path(pm: &mut Pixmap, p: &Path, c: Rgba) {
	pm.fill_path(p, &paint(c), FillRule::Winding, Transform::identity(), None);
}

pub fn line(pm: &mut Pixmap, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, c: Rgba) {
	let mut pb = PathBuilder::new();
	pb.move_to(x0, y0);
	pb.line_to(x1, y1);
	if let Some(p) = pb.finish() {
		let s = Stroke {
			width,
			line_cap: tiny_skia::LineCap::Round,
			..Stroke::default()
		};
		pm.stroke_path(&p, &paint(c), &s, Transform::identity(), None);
	}
}

/// Soft drop shadow under a rounded rectangle, built from stacked translucent outlines.
pub fn shadow_rrect(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, spread: f32, c: Rgba) {
	let steps = 8;
	for i in 0..steps {
		let t = i as f32 / steps as f32;
		let grow = spread * (1.0 - t);
		let a = c.a * 0.12 * (0.3 + t);
		fill_rrect(
			pm,
			x - grow,
			y - grow + spread * 0.35,
			w + grow * 2.0,
			h + grow * 2.0,
			r + grow,
			c.alpha(a / c.a.max(0.001)),
		);
	}
}

pub fn shadow_circle(pm: &mut Pixmap, cx: f32, cy: f32, r: f32, spread: f32, c: Rgba) {
	let steps = 8;
	for i in 0..steps {
		let t = i as f32 / steps as f32;
		let grow = spread * (1.0 - t);
		let a = c.a * 0.12 * (0.3 + t);
		fill_circle(pm, cx, cy + spread * 0.35, r + grow, c.alpha(a / c.a.max(0.001)));
	}
}

/// Rounded-end bar.
pub fn bar(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, c: Rgba) {
	if w > 0.0 {
		fill_rrect(pm, x, y, w, h, h / 2.0, c);
	}
}

/// Checkmark inside a box of `size` at (x, y).
pub fn check(pm: &mut Pixmap, x: f32, y: f32, size: f32, width: f32, c: Rgba) {
	let mut pb = PathBuilder::new();
	pb.move_to(x + size * 0.18, y + size * 0.52);
	pb.line_to(x + size * 0.42, y + size * 0.74);
	pb.line_to(x + size * 0.84, y + size * 0.28);
	if let Some(p) = pb.finish() {
		let s = Stroke {
			width,
			line_cap: tiny_skia::LineCap::Round,
			line_join: tiny_skia::LineJoin::Round,
			..Stroke::default()
		};
		pm.stroke_path(&p, &paint(c), &s, Transform::identity(), None);
	}
}

pub fn cross(pm: &mut Pixmap, cx: f32, cy: f32, size: f32, width: f32, c: Rgba) {
	let h = size / 2.0;
	line(pm, cx - h, cy - h, cx + h, cy + h, width, c);
	line(pm, cx - h, cy + h, cx + h, cy - h, width, c);
}

/// Scales and fades a finished frame around its centre, for the open animation.
pub fn appear(src: &Pixmap, t: f32) -> Pixmap {
	let t = t.clamp(0.0, 1.0);
	if t >= 1.0 {
		return src.clone();
	}
	let mut out = Pixmap::new(src.width(), src.height()).expect("same size");
	let ease = 1.0 - (1.0 - t).powi(3);
	let s = 0.92 + 0.08 * ease;
	let (cx, cy) = (src.width() as f32 / 2.0, src.height() as f32 / 2.0);
	let tf = Transform::from_translate(cx, cy)
		.pre_scale(s, s)
		.pre_translate(-cx, -cy);
	let p = tiny_skia::PixmapPaint {
		opacity: ease,
		quality: tiny_skia::FilterQuality::Bilinear,
		..Default::default()
	};
	out.draw_pixmap(0, 0, src.as_ref(), &p, tf, None);
	out
}

/// Premultiplied RGBA to the premultiplied BGRA a Windows DIB wants.
pub fn to_bgra(pm: &Pixmap) -> Vec<u8> {
	let mut out = pm.data().to_vec();
	for px in out.chunks_exact_mut(4) {
		px.swap(0, 2);
	}
	out
}
