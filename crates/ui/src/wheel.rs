use std::f32::consts::{PI, TAU};

use tiny_skia::Pixmap;

use crate::draw::*;
use crate::text::{Fonts, Weight};
use crate::theme::Theme;
use crate::wrap;

pub const WIDTH: f32 = 344.0;
pub const HEIGHT: f32 = 400.0;
pub const CX: f32 = WIDTH / 2.0;
pub const CY: f32 = 172.0;
pub const R_OUT: f32 = 152.0;
pub const R_IN: f32 = 66.0;
const R_CENTER: f32 = 58.0;
const PILL_Y: f32 = 344.0;
const PILL_H: f32 = 40.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
	pub label: String,
	pub desc: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
	None,
	Slice(usize),
	Center,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyHint {
	pub key: String,
	pub name: String,
	pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WheelView {
	pub slices: Vec<Slice>,
	pub hover: Hover,
	/// Opened from the keyboard or the right-click menu: number the slices and show focus.
	pub keyboard: bool,
	pub title: String,
	pub subtitle: String,
	pub keys: Vec<KeyHint>,
	/// Shown in the centre when there is nothing to pick.
	pub message: Option<String>,
	/// Progress of the open animation, 0 to 1.
	pub appear: f32,
}

pub fn size(scale: f32) -> (u32, u32) {
	((WIDTH * scale).ceil() as u32, (HEIGHT * scale).ceil() as u32)
}

fn slice_angles(n: usize, i: usize) -> (f32, f32, f32) {
	let step = TAU / n as f32;
	let mid = -PI / 2.0 + step * i as f32;
	(mid - step / 2.0, mid, mid + step / 2.0)
}

/// What sits under a point given in logical pixels from the window's top-left.
pub fn hit(n: usize, x: f32, y: f32) -> Hover {
	let (dx, dy) = (x - CX, y - CY);
	let d = (dx * dx + dy * dy).sqrt();
	if d < R_IN {
		return Hover::Center;
	}
	// a little slack past the rim so a quick flick still lands
	if d > R_OUT + 14.0 || n == 0 {
		return Hover::None;
	}
	let step = TAU / n as f32;
	let a = (dy.atan2(dx) + PI / 2.0 + step / 2.0).rem_euclid(TAU);
	Hover::Slice(((a / step) as usize).min(n - 1))
}

/// Whether a window pixel should catch the mouse and drops.
pub fn solid(x: f32, y: f32) -> bool {
	let (dx, dy) = (x - CX, y - CY);
	dx * dx + dy * dy <= (R_OUT + 14.0).powi(2)
}

pub fn render(v: &WheelView, theme: &Theme, fonts: &Fonts, scale: f32) -> Pixmap {
	let (w, h) = size(scale);
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	let p = theme.palette();
	let s = scale;
	let (cx, cy) = (CX * s, CY * s);

	if !theme.high_contrast {
		shadow_circle(&mut pm, cx, cy, R_OUT * s, 12.0 * s, p.shadow);
	}
	fill_circle(&mut pm, cx, cy, R_OUT * s, p.surface);
	stroke_circle(
		&mut pm,
		cx,
		cy,
		R_OUT * s,
		s.max(1.0),
		if theme.high_contrast {
			p.stroke
		} else {
			p.stroke.alpha(1.6)
		},
	);

	let n = v.slices.len();
	for (i, slice) in v.slices.iter().enumerate() {
		let (a0, mid, a1) = slice_angles(n, i);
		let hovered = v.hover == Hover::Slice(i);
		if hovered {
			let gap = if n > 1 { 0.012 } else { 0.0 };
			if let Some(seg) = ring_segment(cx, cy, (R_IN + 4.0) * s, (R_OUT - 4.0) * s, a0 + gap, a1 - gap) {
				fill_path(&mut pm, &seg, p.accent);
			}
		}
		if n > 1 && !theme.high_contrast {
			let (c, sn) = (a0.cos(), a0.sin());
			line(
				&mut pm,
				cx + c * (R_IN + 14.0) * s,
				cy + sn * (R_IN + 14.0) * s,
				cx + c * (R_OUT - 14.0) * s,
				cy + sn * (R_OUT - 14.0) * s,
				s,
				p.stroke.alpha(1.4),
			);
		}
		let r_mid = (R_IN + R_OUT) / 2.0 + 2.0;
		let (lx, ly) = if n == 1 {
			(cx, cy - r_mid * s)
		} else {
			(cx + mid.cos() * r_mid * s, cy + mid.sin() * r_mid * s)
		};
		let chord = if n == 1 {
			140.0
		} else {
			2.0 * r_mid * ((TAU / n as f32) / 2.0).sin() * 0.86
		};
		let max = chord.min(R_OUT - R_IN) * s;
		let color = if hovered { p.on_accent } else { p.text };
		let size = 14.0 * s;
		let lh = fonts.line_height(size, Weight::Semibold);
		let mut top = ly - lh / 2.0;
		if v.keyboard && i < 9 {
			let nsize = 11.0 * s;
			let nh = fonts.line_height(nsize, Weight::Regular);
			top += nh / 2.0;
			let ncolor = if hovered { p.on_accent.alpha(0.8) } else { p.text2 };
			fonts.draw_centered(
				&mut pm,
				&(i + 1).to_string(),
				lx,
				top - nh,
				nsize,
				Weight::Regular,
				ncolor,
				None,
			);
		}
		fonts.draw_centered(&mut pm, &slice.label, lx, top, size, Weight::Semibold, color, Some(max));
		if hovered
			&& v.keyboard
			&& let Some(seg) = ring_segment(cx, cy, (R_IN + 2.0) * s, (R_OUT - 2.0) * s, a0 + 0.006, a1 - 0.006)
		{
			let st = tiny_skia::Stroke {
				width: 2.0 * s,
				..Default::default()
			};
			pm.stroke_path(&seg, &paint(p.text), &st, tiny_skia::Transform::identity(), None);
		}
	}

	// centre
	let center_hover = v.hover == Hover::Center;
	fill_circle(
		&mut pm,
		cx,
		cy,
		R_CENTER * s,
		if center_hover {
			p.raised.mix(p.text, 0.08)
		} else {
			p.raised
		},
	);
	stroke_circle(&mut pm, cx, cy, R_CENTER * s, s.max(1.0), p.stroke);
	let inner = 98.0 * s;
	let (head, body) = match v.hover {
		Hover::Slice(i) if i < n => (None, v.slices[i].desc.clone()),
		Hover::Center => (Some("Cancel".to_string()), "Drop here to leave it".to_string()),
		_ => match &v.message {
			Some(m) => (None, m.clone()),
			None => (Some(v.title.clone()), v.subtitle.clone()),
		},
	};
	let head_size = 15.0 * s;
	let body_size = 12.0 * s;
	let body_weight = if head.is_none() {
		Weight::Semibold
	} else {
		Weight::Regular
	};
	let body_size = if head.is_none() { 13.0 * s } else { body_size };
	let lines = wrap(fonts, &body, body_size, body_weight, inner, 3);
	let head_h = if head.is_some() {
		fonts.line_height(head_size, Weight::Semibold) + 2.0 * s
	} else {
		0.0
	};
	let body_lh = fonts.line_height(body_size, body_weight);
	let total = head_h + body_lh * lines.len() as f32;
	let mut y = cy - total / 2.0;
	if let Some(hd) = &head {
		fonts.draw_centered(&mut pm, hd, cx, y, head_size, Weight::Semibold, p.text, Some(inner));
		y += head_h;
	}
	let body_color = if head.is_some() { p.text2 } else { p.text };
	for l in &lines {
		fonts.draw_centered(&mut pm, l, cx, y, body_size, body_weight, body_color, Some(inner));
		y += body_lh;
	}

	draw_keys(&mut pm, v, theme, fonts, s);
	if v.appear < 1.0 { appear(&pm, v.appear) } else { pm }
}

fn draw_keys(pm: &mut Pixmap, v: &WheelView, theme: &Theme, fonts: &Fonts, s: f32) {
	if v.keys.is_empty() {
		return;
	}
	let p = theme.palette();
	let key_size = 11.0 * s;
	let name_size = 12.0 * s;
	let pad = 10.0 * s;
	let gap = 14.0 * s;
	let widths: Vec<(f32, f32)> = v
		.keys
		.iter()
		.map(|k| {
			let kw = fonts.width(&k.key, key_size, Weight::Semibold) + 12.0 * s;
			let nw = fonts.width(
				&k.name,
				name_size,
				if k.active { Weight::Semibold } else { Weight::Regular },
			);
			(kw, nw)
		})
		.collect();
	let inner: f32 = widths.iter().map(|(k, n)| k + 6.0 * s + n).sum::<f32>() + gap * (widths.len() as f32 - 1.0);
	let total = inner + pad * 2.0;
	let x0 = CX * s - total / 2.0;
	let y0 = PILL_Y * s;
	let h = PILL_H * s;
	let r = if theme.win11 { h / 2.0 } else { 0.0 };
	if !theme.high_contrast {
		shadow_rrect(pm, x0, y0, total, h, r, 8.0 * s, p.shadow);
	}
	fill_rrect(pm, x0, y0, total, h, r, p.surface);
	stroke_rrect(pm, x0, y0, total, h, r, s.max(1.0), p.stroke.alpha(1.6));
	let mut x = x0 + pad;
	for (k, (kw, nw)) in v.keys.iter().zip(widths) {
		let kh = 20.0 * s;
		let ky = y0 + (h - kh) / 2.0;
		let kr = if theme.win11 { 4.0 * s } else { 0.0 };
		if k.active {
			fill_rrect(pm, x, ky, kw, kh, kr, p.accent);
		} else {
			stroke_rrect(pm, x, ky, kw, kh, kr, s.max(1.0), p.text2.alpha(0.6));
		}
		let kcolor = if k.active { p.on_accent } else { p.text2 };
		let kty = ky + (kh - fonts.line_height(key_size, Weight::Semibold)) / 2.0;
		fonts.draw_centered(pm, &k.key, x + kw / 2.0, kty, key_size, Weight::Semibold, kcolor, None);
		x += kw + 6.0 * s;
		let weight = if k.active { Weight::Semibold } else { Weight::Regular };
		let ny = y0 + (h - fonts.line_height(name_size, weight)) / 2.0;
		fonts.draw(
			pm,
			&k.name,
			x,
			ny,
			name_size,
			weight,
			if k.active { p.text } else { p.text2 },
			None,
		);
		x += nw + gap;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hit_testing_follows_the_slices() {
		// slice 0 is straight up, slices go clockwise
		assert_eq!(hit(4, CX, CY - 100.0), Hover::Slice(0));
		assert_eq!(hit(4, CX + 100.0, CY), Hover::Slice(1));
		assert_eq!(hit(4, CX, CY + 100.0), Hover::Slice(2));
		assert_eq!(hit(4, CX - 100.0, CY), Hover::Slice(3));
		assert_eq!(hit(4, CX, CY), Hover::Center);
		assert_eq!(hit(4, 2.0, 2.0), Hover::None);
		assert_eq!(hit(1, CX + 100.0, CY + 30.0), Hover::Slice(0));
		assert_eq!(hit(0, CX, CY - 100.0), Hover::None);
	}

	#[test]
	fn boundaries_split_evenly() {
		let n = 8;
		for i in 0..n {
			let (_, mid, _) = slice_angles(n, i);
			let (x, y) = (CX + mid.cos() * 110.0, CY + mid.sin() * 110.0);
			assert_eq!(hit(n, x, y), Hover::Slice(i));
		}
	}
}
