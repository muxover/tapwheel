use tiny_skia::Pixmap;

use crate::draw::*;
use crate::text::{Fonts, Weight};
use crate::theme::Theme;

pub const CARD_W: f32 = 372.0;
pub const CARD_H: f32 = 88.0;
pub const GAP: f32 = 8.0;
pub const MARGIN: f32 = 16.0;
pub const MAX_CARDS: usize = 4;

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
}

#[derive(Clone, Debug, PartialEq)]
pub struct HudView {
	pub cards: Vec<Card>,
	pub hover: Option<(usize, Button)>,
	/// Seconds, drives the moving bar of jobs without a percentage.
	pub time: f32,
}

pub fn size(n: usize, scale: f32) -> (u32, u32) {
	let n = n.clamp(1, MAX_CARDS) as f32;
	(
		((CARD_W + MARGIN * 2.0) * scale).ceil() as u32,
		((CARD_H * n + GAP * (n - 1.0) + MARGIN * 2.0) * scale).ceil() as u32,
	)
}

fn card_top(i: usize) -> f32 {
	MARGIN + i as f32 * (CARD_H + GAP)
}

const BTN_H: f32 = 28.0;

/// Button rectangles in logical pixels, for drawing and hit testing.
pub fn buttons(card: &Card, i: usize, fonts: &Fonts) -> Vec<(Button, f32, f32, f32, f32)> {
	let y = card_top(i) + CARD_H - BTN_H - 12.0;
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
		out.push((Button::Close, MARGIN + CARD_W - 36.0, card_top(i) + 8.0, 28.0, 28.0));
	}
	out
}

pub fn hit(view: &HudView, fonts: &Fonts, x: f32, y: f32) -> Option<(usize, Button)> {
	for (i, card) in view.cards.iter().enumerate().take(MAX_CARDS) {
		for (b, bx, by, bw, bh) in buttons(card, i, fonts) {
			if x >= bx && x < bx + bw && y >= by && y < by + bh {
				return Some((i, b));
			}
		}
	}
	None
}

/// Whether a point is on a card at all (the gaps let clicks through).
pub fn on_card(n: usize, x: f32, y: f32) -> Option<usize> {
	(0..n.min(MAX_CARDS)).find(|i| {
		let top = card_top(*i);
		(MARGIN..MARGIN + CARD_W).contains(&x) && (top..top + CARD_H).contains(&y)
	})
}

pub fn render(view: &HudView, theme: &Theme, fonts: &Fonts, scale: f32) -> Pixmap {
	let (w, h) = size(view.cards.len(), scale);
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	let p = theme.palette();
	let s = scale;
	let r = theme.radius() * s;
	for (i, card) in view.cards.iter().enumerate().take(MAX_CARDS) {
		let (x, y) = (MARGIN * s, card_top(i) * s);
		let (cw, ch) = (CARD_W * s, CARD_H * s);
		if !theme.high_contrast {
			shadow_rrect(&mut pm, x, y, cw, ch, r, 10.0 * s, p.shadow);
		}
		fill_rrect(&mut pm, x, y, cw, ch, r, p.surface);
		stroke_rrect(&mut pm, x, y, cw, ch, r, s.max(1.0), p.stroke.alpha(1.8));

		// status mark
		let (mx, my, mr) = (x + 26.0 * s, y + 28.0 * s, 10.0 * s);
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
		let text_max = cw - 48.0 * s - 44.0 * s;
		fonts.draw(
			&mut pm,
			&card.title,
			tx,
			y + 12.0 * s,
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
			y + 32.0 * s,
			12.0 * s,
			Weight::Regular,
			status_color,
			Some(cw - 60.0 * s),
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

		for (b, bx, by, bw, bh) in buttons(card, i, fonts) {
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
				fill_rrect(
					&mut pm,
					bx,
					by,
					bw,
					bh,
					br,
					if hovered {
						p.accent.mix(p.surface, 0.12)
					} else {
						p.accent
					},
				);
			} else {
				fill_rrect(
					&mut pm,
					bx,
					by,
					bw,
					bh,
					br,
					if hovered { p.raised.mix(p.text, 0.06) } else { p.raised },
				);
				stroke_rrect(&mut pm, bx, by, bw, bh, br, s.max(1.0), p.stroke.alpha(1.6));
			}
			let ty = by + (bh - fonts.line_height(12.0 * s, Weight::Regular)) / 2.0;
			fonts.draw_centered(
				&mut pm,
				b.label(),
				bx + bw / 2.0,
				ty,
				12.0 * s,
				Weight::Regular,
				if primary { p.on_accent } else { p.text },
				None,
			);
		}
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
	fn gaps_between_cards_let_clicks_through() {
		assert_eq!(on_card(2, MARGIN + 10.0, MARGIN + 10.0), Some(0));
		assert_eq!(on_card(2, MARGIN + 10.0, MARGIN + CARD_H + GAP / 2.0), None);
		assert_eq!(on_card(2, MARGIN + 10.0, MARGIN + CARD_H + GAP + 1.0), Some(1));
	}
}
