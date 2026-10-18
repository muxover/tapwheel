use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::draw::*;
use crate::text::{Fonts, Weight};
use crate::theme::{Palette, Theme};
use crate::wheel::{self, Hover, KeyHint, Slice, WheelView};
use crate::wrap;

pub const WIDTH: f32 = 880.0;
pub const HEIGHT: f32 = 640.0;
pub const KEY_CHOICES: [&str; 4] = ["Shift", "Ctrl", "Alt", "Off"];
pub const WHEELS: [(&str, &str); 3] = [
	("Convert", "Change the format"),
	("Tools", "Compress, resize, unpack"),
	("Actions", "Copy, terminal, folder"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
	Key(usize, usize),
	StartWithWindows,
	LearnOrder,
	Paused,
	Samples,
	Website,
	Privacy,
	Source,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HomeView {
	pub version: String,
	/// Chosen index into `KEY_CHOICES` for Convert, Tools and Actions.
	pub keys: [usize; 3],
	pub start_with_windows: bool,
	pub learn_order: bool,
	pub paused: bool,
	pub hover: Option<Action>,
	pub focus: Option<Action>,
}

const PAD: f32 = 32.0;
const TOP: f32 = 96.0;
const COL_GAP: f32 = 24.0;
const LEFT_W: f32 = 384.0;
const ROW_H: f32 = 64.0;
const SECTION_H: f32 = 30.0;
const SEG_W: f32 = 54.0;
const SEG_H: f32 = 32.0;
const FOOT_Y: f32 = HEIGHT - 52.0;

fn right_x() -> f32 {
	PAD + LEFT_W + COL_GAP
}

fn right_w() -> f32 {
	WIDTH - right_x() - PAD
}

fn keys_top() -> f32 {
	TOP + SECTION_H
}

fn general_top() -> f32 {
	keys_top() + ROW_H * 3.0 + 20.0 + SECTION_H
}

const LINKS: [(Action, &str); 3] = [
	(Action::Website, "Website"),
	(Action::Privacy, "Privacy"),
	(Action::Source, "Source code"),
];

/// Every clickable thing and where it is, in logical pixels: (action, x, y, w, h).
pub fn layout(fonts: &Fonts) -> Vec<(Action, f32, f32, f32, f32)> {
	let mut out = Vec::new();
	let rx = right_x();
	let rw = right_w();
	for w in 0..3 {
		let y = keys_top() + ROW_H * w as f32 + (ROW_H - SEG_H) / 2.0;
		let x0 = rx + rw - 16.0 - SEG_W * 4.0;
		for c in 0..4 {
			out.push((Action::Key(w, c), x0 + SEG_W * c as f32, y, SEG_W, SEG_H));
		}
	}
	for (i, a) in [Action::StartWithWindows, Action::LearnOrder, Action::Paused]
		.into_iter()
		.enumerate()
	{
		let y = general_top() + ROW_H * i as f32;
		out.push((a, rx, y, rw, ROW_H));
	}
	let (bx, by, bw, bh) = samples_button();
	out.push((Action::Samples, bx, by, bw, bh));
	let mut x = PAD;
	for (a, label) in LINKS {
		let w = fonts.width(label, 13.0, Weight::Regular);
		out.push((a, x - 4.0, FOOT_Y + 6.0, w + 8.0, 28.0));
		x += w + 24.0;
	}
	out
}

fn samples_button() -> (f32, f32, f32, f32) {
	let card_bottom = TOP + SECTION_H + left_card_h();
	(PAD + 24.0, card_bottom - 24.0 - 36.0, LEFT_W - 48.0, 36.0)
}

fn left_card_h() -> f32 {
	general_top() + ROW_H * 3.0 - (TOP + SECTION_H)
}

pub fn hit(fonts: &Fonts, x: f32, y: f32) -> Option<Action> {
	layout(fonts)
		.into_iter()
		.find(|(_, ax, ay, aw, ah)| x >= *ax && x < ax + aw && y >= *ay && y < ay + ah)
		.map(|(a, ..)| a)
}

/// Keyboard order for Tab.
pub fn tab_order(fonts: &Fonts) -> Vec<Action> {
	let all = layout(fonts);
	let mut order = vec![Action::Samples];
	for w in 0..3 {
		order.push(Action::Key(w, 0));
	}
	order.extend([Action::StartWithWindows, Action::LearnOrder, Action::Paused]);
	order.extend(LINKS.iter().map(|(a, _)| *a));
	order.retain(|a| all.iter().any(|(b, ..)| b == a));
	order
}

pub fn render(v: &HomeView, theme: &Theme, fonts: &Fonts, scale: f32, logo: &Pixmap) -> Pixmap {
	let s = scale;
	let w = (WIDTH * s).ceil() as u32;
	let h = (HEIGHT * s).ceil() as u32;
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	let p = theme.palette();
	let base = if theme.high_contrast {
		p.surface
	} else if theme.dark {
		crate::theme::Rgba::hex(0x202020)
	} else {
		crate::theme::Rgba::hex(0xf3f3f3)
	};
	pm.fill(base.skia());
	let r = theme.radius() * s;

	// header
	let logo_size = 40.0 * s;
	let ls = logo_size / logo.width() as f32;
	pm.draw_pixmap(
		0,
		0,
		logo.as_ref(),
		&PixmapPaint {
			quality: tiny_skia::FilterQuality::Bicubic,
			..Default::default()
		},
		Transform::from_translate(PAD * s, 32.0 * s).pre_scale(ls, ls),
		None,
	);
	fonts.draw(
		&mut pm,
		"TapWheel",
		(PAD + 52.0) * s,
		30.0 * s,
		26.0 * s,
		Weight::Semibold,
		p.text,
		None,
	);
	let status = if v.paused { "Paused" } else { "Ready" };
	let sub = format!("{status} · Version {}", v.version);
	fonts.draw(
		&mut pm,
		&sub,
		(PAD + 54.0) * s,
		64.0 * s,
		12.0 * s,
		Weight::Regular,
		p.text2,
		None,
	);

	// left: get started
	let lx = PAD * s;
	fonts.draw(
		&mut pm,
		"Get started",
		lx,
		TOP * s,
		15.0 * s,
		Weight::Semibold,
		p.text,
		None,
	);
	let (cx, cy, cw, ch) = (lx, (TOP + SECTION_H) * s, LEFT_W * s, left_card_h() * s);
	card(&mut pm, cx, cy, cw, ch, r, &p, theme);

	let demo = WheelView {
		slices: ["PNG", "JPG", "WebP", "AVIF", "GIF", "TIFF", "BMP", "ICO"]
			.iter()
			.map(|l| Slice {
				label: l.to_string(),
				desc: format!("Convert to {l}"),
			})
			.collect(),
		hover: Hover::Slice(0),
		keyboard: false,
		title: "Convert".into(),
		subtitle: "photo.heic".into(),
		keys: Vec::new(),
		message: None,
		appear: 1.0,
	};
	let ws = 0.62 * s;
	let wheel_pm = wheel::render(&demo, theme, fonts, ws);
	let wx = cx + (cw - wheel_pm.width() as f32) / 2.0;
	pm.draw_pixmap(
		wx as i32,
		(cy + 4.0 * s) as i32,
		wheel_pm.as_ref(),
		&PixmapPaint::default(),
		Transform::identity(),
		None,
	);

	let steps = [
		"Start dragging any file or folder.",
		"Tap Shift, Ctrl or Alt. The wheel opens under your pointer.",
		"Drop on what you want. The new file lands next to the old one.",
	];
	let mut sy = cy + 234.0 * s;
	for (i, step) in steps.iter().enumerate() {
		let nx = cx + 36.0 * s;
		fill_circle(&mut pm, nx, sy + 10.0 * s, 11.0 * s, p.raised.mix(p.text, 0.1));
		let num = (i + 1).to_string();
		fonts.draw_centered(
			&mut pm,
			&num,
			nx,
			sy + 10.0 * s - fonts.line_height(12.0 * s, Weight::Semibold) / 2.0,
			12.0 * s,
			Weight::Semibold,
			p.text,
			None,
		);
		let lines = wrap(fonts, step, 13.0 * s, Weight::Regular, cw - 84.0 * s, 2);
		let mut ty = sy;
		for l in &lines {
			fonts.draw(&mut pm, l, cx + 58.0 * s, ty, 13.0 * s, Weight::Regular, p.text, None);
			ty += fonts.line_height(13.0 * s, Weight::Regular);
		}
		sy += 44.0 * s;
	}
	let (bx, by, bw, bh) = samples_button();
	button(
		&mut pm,
		fonts,
		theme,
		&p,
		"Try it on sample files",
		(bx, by, bw, bh),
		s,
		true,
		v.hover == Some(Action::Samples),
		v.focus == Some(Action::Samples),
	);

	// right: keys
	let rx = right_x() * s;
	let rw = right_w() * s;
	fonts.draw(&mut pm, "Keys", rx, TOP * s, 15.0 * s, Weight::Semibold, p.text, None);
	card(&mut pm, rx, keys_top() * s, rw, ROW_H * 3.0 * s, r, &p, theme);
	for (wi, (name, desc)) in WHEELS.iter().enumerate() {
		let y = (keys_top() + ROW_H * wi as f32) * s;
		if wi > 0 {
			line(
				&mut pm,
				rx + 16.0 * s,
				y,
				rx + rw - 16.0 * s,
				y,
				s.max(1.0),
				p.stroke.alpha(1.2),
			);
		}
		fonts.draw(
			&mut pm,
			name,
			rx + 16.0 * s,
			y + 13.0 * s,
			14.0 * s,
			Weight::Semibold,
			p.text,
			None,
		);
		fonts.draw(
			&mut pm,
			desc,
			rx + 16.0 * s,
			y + 34.0 * s,
			12.0 * s,
			Weight::Regular,
			p.text2,
			Some(rw - SEG_W * 4.0 * s - 48.0 * s),
		);
	}
	for (a, x, y, w, h) in layout(fonts) {
		if let Action::Key(wi, c) = a {
			let selected = v.keys[wi] == c;
			let (x, y, w, h) = (x * s, y * s, w * s, h * s);
			let hovered = v.hover == Some(a);
			let rr = if theme.win11 { 4.0 * s } else { 0.0 };
			if selected {
				fill_rrect(&mut pm, x + s, y, w - 2.0 * s, h, rr, p.accent);
			} else {
				fill_rrect(
					&mut pm,
					x + s,
					y,
					w - 2.0 * s,
					h,
					rr,
					if hovered { p.raised.mix(p.text, 0.08) } else { p.raised },
				);
				stroke_rrect(&mut pm, x + s, y, w - 2.0 * s, h, rr, s.max(1.0), p.stroke.alpha(1.6));
			}
			let label = KEY_CHOICES[c];
			let weight = if selected { Weight::Semibold } else { Weight::Regular };
			let ty = y + (h - fonts.line_height(12.0 * s, weight)) / 2.0;
			fonts.draw_centered(
				&mut pm,
				label,
				x + w / 2.0,
				ty,
				12.0 * s,
				weight,
				if selected { p.on_accent } else { p.text },
				None,
			);
			if v.focus.is_some_and(|f| matches!(f, Action::Key(fw, _) if fw == wi)) && selected {
				stroke_rrect(
					&mut pm,
					x - s,
					y - 2.0 * s,
					w + 2.0 * s,
					h + 4.0 * s,
					rr + 2.0 * s,
					2.0 * s,
					p.text,
				);
			}
		}
	}

	// right: general
	let gy = general_top() * s;
	fonts.draw(
		&mut pm,
		"General",
		rx,
		gy - SECTION_H * s,
		15.0 * s,
		Weight::Semibold,
		p.text,
		None,
	);
	card(&mut pm, rx, gy, rw, ROW_H * 3.0 * s, r, &p, theme);
	let rows = [
		(
			Action::StartWithWindows,
			"Start with Windows",
			"Ready as soon as you sign in",
			v.start_with_windows,
		),
		(
			Action::LearnOrder,
			"Favorites first",
			"Slices you pick most move to the top",
			v.learn_order,
		),
		(
			Action::Paused,
			"Pause TapWheel",
			"Key taps do nothing until you turn this off",
			v.paused,
		),
	];
	for (i, (a, title, desc, on)) in rows.iter().enumerate() {
		let y = gy + ROW_H * i as f32 * s;
		if i > 0 {
			line(
				&mut pm,
				rx + 16.0 * s,
				y,
				rx + rw - 16.0 * s,
				y,
				s.max(1.0),
				p.stroke.alpha(1.2),
			);
		}
		if v.hover == Some(*a) {
			fill_rrect(
				&mut pm,
				rx + 4.0 * s,
				y + 4.0 * s,
				rw - 8.0 * s,
				ROW_H * s - 8.0 * s,
				4.0 * s,
				p.text.alpha(0.04),
			);
		}
		fonts.draw(
			&mut pm,
			title,
			rx + 16.0 * s,
			y + 13.0 * s,
			14.0 * s,
			Weight::Regular,
			p.text,
			None,
		);
		fonts.draw(
			&mut pm,
			desc,
			rx + 16.0 * s,
			y + 34.0 * s,
			12.0 * s,
			Weight::Regular,
			p.text2,
			Some(rw - 100.0 * s),
		);
		toggle(
			&mut pm,
			theme,
			&p,
			rx + rw - 16.0 * s - 40.0 * s,
			y + (ROW_H - 20.0) / 2.0 * s,
			s,
			*on,
		);
		if v.focus == Some(*a) {
			stroke_rrect(
				&mut pm,
				rx + 2.0 * s,
				y + 2.0 * s,
				rw - 4.0 * s,
				ROW_H * s - 4.0 * s,
				4.0 * s,
				2.0 * s,
				p.text,
			);
		}
	}

	// footer
	let fy = FOOT_Y * s;
	for (a, x, y, w, h) in layout(fonts) {
		let Some(label) = LINKS.iter().find(|(b, _)| *b == a).map(|(_, l)| *l) else {
			continue;
		};
		let (x, y, w, h) = (x * s, y * s, w * s, h * s);
		let color = if v.hover == Some(a) { p.accent } else { p.text };
		fonts.draw(
			&mut pm,
			label,
			x + 4.0 * s,
			y + 5.0 * s,
			13.0 * s,
			Weight::Regular,
			color,
			None,
		);
		if v.hover == Some(a) {
			line(
				&mut pm,
				x + 4.0 * s,
				y + 23.0 * s,
				x + w - 4.0 * s,
				y + 23.0 * s,
				s.max(1.0),
				color,
			);
		}
		if v.focus == Some(a) {
			stroke_rrect(&mut pm, x, y, w, h, 4.0 * s, 2.0 * s, p.text);
		}
	}
	let credit = "Inspired by Tangerine for Mac. Free and open source.";
	let cw2 = fonts.width(credit, 12.0 * s, Weight::Regular);
	fonts.draw(
		&mut pm,
		credit,
		(WIDTH - PAD) * s - cw2,
		fy + 12.0 * s,
		12.0 * s,
		Weight::Regular,
		p.text2,
		None,
	);
	pm
}

fn card(pm: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, r: f32, p: &Palette, theme: &Theme) {
	let fill = if theme.dark || theme.high_contrast {
		p.surface
	} else {
		p.raised
	};
	fill_rrect(pm, x, y, w, h, r, fill);
	stroke_rrect(pm, x, y, w, h, r, 1.0f32.max(r / 8.0).min(1.5), p.stroke.alpha(1.4));
}

fn toggle(pm: &mut Pixmap, theme: &Theme, p: &Palette, x: f32, y: f32, s: f32, on: bool) {
	let (w, h) = (40.0 * s, 20.0 * s);
	let r = if theme.win11 { h / 2.0 } else { 0.0 };
	if on {
		fill_rrect(pm, x, y, w, h, r, p.accent);
		let k = 12.0 * s;
		let kx = x + w - k - 4.0 * s;
		if theme.win11 {
			fill_circle(pm, kx + k / 2.0, y + h / 2.0, k / 2.0, p.on_accent);
		} else {
			fill_rrect(pm, kx, y + 4.0 * s, k, k, 0.0, p.on_accent);
		}
	} else {
		stroke_rrect(pm, x, y, w, h, r, s.max(1.0), p.text2);
		let k = 10.0 * s;
		let kx = x + 5.0 * s;
		if theme.win11 {
			fill_circle(pm, kx + k / 2.0, y + h / 2.0, k / 2.0, p.text2);
		} else {
			fill_rrect(pm, kx, y + 5.0 * s, k, k, 0.0, p.text2);
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn button(
	pm: &mut Pixmap,
	fonts: &Fonts,
	theme: &Theme,
	p: &Palette,
	label: &str,
	rect: (f32, f32, f32, f32),
	s: f32,
	primary: bool,
	hovered: bool,
	focused: bool,
) {
	let (x, y, w, h) = (rect.0 * s, rect.1 * s, rect.2 * s, rect.3 * s);
	let r = if theme.win11 { 4.0 * s } else { 0.0 };
	if primary {
		fill_rrect(
			pm,
			x,
			y,
			w,
			h,
			r,
			if hovered {
				p.accent.mix(p.surface, 0.12)
			} else {
				p.accent
			},
		);
	} else {
		fill_rrect(
			pm,
			x,
			y,
			w,
			h,
			r,
			if hovered { p.raised.mix(p.text, 0.06) } else { p.raised },
		);
		stroke_rrect(pm, x, y, w, h, r, s.max(1.0), p.stroke.alpha(1.6));
	}
	let ty = y + (h - fonts.line_height(14.0 * s, Weight::Semibold)) / 2.0;
	fonts.draw_centered(
		pm,
		label,
		x + w / 2.0,
		ty,
		14.0 * s,
		Weight::Semibold,
		if primary { p.on_accent } else { p.text },
		None,
	);
	if focused {
		stroke_rrect(
			pm,
			x - 3.0 * s,
			y - 3.0 * s,
			w + 6.0 * s,
			h + 6.0 * s,
			r + 3.0 * s,
			2.0 * s,
			p.text,
		);
	}
}

/// Keys used by a wheel, for the pill under the real wheel.
pub fn key_hints(keys: [usize; 3], active: usize) -> Vec<KeyHint> {
	(0..3)
		.filter(|w| KEY_CHOICES[keys[*w]] != "Off")
		.map(|w| KeyHint {
			key: KEY_CHOICES[keys[w]].to_string(),
			name: WHEELS[w].0.to_string(),
			active: w == active,
		})
		.collect()
}
