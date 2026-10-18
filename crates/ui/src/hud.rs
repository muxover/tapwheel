use tiny_skia::Pixmap;

use crate::draw::*;
use crate::text::{Fonts, Weight};
use crate::theme::Theme;

pub const CARD_W: f32 = 372.0;
/// A card with a progress bar or a row of buttons.
pub const FULL_H: f32 = 88.0;
/// A card that only says something, with at most a close button.
pub const COMPACT_H: f32 = 60.0;
pub const GAP: f32 = 8.0;
pub const MARGIN: f32 = 16.0;
pub const MAX_CARDS: usize = 4;
const SLIDE: f32 = 28.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
	Running,
	Done,
	Failed,
	Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
	Open,
	Show,
	Undo,
	Cancel,
	Close,
}

impl Button {
	pub fn label(self) -> &'static str {
		match self {
			Button::Open => "Open",
			Button::Show => "Show in folder",
			Button::Undo => "Undo",
			Button::Cancel => "Cancel",
			Button::Close => "Close",
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Card {
	pub title: String,
	pub status: String,
	pub state: State,
	/// `None` while running means the job can't report a percentage.
	pub progress: Option<f32>,
	pub buttons: Vec<Button>,
	/// 0 to 1 while a card slides in or folds away; 1 when it's settled.
	pub shown: f32,
}

impl Card {
	fn has_row(&self) -> bool {
		self.state == State::Running || self.buttons.iter().any(|b| *b != Button::Close)
	}

	pub fn height(&self) -> f32 {
		if self.has_row() { FULL_H } else { COMPACT_H }
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct HudView {
	pub cards: Vec<Card>,
	pub hover: Option<(usize, Button)>,
	/// Seconds, drives the moving bar of jobs without a percentage.
	pub time: f32,
}

fn ease(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	1.0 - (1.0 - t).powi(3)
}

/// Top of each card in logical pixels. A card that's folding away gives up its space gradually.
fn tops(cards: &[Card]) -> Vec<f32> {
	let mut y = MARGIN;
	cards
		.iter()
		.take(MAX_CARDS)
		.map(|c| {
			let top = y;
			y += (c.height() + GAP) * ease(c.shown);
			top
		})
		.collect()
}

pub fn size(cards: &[Card], scale: f32) -> (u32, u32) {
	let used: f32 = cards
		.iter()
		.take(MAX_CARDS)
		.map(|c| (c.height() + GAP) * ease(c.shown))
		.sum();
	let h = (used - GAP).max(COMPACT_H * 0.5) + MARGIN * 2.0;
	(
		((CARD_W + MARGIN * 2.0) * scale).ceil() as u32,
		(h * scale).ceil() as u32,
	)
}

const BTN_H: f32 = 28.0;

/// Button rectangles in logical pixels, for drawing and hit testing.
fn buttons_at(card: &Card, top: f32, fonts: &Fonts) -> Vec<(Button, f32, f32, f32, f32)> {
	let y = top + card.height() - BTN_H - 12.0;
	let mut x = MARGIN + CARD_W - 12.0;
	let mut out = Vec::new();
	for b in card.buttons.iter().rev() {
		if *b == Button::Close {
			continue;
		}
		let w = fonts.width(b.label(), 12.0, Weight::Regular) + 24.0;
		x -= w;
		out.push((*b, x, y, w, BTN_H));
		x -= 8.0;
	}
	if card.buttons.contains(&Button::Close) {
		let cy = if card.has_row() {
			top + 8.0
		} else {
			top + (card.height() - 28.0) / 2.0
		};
		out.push((Button::Close, MARGIN + CARD_W - 40.0, cy, 28.0, 28.0));
	}
	out
}

pub fn hit(view: &HudView, fonts: &Fonts, x: f32, y: f32) -> Option<(usize, Button)> {
	for (i, (card, top)) in view.cards.iter().zip(tops(&view.cards)).enumerate() {
		if card.shown < 1.0 {
			continue;
		}
		for (b, bx, by, bw, bh) in buttons_at(card, top, fonts) {
			if x >= bx && x < bx + bw && y >= by && y < by + bh {
				return Some((i, b));
			}
		}
	}
	None
}

pub fn render(view: &HudView, theme: &Theme, fonts: &Fonts, scale: f32) -> Pixmap {
	let (w, h) = size(&view.cards, scale);
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	let s = scale;
	for (i, (card, top)) in view.cards.iter().zip(tops(&view.cards)).enumerate() {
		if card.shown <= 0.0 {
			continue;
		}
		let one = render_card(view, i, card, theme, fonts, s);
		let e = ease(card.shown);
		let paint = tiny_skia::PixmapPaint {
			opacity: e,
			..Default::default()
		};
		let dx = ((1.0 - e) * SLIDE * s).round() as i32;
		pm.draw_pixmap(
			dx,
			((top - MARGIN) * s).round() as i32,
			one.as_ref(),
			&paint,
			tiny_skia::Transform::identity(),
			None,
		);
	}
	pm
}

/// One card on its own canvas with room for its shadow; the card itself starts at (MARGIN, MARGIN).
fn render_card(view: &HudView, i: usize, card: &Card, theme: &Theme, fonts: &Fonts, s: f32) -> Pixmap {
	let ch_l = card.height();
	let mut pm = Pixmap::new(
		((CARD_W + MARGIN * 2.0) * s).ceil() as u32,
		((ch_l + MARGIN * 2.0) * s).ceil() as u32,
	)
	.expect("non-zero size");
	let p = theme.palette();
	let r = theme.radius() * s;
	let (x, y) = (MARGIN * s, MARGIN * s);
	let (cw, ch) = (CARD_W * s, ch_l * s);
	if !theme.high_contrast {
		shadow_rrect(&mut pm, x, y, cw, ch, r, 10.0 * s, p.shadow);
	}
	fill_rrect(&mut pm, x, y, cw, ch, r, p.surface);
	stroke_rrect(&mut pm, x, y, cw, ch, r, s.max(1.0), p.stroke.alpha(1.8));

	let compact = !card.has_row();
	let (mx, my, mr) = (
		x + 26.0 * s,
		if compact { y + ch / 2.0 } else { y + 28.0 * s },
		10.0 * s,
	);
	match card.state {
		State::Running => {
			stroke_circle(&mut pm, mx, my, mr - s, 2.0 * s, p.text2.alpha(0.35));
			let start = view.time * 5.0;
			if let Some(seg) = ring_segment(mx, my, mr - 2.0 * s, mr, start, start + 1.8) {
				fill_path(&mut pm, &seg, p.accent);
			}
		}
		State::Done => {
			fill_circle(&mut pm, mx, my, mr, p.accent);
			check(&mut pm, mx - mr * 0.62, my - mr * 0.62, mr * 1.24, 2.0 * s, p.on_accent);
		}
		State::Failed => {
			fill_circle(&mut pm, mx, my, mr, p.danger);
			cross(&mut pm, mx, my, mr * 0.7, 2.0 * s, p.surface);
		}
		State::Cancelled => {
			stroke_circle(&mut pm, mx, my, mr - s, 2.0 * s, p.text2);
			line(&mut pm, mx - mr * 0.45, my, mx + mr * 0.45, my, 2.0 * s, p.text2);
		}
	}

	let tx = x + 48.0 * s;
	let text_max = cw - 48.0 * s - 48.0 * s;
	let (ty, sy) = if compact {
		(y + 10.0 * s, y + 31.0 * s)
	} else {
		(y + 12.0 * s, y + 32.0 * s)
	};
	fonts.draw(
		&mut pm,
		&card.title,
		tx,
		ty,
		14.0 * s,
		Weight::Semibold,
		p.text,
		Some(text_max),
	);
	let status_color = if card.state == State::Failed { p.danger } else { p.text2 };
	fonts.draw(
		&mut pm,
		&card.status,
		tx,
		sy,
		12.0 * s,
		Weight::Regular,
		status_color,
		Some(text_max),
	);

	if card.state == State::Running {
		let (bx, by, bw, bh) = (tx, y + ch - 22.0 * s, cw - 48.0 * s - 16.0 * s - 90.0 * s, 4.0 * s);
		bar(&mut pm, bx, by, bw, bh, p.text2.alpha(0.2));
		match card.progress {
			Some(f) => bar(&mut pm, bx, by, bw * f.clamp(0.0, 1.0), bh, p.accent),
			None => {
				let seg = bw * 0.3;
				let t = (view.time * 0.8).fract();
				let start = bx - seg + (bw + seg) * t;
				let a = start.max(bx);
				let b = (start + seg).min(bx + bw);
				if b > a {
					bar(&mut pm, a, by, b - a, bh, p.accent);
				}
			}
		}
	}

	// buttons are laid out for a card whose top is MARGIN, which is where this canvas puts it
	for (b, bx, by, bw, bh) in buttons_at(card, MARGIN, fonts) {
		let hovered = view.hover == Some((i, b));
		let (bx, by, bw, bh) = (bx * s, by * s, bw * s, bh * s);
		if b == Button::Close {
			if hovered {
				fill_rrect(&mut pm, bx, by, bw, bh, 4.0 * s, p.text.alpha(0.08));
			}
			cross(&mut pm, bx + bw / 2.0, by + bh / 2.0, 8.0 * s, 1.4 * s, p.text2);
			continue;
		}
		let primary = b == Button::Open;
		let br = if theme.win11 { 4.0 * s } else { 0.0 };
		if primary {
			let fill = if hovered {
				p.accent.mix(p.surface, 0.12)
			} else {
				p.accent
			};
			fill_rrect(&mut pm, bx, by, bw, bh, br, fill);
		} else {
			let fill = if hovered { p.raised.mix(p.text, 0.06) } else { p.raised };
			fill_rrect(&mut pm, bx, by, bw, bh, br, fill);
			stroke_rrect(&mut pm, bx, by, bw, bh, br, s.max(1.0), p.stroke.alpha(1.6));
		}
		let ty = by + (bh - fonts.line_height(12.0 * s, Weight::Regular)) / 2.0;
		let color = if primary { p.on_accent } else { p.text };
		fonts.draw_centered(
			&mut pm,
			b.label(),
			bx + bw / 2.0,
			ty,
			12.0 * s,
			Weight::Regular,
			color,
			None,
		);
	}
	pm
}

pub fn human_size(bytes: u64) -> String {
	const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
	if bytes < 1024 {
		return format!("{bytes} bytes");
	}
	let mut v = bytes as f64 / 1024.0;
	let mut u = 0;
	while v >= 1024.0 && u < UNITS.len() - 1 {
		v /= 1024.0;
		u += 1;
	}
	if v >= 100.0 {
		format!("{v:.0} {}", UNITS[u])
	} else {
		format!("{v:.1} {}", UNITS[u])
	}
}

/// "4.2 MB to 680 KB, 84% smaller" or just the new size.
pub fn size_change(before: u64, after: u64) -> String {
	if before == 0 {
		return human_size(after);
	}
	let pct = (after as f64 / before as f64 - 1.0) * 100.0;
	let tail = if pct <= -1.0 {
		format!(", {:.0}% smaller", -pct)
	} else if pct >= 1.0 {
		format!(", {:.0}% larger", pct)
	} else {
		String::new()
	};
	format!("{} to {}{}", human_size(before), human_size(after), tail)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sizes_read_naturally() {
		assert_eq!(human_size(900), "900 bytes");
		assert_eq!(human_size(1536), "1.5 KB");
		assert_eq!(human_size(4_400_000), "4.2 MB");
		assert_eq!(size_change(4_400_000, 696_320), "4.2 MB to 680 KB, 84% smaller");
		assert_eq!(size_change(1000, 1004), "1000 bytes to 1004 bytes");
	}

	#[test]
	fn cards_without_buttons_are_compact() {
		let card = |state, buttons: Vec<Button>| Card {
			title: "a".into(),
			status: "b".into(),
			state,
			progress: None,
			buttons,
			shown: 1.0,
		};
		assert_eq!(card(State::Done, vec![Button::Close]).height(), COMPACT_H);
		assert_eq!(card(State::Failed, vec![Button::Close]).height(), COMPACT_H);
		assert_eq!(card(State::Done, vec![Button::Undo, Button::Close]).height(), FULL_H);
		assert_eq!(card(State::Running, vec![Button::Cancel]).height(), FULL_H);
		let cards = vec![
			card(State::Done, vec![Button::Close]),
			card(State::Running, vec![Button::Cancel]),
		];
		assert_eq!(tops(&cards), vec![MARGIN, MARGIN + COMPACT_H + GAP]);
		let (_, h) = size(&cards, 1.0);
		assert_eq!(h as f32, (COMPACT_H + GAP + FULL_H + MARGIN * 2.0).ceil());
	}

	#[test]
	fn a_folding_card_gives_up_its_space() {
		let mut cards = vec![
			Card {
				title: "a".into(),
				status: "b".into(),
				state: State::Done,
				progress: None,
				buttons: vec![Button::Close],
				shown: 0.0,
			},
			Card {
				title: "c".into(),
				status: "d".into(),
				state: State::Done,
				progress: None,
				buttons: vec![Button::Close],
				shown: 1.0,
			},
		];
		assert_eq!(tops(&cards)[1], MARGIN);
		cards[0].shown = 1.0;
		assert_eq!(tops(&cards)[1], MARGIN + COMPACT_H + GAP);
	}
}
