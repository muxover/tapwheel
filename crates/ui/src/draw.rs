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
	for px in out.as_chunks_mut::<4>().0 {
		px.swap(0, 2);
	}
	out
}

/// Draws a Fluent icon by name, `size` pixels square, top-left at (x, y).
pub fn icon(pm: &mut Pixmap, name: &str, x: f32, y: f32, size: f32, c: Rgba) -> bool {
	let Some(d) = crate::icons::path(name) else {
		return false;
	};
	let Some(path) = svg_path(d) else {
		return false;
	};
	let s = size / 20.0;
	pm.fill_path(
		&path,
		&paint(c),
		FillRule::Winding,
		Transform::from_row(s, 0.0, 0.0, s, x, y),
		None,
	);
	true
}

/// SVG path data into a tiny-skia path. Arcs are approximated by the parser's cubic output.
pub fn svg_path(d: &str) -> Option<Path> {
	use svgtypes::{SimplePathSegment as S, SimplifyingPathParser};
	let mut pb = PathBuilder::new();
	for seg in SimplifyingPathParser::from(d) {
		match seg.ok()? {
			S::MoveTo { x, y } => pb.move_to(x as f32, y as f32),
			S::LineTo { x, y } => pb.line_to(x as f32, y as f32),
			S::CurveTo { x1, y1, x2, y2, x, y } => {
				pb.cubic_to(x1 as f32, y1 as f32, x2 as f32, y2 as f32, x as f32, y as f32)
			}
			S::Quadratic { x1, y1, x, y } => pb.quad_to(x1 as f32, y1 as f32, x as f32, y as f32),
			S::ClosePath => pb.close(),
		}
	}
	pb.finish()
}

/// A ring segment with rounded corners: the segment is shrunk by `corner`, then grown back with
/// round joins. Both shapes go into one mask first, so see-through colours don't double up.
pub fn petal(pm: &mut Pixmap, cx: f32, cy: f32, r_in: f32, r_out: f32, a0: f32, a1: f32, corner: f32, c: Rgba) {
	let mid_r = (r_in + r_out) / 2.0;
	let pad = (corner / mid_r).min((a1 - a0) / 2.0 - 0.001);
	let Some(core) = ring_segment(cx, cy, r_in + corner, r_out - corner, a0 + pad, a1 - pad) else {
		return;
	};
	if corner <= 0.0 || c.a >= 0.999 {
		let paint = paint(c);
		pm.fill_path(&core, &paint, FillRule::Winding, Transform::identity(), None);
		if corner > 0.0 {
			let s = Stroke {
				width: corner * 2.0,
				line_join: tiny_skia::LineJoin::Round,
				..Stroke::default()
			};
			pm.stroke_path(&core, &paint, &s, Transform::identity(), None);
		}
		return;
	}
	let b = core.bounds();
	let (x0, y0) = (
		(b.left() - corner - 2.0).floor().max(0.0),
		(b.top() - corner - 2.0).floor().max(0.0),
	);
	let (w, h) = (
		((b.right() + corner + 2.0).ceil() - x0).max(1.0) as u32,
		((b.bottom() + corner + 2.0).ceil() - y0).max(1.0) as u32,
	);
	let Some(mut mask) = tiny_skia::Mask::new(w, h) else {
		return;
	};
	let shift = Transform::from_translate(-x0, -y0);
	mask.fill_path(&core, FillRule::Winding, true, shift);
	let s = Stroke {
		width: corner * 2.0,
		line_join: tiny_skia::LineJoin::Round,
		..Stroke::default()
	};
	if let Some(outline) = core.stroke(&s, 1.0) {
		mask.fill_path(&outline, FillRule::Winding, true, shift);
	}
	let Some(rect) = Rect::from_xywh(0.0, 0.0, w as f32, h as f32) else {
		return;
	};
	let mut tile = match Pixmap::new(w, h) {
		Some(t) => t,
		None => return,
	};
	tile.fill_rect(rect, &paint(c), Transform::identity(), Some(&mask));
	pm.draw_pixmap(
		x0 as i32,
		y0 as i32,
		tile.as_ref(),
		&tiny_skia::PixmapPaint::default(),
		Transform::identity(),
		None,
	);
}

/// Three box-blur passes, close to a gaussian, on premultiplied pixels.
pub fn blur(pm: &mut Pixmap, radius: usize) {
	if radius == 0 {
		return;
	}
	let (w, h) = (pm.width() as usize, pm.height() as usize);
	let data = pm.data_mut();
	let mut tmp = vec![0u8; data.len()];
	for _ in 0..3 {
		box_pass(data, &mut tmp, w, h, radius, true);
		box_pass(&tmp, data, w, h, radius, false);
	}
}

fn box_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, r: usize, horizontal: bool) {
	let (len, lines) = if horizontal { (w, h) } else { (h, w) };
	let at = |line: usize, i: usize| {
		if horizontal {
			(line * w + i) * 4
		} else {
			(i * w + line) * 4
		}
	};
	let div = (2 * r + 1) as u32;
	for line in 0..lines {
		let mut sum = [0u32; 4];
		for k in 0..=2 * r {
			let i = k.saturating_sub(r).min(len - 1);
			for c in 0..4 {
				sum[c] += src[at(line, i) + c] as u32;
			}
		}
		for i in 0..len {
			for c in 0..4 {
				dst[at(line, i) + c] = (sum[c] / div) as u8;
			}
			let out = i.saturating_sub(r);
			let inn = (i + r + 1).min(len - 1);
			for c in 0..4 {
				sum[c] = sum[c] + src[at(line, inn) + c] as u32 - src[at(line, out) + c] as u32;
			}
		}
	}
}

pub fn ease_out(t: f32) -> f32 {
	1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Overshoots a little before settling, for things that pop open.
pub fn ease_back(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	let c = 1.4;
	1.0 + (c + 1.0) * (t - 1.0).powi(3) + c * (t - 1.0).powi(2)
}
