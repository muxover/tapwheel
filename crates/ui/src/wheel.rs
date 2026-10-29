//! The wheel: petals with icons and labels, an outer ring of choices, pages past eight slices,
//! and the motion for opening, hovering, picking and closing. Pure drawing, so it renders in tests.

use std::f32::consts::{PI, TAU};

use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::draw::*;
use crate::text::{Fonts, Weight};
use crate::theme::{Rgba, Theme};
use crate::wrap;

pub const WIDTH: f32 = 600.0;
pub const HEIGHT: f32 = 600.0;
pub const CX: f32 = WIDTH / 2.0;
pub const CY: f32 = 290.0;
pub const R_OUT: f32 = 172.0;
pub const R_IN: f32 = 70.0;
const R_CENTER: f32 = 62.0;
const RING_IN: f32 = R_OUT + 8.0;
const RING_OUT: f32 = R_OUT + 58.0;
/// How far past the ring's ends (radians) and inside the slices' rim (px) the open ring holds on.
const RING_SLACK: f32 = 0.12;
/// The even gap between neighbouring slices, in logical pixels.
const GAP: f32 = 4.0;
const RIM: f32 = 28.0;
/// The reach of the invisible layer that keeps the pointer on the wheel: past the ring's slack.
const CATCH: f32 = RING_OUT + 44.0;
const PILL_Y: f32 = CY + 254.0;
const PILL_H: f32 = 40.0;
/// Slices on one page; with more, the last one turns the page.
pub const PER_PAGE: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
	pub label: String,
	/// Fluent icon name; empty draws the label alone, larger (format names).
	pub icon: String,
	pub desc: String,
	/// Choices shown on the ring when this slice is hovered.
	pub ring: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hover {
	None,
	Center,
	/// A slice on the current page, counted from the page's first.
	Slice(usize),
	/// A choice on the ring around a slice.
	Ring(usize, usize),
	/// The page-turning slice.
	More,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyHint {
	pub key: String,
	pub name: String,
	pub active: bool,
}

/// Where each moving part is, all 0 to 1. The app advances these on a timer.
#[derive(Clone, Debug, PartialEq)]
pub struct Motion {
	pub open: f32,
	/// Closing: 0 while open, rising to 1 as it goes.
	pub close: f32,
	/// The slice that was picked, which pulses while the rest fade.
	pub picked: Option<usize>,
	/// How hovered each slice on the page looks, easing in and out.
	pub hover: Vec<f32>,
	/// How far the ring has grown out.
	pub ring: f32,
	/// Turning to a new page: 0 at the start, 1 when done.
	pub page: f32,
}

impl Default for Motion {
	fn default() -> Self {
		Motion {
			open: 1.0,
			close: 0.0,
			picked: None,
			hover: Vec::new(),
			ring: 1.0,
			page: 1.0,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct WheelView {
	/// Every slice; the wheel shows one page of them.
	pub slices: Vec<Slice>,
	pub page: usize,
	pub hover: Hover,
	/// Opened from the keyboard or the right-click menu: number the slices and show focus.
	pub keyboard: bool,
	pub title: String,
	pub subtitle: String,
	pub keys: Vec<KeyHint>,
	/// Shown in the centre when there is nothing to pick.
	pub message: Option<String>,
	/// Extra line for the hovered slice, like the size a compression will give.
	pub hint: Option<String>,
	pub motion: Motion,
}

pub fn size(scale: f32) -> (u32, u32) {
	((WIDTH * scale).ceil() as u32, (HEIGHT * scale).ceil() as u32)
}

/// Where the wheel's window goes and how big it is, centred on `pt` on the monitor whose work
/// area is `work` (left, top, right, bottom in screen pixels, which can be negative on a monitor
/// left of or above the main one), at that monitor's scale times the size picked in Settings.
/// The wheel shrinks to fit a small work area and never crosses its edges.
pub fn place_at(pt: (i32, i32), work: [i32; 4], monitor_scale: f32, size_factor: f32) -> (i32, i32, f32) {
	let [wl, wt, wr, wb] = work;
	let scale = fit_scale(monitor_scale * size_factor, wr - wl, wb - wt);
	let (w, h) = size(scale);
	let x = (pt.0 - (CX * scale) as i32).clamp(wl, (wr - w as i32).max(wl));
	let y = (pt.1 - (CY * scale) as i32).clamp(wt, (wb - h as i32).max(wt));
	(x, y, scale)
}

/// Where the wheel's window goes so its petals open just beside a file on screen (`item` is the
/// file's left, top, right, bottom) rather than under the pointer: to the right of the file's name,
/// or to its left when the screen ends first, kept inside `work`. A details-view row is as wide as
/// the window, so only its first part, where the name is, counts.
pub fn beside(item: [i32; 4], work: [i32; 4], scale: f32) -> (i32, i32) {
	let [left, top, right, bottom] = item;
	let [wl, wt, wr, wb] = work;
	let (w, h) = size(scale);
	let (w, h) = (w as i32, h as i32);
	let reach = ((R_OUT + 16.0) * scale) as i32;
	let name_end = right.min(left + (260.0 * scale) as i32);
	let mid = (top + bottom) / 2;
	let cx = if name_end + reach + (R_OUT * scale) as i32 <= wr {
		name_end + reach
	} else {
		left - reach
	};
	let x = (cx - (CX * scale) as i32).clamp(wl, (wr - w).max(wl));
	let y = (mid - (CY * scale) as i32).clamp(wt, (wb - h).max(wt));
	(x, y)
}

/// The scale to draw at on a work area of `work_w` by `work_h` pixels: the display's own, or
/// smaller when the wheel wouldn't fit, as on a 1366 by 768 screen at 125%.
pub fn fit_scale(scale: f32, work_w: i32, work_h: i32) -> f32 {
	let room = (work_w.max(1) as f32 / WIDTH).min(work_h.max(1) as f32 / HEIGHT);
	// a pixel to spare, so rounding the size up never pushes past the edge
	scale.min(room * (1.0 - 1.0 / HEIGHT))
}

pub fn pages(n: usize) -> usize {
	if n <= PER_PAGE { 1 } else { n.div_ceil(PER_PAGE - 1) }
}

/// The slices on a page, by index into `slices`, and whether the page ends in More.
pub fn page_items(n: usize, page: usize) -> (Vec<usize>, bool) {
	if n <= PER_PAGE {
		return ((0..n).collect(), false);
	}
	let per = PER_PAGE - 1;
	let start = page * per;
	((start..(start + per).min(n)).collect(), true)
}

fn count_on_page(v: &WheelView) -> usize {
	let (items, more) = page_items(v.slices.len(), v.page);
	items.len() + more as usize
}

fn angles(n: usize, i: usize) -> (f32, f32, f32) {
	let step = TAU / n as f32;
	let mid = -PI / 2.0 + step * i as f32;
	(mid - step / 2.0, mid, mid + step / 2.0)
}

/// Angle spans of the ring choices around slice `i`, centred on it.
fn ring_layout(v: &WheelView, fonts: &Fonts, i: usize) -> Vec<(f32, f32)> {
	let (items, _) = page_items(v.slices.len(), v.page);
	let Some(slice) = items.get(i).and_then(|&k| v.slices.get(k)) else {
		return Vec::new();
	};
	if slice.ring.is_empty() {
		return Vec::new();
	}
	let n = count_on_page(v);
	let (_, mid, _) = angles(n, i);
	let r = (RING_IN + RING_OUT) / 2.0;
	let gap = 6.0 / r;
	let widths: Vec<f32> = slice
		.ring
		.iter()
		.map(|l| (fonts.width(l, 12.5, Weight::Semibold) + 30.0).max(64.0) / r)
		.collect();
	let total: f32 = widths.iter().sum::<f32>() + gap * (widths.len() as f32 - 1.0);
	let mut a = mid - total / 2.0;
	widths
		.into_iter()
		.map(|w| {
			let span = (a, a + w);
			a += w + gap;
			span
		})
		.collect()
}

/// Kept clear at each end of a label's arc, inside the petal's rounded corners.
const LABEL_PAD: f32 = 8.0;

/// Where a slice's label goes: one or two lines, each on its own arc (first line first), the
/// size that fits, which way the text runs, and the icon's radius.
#[derive(Debug, Clone)]
pub struct LabelFit {
	pub lines: Vec<(String, f32)>,
	pub size: f32,
	pub flip: bool,
	pub icon_r: Option<f32>,
}

/// Below the centre, text runs the other way so it never reads upside down.
fn flipped(mid: f32) -> bool {
	let (s, c) = mid.sin_cos();
	s > 1e-3 || (s.abs() <= 1e-3 && c < 0.0)
}

/// Fits a label into a slice `span` radians wide: one line if it fits at any size, else two
/// lines split where they balance. Labels that fit neither way are left for the label test.
pub fn fit_label(fonts: &Fonts, label: &str, has_icon: bool, span: f32, mid: f32) -> LabelFit {
	let flip = flipped(mid);
	let sizes: &[f32] = if has_icon {
		&[12.5, 12.0, 11.5, 11.0]
	} else {
		&[16.0, 15.0, 14.0, 13.0]
	};
	// the icon sits above the text as it reads: outside on the top half, inside on the bottom
	let (icon_r, text_r) = match (has_icon, flip) {
		(true, false) => (Some(140.0), 104.0),
		(true, true) => (Some(104.0), 140.0),
		(false, _) => (None, (R_IN + R_OUT) / 2.0),
	};
	let room = |r: f32| span.min(PI) * r - 2.0 * LABEL_PAD;
	let width = |t: &str, size: f32| fonts.width(t, size, Weight::Semibold);
	for &size in sizes {
		if width(label, size) <= room(text_r) {
			return LabelFit {
				lines: vec![(label.to_string(), text_r)],
				size,
				flip,
				icon_r,
			};
		}
	}
	let words: Vec<&str> = label.split(' ').collect();
	for &size in sizes {
		let gap = fonts.line_height(size, Weight::Semibold) * 0.9;
		let (r1, r2) = if flip {
			(text_r - gap / 2.0, text_r + gap / 2.0)
		} else {
			(text_r + gap / 2.0, text_r - gap / 2.0)
		};
		let best = (1..words.len())
			.map(|k| (words[..k].join(" "), words[k..].join(" ")))
			.min_by(|a, b| {
				let wa = width(&a.0, size).max(width(&a.1, size));
				let wb = width(&b.0, size).max(width(&b.1, size));
				wa.total_cmp(&wb)
			});
		if let Some((a, b)) = best
			&& width(&a, size) <= room(r1)
			&& width(&b, size) <= room(r2)
		{
			return LabelFit {
				lines: vec![(a, r1), (b, r2)],
				size,
				flip,
				icon_r,
			};
		}
	}
	LabelFit {
		lines: vec![(label.to_string(), text_r)],
		size: sizes[sizes.len() - 1],
		flip,
		icon_r,
	}
}

/// Every label on the open wheel as filled outlines, each with the shape it must stay inside
/// (the petal or ring chip, less its rounded corners). Used by the label test.
// `items` is one short when the last slice is More
#[allow(clippy::needless_range_loop)]
pub fn label_shapes(v: &WheelView, fonts: &Fonts, win11: bool) -> Vec<(String, tiny_skia::Path, tiny_skia::Path)> {
	let mut out = Vec::new();
	let n = count_on_page(v);
	let (items, more) = page_items(v.slices.len(), v.page);
	let corner = if win11 { 8.0 } else { 2.0 };
	let gap = if n > 1 { GAP } else { 0.0 };
	let ga = gap / 2.0 / R_OUT;
	for k in 0..n {
		let (a0, mid, a1) = angles(n, k);
		let mid = if n == 1 { -PI / 2.0 } else { mid };
		let (label, has_icon) = if more && k == n - 1 {
			("More".to_string(), true)
		} else {
			let sl = &v.slices[items[k]];
			(sl.label.clone(), !sl.icon.is_empty())
		};
		let fit = fit_label(fonts, &label, has_icon, a1 - a0 - 2.0 * ga, mid);
		let (r_in, r_out) = (R_IN + 6.0 + corner, R_OUT - 6.0 - corner);
		let Some(region) = slice_shape(CX, CY, r_in, r_out, a0, a1, gap / 2.0 + corner) else {
			continue;
		};
		for (line, r) in &fit.lines {
			if let Some(path) = fonts.arc(line, fit.size, Weight::Semibold, CX, CY, *r, mid, fit.flip) {
				out.push((label.clone(), path, region.clone()));
			}
		}
	}
	if let Hover::Slice(i) | Hover::Ring(i, _) = v.hover
		&& let Some(&k) = items.get(i)
	{
		let rc = if win11 { 8.0 } else { 2.0 };
		for (j, (a0, a1)) in ring_layout(v, fonts, i).into_iter().enumerate() {
			let label = &v.slices[k].ring[j];
			let mid = (a0 + a1) / 2.0;
			let rr = (RING_IN + RING_OUT) / 2.0;
			let pad = rc / rr;
			let Some(region) = ring_segment(CX, CY, RING_IN + rc, RING_OUT - rc, a0 + pad, a1 - pad) else {
				continue;
			};
			if let Some(path) = fonts.arc(label, 12.5, Weight::Semibold, CX, CY, rr, mid, flipped(mid)) {
				out.push((label.clone(), path, region));
			}
		}
	}
	out
}

fn angle_in(a: f32, a0: f32, a1: f32) -> bool {
	let a = (a - a0).rem_euclid(TAU);
	a <= (a1 - a0)
}

/// What sits under a point given in logical pixels from the window's top-left.
pub fn hit(v: &WheelView, fonts: &Fonts, x: f32, y: f32) -> Hover {
	let (dx, dy) = (x - CX, y - CY);
	let d = (dx * dx + dy * dy).sqrt();
	let a = dy.atan2(dx);
	let n = count_on_page(v);
	// The open ring owns the band from the slices' outer edge to a little past its chips, across
	// its whole arc. Its chips fan out wider than their slice, so on the way to an outer chip the
	// pointer crosses the rim of a neighbour; that must not switch slices and close the ring.
	// Inside the band the nearest chip is picked, so the gaps between chips are never dead.
	if let Hover::Slice(i) | Hover::Ring(i, _) = v.hover {
		let spans = ring_layout(v, fonts, i);
		if let (Some(first), Some(last)) = (spans.first(), spans.last()) {
			let in_arc = angle_in(a, first.0 - RING_SLACK, last.1 + RING_SLACK);
			if in_arc && (RING_IN - 4.0..RING_OUT + 40.0).contains(&d) {
				let off = |m: f32| (a - m + PI).rem_euclid(TAU) - PI;
				let j = (0..spans.len())
					.min_by(|&x, &y| {
						let mx = off((spans[x].0 + spans[x].1) / 2.0).abs();
						let my = off((spans[y].0 + spans[y].1) / 2.0).abs();
						mx.total_cmp(&my)
					})
					.unwrap_or(0);
				return Hover::Ring(i, j);
			}
			if in_arc && d > R_OUT - RIM && d < RING_IN - 4.0 {
				return Hover::Slice(i);
			}
		}
	}
	if d < R_IN {
		return Hover::Center;
	}
	// a little slack past the rim so a quick flick still lands
	if d > R_OUT + 12.0 || n == 0 {
		return Hover::None;
	}
	let step = TAU / n as f32;
	let k = ((a + PI / 2.0 + step / 2.0).rem_euclid(TAU) / step) as usize;
	let k = k.min(n - 1);
	let (_, more) = page_items(v.slices.len(), v.page);
	if more && k == n - 1 {
		Hover::More
	} else {
		Hover::Slice(k)
	}
}

/// Index into `slices` for a hovered slice on the current page.
pub fn slice_index(v: &WheelView, i: usize) -> Option<usize> {
	page_items(v.slices.len(), v.page).0.get(i).copied()
}

fn glass(theme: &Theme) -> Rgba {
	theme.palette().surface
}

/// Draws the wheel. `backdrop` is a blurred copy of the screen behind it, for the Windows 11 glass.
// `items` is one short when the last slice is More
#[allow(clippy::needless_range_loop)]
pub fn render(v: &WheelView, theme: &Theme, fonts: &Fonts, scale: f32, backdrop: Option<&Pixmap>) -> Pixmap {
	let (w, h) = size(scale);
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	let p = theme.palette();
	let s = scale;
	let (cx, cy) = (CX * s, CY * s);
	// Windows lets the pointer fall through a layered window's fully clear pixels, and a drag
	// that crosses one "leaves" the wheel. An all but invisible disc out past the ring keeps the
	// gaps between slices, the ring and its chips part of the wheel.
	fill_circle(&mut pm, cx, cy, CATCH * s, Rgba::hex(0x000000).alpha(2.0 / 255.0));
	let m = &v.motion;
	let n = count_on_page(v);
	let (items, more) = page_items(v.slices.len(), v.page);
	let corner = if theme.win11 { 8.0 * s } else { 2.0 * s };

	// disc, with frosted glass behind on Windows 11
	if !theme.high_contrast {
		shadow_circle(&mut pm, cx, cy, R_OUT * s, 16.0 * s, p.shadow);
	}
	let mut frosted = false;
	if let Some(bd) = backdrop.filter(|_| theme.win11 && !theme.high_contrast)
		&& let Some(mask) = disc_mask(w, h, cx, cy, R_OUT * s)
	{
		pm.draw_pixmap(
			0,
			0,
			bd.as_ref(),
			&PixmapPaint::default(),
			Transform::identity(),
			Some(&mask),
		);
		frosted = true;
	}
	// the backdrop already carries the acrylic tint; without one the disc is solid
	if !frosted {
		fill_circle(&mut pm, cx, cy, R_OUT * s, glass(theme));
	}
	stroke_circle(
		&mut pm,
		cx,
		cy,
		R_OUT * s,
		s.max(1.0),
		if theme.high_contrast {
			p.stroke
		} else {
			p.stroke.alpha(1.8)
		},
	);

	// petals; a page turn slides them round
	let turn = (1.0 - ease_out(m.page)) * 0.5;
	let tile = if theme.dark {
		Rgba::hex(0xffffff).alpha(0.045)
	} else {
		Rgba::hex(0x000000).alpha(0.035)
	};
	for k in 0..n {
		let (mut a0, mut mid, mut a1) = angles(n, k);
		a0 += turn;
		mid += turn;
		a1 += turn;
		let hv = m.hover.get(k).copied().unwrap_or(0.0);
		let hovered = matches!(v.hover, Hover::Slice(i) | Hover::Ring(i, _) if i == k)
			|| (more && k == n - 1 && v.hover == Hover::More);
		let picked = m.picked == Some(k);
		// picked pops out; the rest fade as the wheel closes
		let fade = if m.picked.is_some() && !picked {
			1.0 - ease_out(m.close * 2.0)
		} else {
			1.0
		};
		// hover only colours a slice; moving it would crowd its neighbours. A pick pops it out
		let pop = if picked { 8.0 * (m.close * PI).sin() * s } else { 0.0 };
		let (ox, oy) = (mid.cos() * pop, mid.sin() * pop);
		let gap = if n > 1 { GAP * s } else { 0.0 };
		let ga = gap / 2.0 / (R_OUT * s);
		let fill = tile.mix(p.accent, hv).alpha(fade);
		slice(
			&mut pm,
			cx + ox,
			cy + oy,
			(R_IN + 6.0) * s,
			(R_OUT - 6.0) * s,
			a0,
			a1,
			gap,
			corner,
			fill,
		);
		if theme.high_contrast
			&& hovered
			&& let Some(seg) = slice_shape(cx, cy, (R_IN + 6.0) * s, (R_OUT - 6.0) * s, a0, a1, gap / 2.0)
		{
			let st = tiny_skia::Stroke {
				width: 2.0 * s,
				..Default::default()
			};
			pm.stroke_path(&seg, &paint(p.text), &st, Transform::identity(), None);
		}

		let ink = p.text.mix(p.on_accent, hv).alpha(fade);
		let (label, icon_name) = if more && k == n - 1 {
			("More".to_string(), "more_horizontal".to_string())
		} else {
			let sl = &v.slices[items[k]];
			(sl.label.clone(), sl.icon.clone())
		};
		let mid = if n == 1 { -PI / 2.0 } else { mid };
		let fit = fit_label(fonts, &label, !icon_name.is_empty(), a1 - a0 - 2.0 * ga, mid);
		let (pcx, pcy) = (cx + ox, cy + oy);
		let at = |r: f32| (pcx + mid.cos() * r * s, pcy + mid.sin() * r * s);
		if let Some(ir) = fit.icon_r {
			let size = 24.0 * s;
			let (x, y) = at(ir);
			icon(&mut pm, &icon_name, x - size / 2.0, y - size / 2.0, size, ink);
		}
		for (line, r) in &fit.lines {
			if let Some(path) = fonts.arc(line, fit.size * s, Weight::Semibold, pcx, pcy, r * s, mid, fit.flip) {
				fill_path(&mut pm, &path, ink);
			}
		}
		if v.keyboard && k < 9 && !(more && k == n - 1) {
			let nsize = 10.5 * s;
			let (x, y) = at(if fit.flip { 84.0 } else { 158.0 });
			let nh = fonts.line_height(nsize, Weight::Regular);
			fonts.draw_centered(
				&mut pm,
				&(k + 1).to_string(),
				x,
				y - nh / 2.0,
				nsize,
				Weight::Regular,
				p.text2.mix(p.on_accent, hv).alpha(fade),
				None,
			);
		}
		if more && k == n - 1 {
			// page dots under the label as it reads
			let last = fit.lines.last().map_or(104.0, |l| l.1);
			let r = if fit.flip { last + 14.0 } else { last - 14.0 };
			let dots = pages(v.slices.len());
			for d in 0..dots {
				let off = (d as f32 - (dots as f32 - 1.0) / 2.0) * 9.0 / r;
				let a = if fit.flip { mid - off } else { mid + off };
				let c = if d == v.page { ink } else { ink.alpha(0.35) };
				fill_circle(&mut pm, pcx + a.cos() * r * s, pcy + a.sin() * r * s, 2.6 * s, c);
			}
		}
		if hovered
			&& v.keyboard
			&& let Some(seg) = slice_shape(
				cx + ox,
				cy + oy,
				(R_IN + 4.0) * s,
				(R_OUT - 4.0) * s,
				a0,
				a1,
				gap / 2.0 - 2.0 * s,
			) {
			let st = tiny_skia::Stroke {
				width: 2.0 * s,
				..Default::default()
			};
			pm.stroke_path(&seg, &paint(p.text), &st, Transform::identity(), None);
			// Windows' focus outline is two rings: the text colour outside, the surface inside
			if let Some(inner) = slice_shape(
				cx + ox,
				cy + oy,
				(R_IN + 5.5) * s,
				(R_OUT - 5.5) * s,
				a0,
				a1,
				gap / 2.0 - 0.5 * s,
			) {
				let st = tiny_skia::Stroke {
					width: s.max(1.0),
					..Default::default()
				};
				pm.stroke_path(&inner, &paint(p.surface), &st, Transform::identity(), None);
			}
		}
	}

	// the ring of choices around the hovered slice
	if let Hover::Slice(i) | Hover::Ring(i, _) = v.hover
		&& m.picked.is_none()
	{
		let spans = ring_layout(v, fonts, i);
		if let Some(&k) = items.get(i) {
			let grow = ease_back(m.ring);
			let r0 = (R_OUT + (RING_IN - R_OUT) * grow) * s;
			let r1 = (R_OUT + (RING_OUT - R_OUT) * grow) * s;
			let alpha = ease_out(m.ring * 1.6);
			let rc = if theme.win11 { 8.0 * s } else { 2.0 * s };
			for (j, (a0, a1)) in spans.iter().enumerate() {
				let on = v.hover == Hover::Ring(i, j);
				if !theme.high_contrast {
					petal(
						&mut pm,
						cx,
						cy + 3.0 * s,
						r0,
						r1,
						*a0,
						*a1,
						rc,
						p.shadow.alpha(0.5 * alpha),
					);
				}
				// fill and shadow only, like Windows 11's menus; high contrast keeps an edge
				let bg = if on { p.accent } else { p.raised };
				petal(&mut pm, cx, cy, r0, r1, *a0, *a1, rc, bg.alpha(alpha));
				if theme.high_contrast
					&& !on && let Some(seg) =
					ring_segment(cx, cy, r0 + rc / 2.0, r1 - rc / 2.0, *a0 + 0.005, *a1 - 0.005)
				{
					let st = tiny_skia::Stroke {
						width: s.max(1.0),
						..Default::default()
					};
					pm.stroke_path(&seg, &paint(p.stroke), &st, Transform::identity(), None);
				}
				let mid = (a0 + a1) / 2.0;
				let rr = (r0 + r1) / 2.0;
				let label = &v.slices[k].ring[j];
				let ink = if on { p.on_accent } else { p.text };
				if let Some(path) = fonts.arc(label, 12.5 * s, Weight::Semibold, cx, cy, rr, mid, flipped(mid)) {
					fill_path(&mut pm, &path, ink.alpha(alpha));
				}
			}
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
			p.raised.mix(p.text, 0.1)
		} else {
			p.raised
		},
	);
	stroke_circle(&mut pm, cx, cy, R_CENTER * s, s.max(1.0), p.stroke.alpha(1.6));
	let inner = 104.0 * s;
	let (head, body): (Option<String>, String) = match v.hover {
		Hover::Slice(i) => match slice_index(v, i) {
			Some(k) => (
				Some(v.slices[k].label.clone()),
				v.hint.clone().unwrap_or_else(|| v.slices[k].desc.clone()),
			),
			None => (None, String::new()),
		},
		Hover::Ring(i, j) => match slice_index(v, i) {
			Some(k) => (
				Some(v.slices[k].ring.get(j).cloned().unwrap_or_default()),
				v.hint.clone().unwrap_or_else(|| v.slices[k].label.clone()),
			),
			None => (None, String::new()),
		},
		Hover::More => (
			Some("More".into()),
			format!("Page {} of {}", v.page + 1, pages(v.slices.len())),
		),
		Hover::Center => (Some("Cancel".into()), "Drop here to leave it".into()),
		Hover::None => match &v.message {
			Some(msg) => (None, msg.clone()),
			None => (Some(v.title.clone()), v.subtitle.clone()),
		},
	};
	let head_size = 15.0 * s;
	let body_size = if head.is_none() { 13.0 * s } else { 12.0 * s };
	let body_weight = if head.is_none() {
		Weight::Semibold
	} else {
		Weight::Regular
	};
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
	finish(pm, m)
}

/// Opening scales up with a little overshoot; closing shrinks and fades.
fn finish(pm: Pixmap, m: &Motion) -> Pixmap {
	if m.open >= 1.0 && m.close <= 0.0 {
		return pm;
	}
	let (scale, alpha) = if m.close > 0.0 {
		let t = ease_out(m.close);
		let s = if m.picked.is_some() {
			1.0 - 0.06 * t
		} else {
			1.0 - 0.1 * t
		};
		(s, 1.0 - t)
	} else {
		(0.86 + 0.14 * ease_back(m.open), ease_out(m.open * 1.4))
	};
	let mut out = Pixmap::new(pm.width(), pm.height()).expect("same size");
	let (cx, cy) = (pm.width() as f32 / 2.0, CY / HEIGHT * pm.height() as f32);
	let tf = Transform::from_translate(cx, cy)
		.pre_scale(scale, scale)
		.pre_translate(-cx, -cy);
	let paint = PixmapPaint {
		opacity: alpha.clamp(0.0, 1.0),
		quality: tiny_skia::FilterQuality::Bilinear,
		..Default::default()
	};
	out.draw_pixmap(0, 0, pm.as_ref(), &paint, tf, None);
	out
}

fn disc_mask(w: u32, h: u32, cx: f32, cy: f32, r: f32) -> Option<tiny_skia::Mask> {
	let mut mask = tiny_skia::Mask::new(w, h)?;
	let path = circle(cx, cy, r)?;
	mask.fill_path(&path, tiny_skia::FillRule::Winding, true, Transform::identity());
	Some(mask)
}

/// An all-but-invisible disc that still catches drags, used while checking what's being dragged.
pub fn probe(scale: f32) -> Pixmap {
	let (w, h) = size(scale);
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	fill_circle(
		&mut pm,
		CX * scale,
		CY * scale,
		(R_OUT + 12.0) * scale,
		Rgba::hex(0x000000).alpha(1.5 / 255.0),
	);
	pm
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

	fn inside(x: i32, y: i32, scale: f32, work: [i32; 4]) -> bool {
		let (w, h) = size(scale);
		x >= work[0] && y >= work[1] && x + w as i32 <= work[2] && y + h as i32 <= work[3]
	}

	#[test]
	fn the_wheel_fits_every_screen() {
		// (work area, monitor scale), taskbar along the bottom
		let screens = [
			([0, 0, 1366, 708], 1.25, "1366x768 at 125%"),
			([0, 0, 1280, 680], 1.0, "1280x720 at 100%"),
			([0, 0, 1920, 1032], 1.5, "1920x1080 at 150%"),
			([0, 0, 2560, 1392], 1.75, "2560x1440 at 175%"),
			([0, 0, 3840, 2088], 2.0, "3840x2160 at 200%"),
			([0, 0, 3840, 2088], 2.25, "3840x2160 at 225%"),
		];
		for (work, ms, name) in screens {
			for factor in [0.85, 1.0, 1.15] {
				for pt in [
					(work[0] + 5, work[1] + 5),
					((work[0] + work[2]) / 2, (work[1] + work[3]) / 2),
					(work[2] - 5, work[3] - 5),
				] {
					let (x, y, scale) = place_at(pt, work, ms, factor);
					assert!(inside(x, y, scale, work), "{name}, size {factor}, pointer {pt:?}");
					assert!(scale <= ms * factor + f32::EPSILON, "{name}: never bigger than asked");
				}
			}
		}
		// on a roomy screen the wheel keeps the size asked for
		let (_, _, s) = place_at((1000, 500), [0, 0, 3840, 2088], 1.5, 1.0);
		assert_eq!(s, 1.5);
		// 1366x768 at 125%: the 750 px wheel shrinks to the 708 px work area
		let (_, _, s) = place_at((683, 354), [0, 0, 1366, 708], 1.25, 1.0);
		assert!(s < 1.25 && size(s).1 <= 708);
	}

	#[test]
	fn on_two_monitors_the_wheel_stays_on_the_one_under_the_pointer() {
		// a 4K at 150% on the left of a 1080p main screen at 100%: negative coordinates
		let left = [-3840, -200, 0, 1912];
		let main = [0, 0, 1920, 1032];
		let (x, y, s) = place_at((-20, 900), left, 1.5, 1.0);
		assert!(
			inside(x, y, s, left),
			"near the shared edge it doesn't spill onto the main screen"
		);
		assert_eq!(s, 1.5);
		let (x, y, s) = place_at((10, 10), main, 1.0, 1.0);
		assert!(inside(x, y, s, main));
		assert_eq!(s, 1.0);
		// beside a file near the shared edge, too
		let (x, y) = beside([-90, 400, -10, 480], left, 1.5);
		assert!(inside(x, y, 1.5, left));
	}

	#[test]
	fn the_wheel_opens_beside_a_file_not_on_it() {
		let work = [0, 0, 1920, 1040];
		// an icon on the desktop's left: the petals open to its right, clear of it
		let icon = [20, 300, 100, 380];
		let (x, y) = beside(icon, work, 1.0);
		let centre = (x + CX as i32, y + CY as i32);
		assert!(centre.0 - R_OUT as i32 >= icon[2], "petals cover the icon: {centre:?}");
		assert_eq!(centre.1, 340);
		// near the right edge it opens to the left instead
		let icon = [1800, 300, 1880, 380];
		let (x, _) = beside(icon, work, 1.0);
		assert!(x + CX as i32 + (R_OUT as i32) <= icon[0], "petals cover the icon");
		// a details-view row spans the window: the wheel sits by the name, still on screen
		let row = [300, 500, 1900, 522];
		let (x, y) = beside(row, work, 1.5);
		let (w, h) = size(1.5);
		assert!(x >= 0 && x + w as i32 <= 1920 && y >= 0 && y + h as i32 <= 1040);
		// at the bottom of the screen it stays inside the work area
		let low = [20, 1000, 100, 1030];
		let (_, y) = beside(low, work, 1.0);
		assert!(y + size(1.0).1 as i32 <= 1040);
	}

	use super::*;

	#[test]
	fn the_wheel_fits_a_small_screen() {
		// 1366 by 768 at 125%, with the taskbar: the work area is 1366 by 708 pixels
		let s = fit_scale(1.25, 1366, 708);
		assert!(s < 1.25);
		let (w, h) = size(s);
		assert!(w <= 1366 && h <= 708, "{w} by {h}");
		// where it fits, the display's own scale stays
		assert_eq!(fit_scale(1.25, 1920, 1032), 1.25);
		assert_eq!(fit_scale(2.0, 3840, 2088), 2.0);
		for (scale, ww, wh) in [(1.0, 1024, 560), (1.5, 1280, 680), (2.0, 1100, 900), (1.75, 900, 1500)] {
			let (w, h) = size(fit_scale(scale, ww, wh));
			assert!(w as i32 <= ww && h as i32 <= wh, "{scale}: {w} by {h} on {ww} by {wh}");
		}
	}

	fn view(n: usize, ring: &[&str]) -> WheelView {
		WheelView {
			slices: (0..n)
				.map(|i| Slice {
					label: format!("S{i}"),
					icon: "crop".into(),
					desc: String::new(),
					ring: ring.iter().map(|r| r.to_string()).collect(),
				})
				.collect(),
			page: 0,
			hover: Hover::None,
			keyboard: false,
			title: String::new(),
			subtitle: String::new(),
			keys: Vec::new(),
			message: None,
			hint: None,
			motion: Motion::default(),
		}
	}

	fn fonts() -> Fonts {
		Fonts::fallback()
	}

	#[test]
	fn hit_testing_follows_the_slices() {
		let f = fonts();
		let v = view(4, &[]);
		// slice 0 is straight up, slices go clockwise
		assert_eq!(hit(&v, &f, CX, CY - 120.0), Hover::Slice(0));
		assert_eq!(hit(&v, &f, CX + 120.0, CY), Hover::Slice(1));
		assert_eq!(hit(&v, &f, CX, CY + 120.0), Hover::Slice(2));
		assert_eq!(hit(&v, &f, CX - 120.0, CY), Hover::Slice(3));
		assert_eq!(hit(&v, &f, CX, CY), Hover::Center);
		assert_eq!(hit(&v, &f, 2.0, 2.0), Hover::None);
	}

	#[test]
	fn the_ring_opens_around_its_slice() {
		let f = fonts();
		let mut v = view(4, &["Small", "Big"]);
		v.hover = Hover::Slice(0);
		let spans = ring_layout(&v, &f, 0);
		assert_eq!(spans.len(), 2);
		let r = (RING_IN + RING_OUT) / 2.0;
		let a = (spans[1].0 + spans[1].1) / 2.0;
		assert_eq!(hit(&v, &f, CX + a.cos() * r, CY + a.sin() * r), Hover::Ring(0, 1));
		// without a hovered slice the ring isn't there
		v.hover = Hover::None;
		assert_eq!(hit(&v, &f, CX + a.cos() * r, CY + a.sin() * r), Hover::None);
	}

	#[test]
	fn the_way_out_to_a_ring_choice_never_drops_it() {
		let f = fonts();
		// a five-choice ring fans well past its own slice in an eight-slice wheel
		let mut v = view(8, &["Balanced", "Smaller", "Best", "Custom", "Other"]);
		v.hover = Hover::Slice(3);
		let spans = ring_layout(&v, &f, 3);
		let (_, mid, _) = angles(8, 3);
		let step = TAU / 8.0;
		assert!(
			spans[0].0 < mid - step / 2.0 && spans[4].1 > mid + step / 2.0,
			"the ring is wider than its slice"
		);
		// walk straight out to every chip, through the neighbours' rims, the gap and the chip
		for (j, (a0, a1)) in spans.iter().enumerate() {
			let a = (a0 + a1) / 2.0;
			for r in [
				R_OUT - 20.0,
				R_OUT - 4.0,
				R_OUT + 3.0,
				RING_IN,
				(RING_IN + RING_OUT) / 2.0,
				RING_OUT + 20.0,
			] {
				let h = hit(&v, &f, CX + a.cos() * r, CY + a.sin() * r);
				assert!(
					matches!(h, Hover::Slice(3) | Hover::Ring(3, _)),
					"chip {j} at {r}: {h:?}"
				);
				v.hover = h;
			}
			assert_eq!(v.hover, Hover::Ring(3, j));
			v.hover = Hover::Slice(3);
		}
		// between two chips the nearer one is picked
		let between = (spans[1].1 + spans[2].0) / 2.0 + 0.001;
		let r = (RING_IN + RING_OUT) / 2.0;
		assert_eq!(
			hit(&v, &f, CX + between.cos() * r, CY + between.sin() * r),
			Hover::Ring(3, 2)
		);
		// the inner part of a neighbour still switches to it
		let (_, next, _) = angles(8, 4);
		assert_eq!(
			hit(&v, &f, CX + next.cos() * 100.0, CY + next.sin() * 100.0),
			Hover::Slice(4)
		);
	}

	#[test]
	fn long_lists_page_with_more() {
		assert_eq!(pages(8), 1);
		assert_eq!(pages(9), 2);
		assert_eq!(pages(14), 2);
		assert_eq!(pages(15), 3);
		let (items, more) = page_items(12, 1);
		assert_eq!(items, vec![7, 8, 9, 10, 11]);
		assert!(more);
		let f = fonts();
		let v = view(12, &[]);
		let (_, mid, _) = angles(8, 7);
		assert_eq!(hit(&v, &f, CX + mid.cos() * 120.0, CY + mid.sin() * 120.0), Hover::More);
	}

	#[test]
	fn renders_every_state() {
		let f = fonts();
		let theme = Theme {
			dark: true,
			win11: true,
			accent: Rgba::hex(0x0078d4),
			shade: None,
			high_contrast: false,
		};
		let mut v = view(10, &["One", "Two", "Three"]);
		for hover in [
			Hover::None,
			Hover::Center,
			Hover::Slice(2),
			Hover::Ring(2, 1),
			Hover::More,
		] {
			v.hover = hover;
			v.motion.hover = vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
			let pm = render(&v, &theme, &f, 1.5, None);
			assert_eq!(pm.width(), (WIDTH * 1.5).ceil() as u32);
		}
		v.motion = Motion {
			open: 0.3,
			..Motion::default()
		};
		render(&v, &theme, &f, 1.0, None);
		v.motion = Motion {
			close: 0.5,
			picked: Some(1),
			..Motion::default()
		};
		render(&v, &theme, &f, 1.0, None);
	}
}
