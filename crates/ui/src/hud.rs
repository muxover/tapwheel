//! Progress and result cards, stacked above the taskbar. Every card is the same compact height;
//! its buttons slide out only while the pointer is on it, so no card carries an empty row.

use tiny_skia::Pixmap;

use crate::draw::*;
use crate::text::{Fonts, Weight};
use crate::theme::Theme;

pub const CARD_W: f32 = 360.0;
pub const CARD_H: f32 = 64.0;
/// Extra height when a card shows its buttons.
pub const ROW_H: f32 = 42.0;
pub const GAP: f32 = 8.0;
pub const MARGIN: f32 = 14.0;
pub const MAX_CARDS: usize = 4;
const SLIDE: f32 = 36.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
	/// Queued behind other jobs, so the PC isn't swamped by several encodes at once.
	Waiting,
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
	Copy,
	Retry,
	Cancel,
	Close,
}

impl Button {
	pub fn label(self) -> &'static str {
		match self {
			Button::Open => "Open",
			Button::Show => "Show in folder",
			Button::Undo => "Undo",
			Button::Copy => "Copy",
			Button::Retry => "Retry",
			Button::Cancel => "Cancel",
			Button::Close => "Close",
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Card {
	pub title: String,
	pub detail: String,
	pub state: State,
	/// `None` while running means the job can't report a percentage.
	pub progress: Option<f32>,
	/// Buttons revealed on hover. Cancel and Close are drawn as the corner button instead.
	pub buttons: Vec<Button>,
	/// Fluent icon for the tool; the state's own mark is used when empty.
	pub icon: String,
	/// 0 to 1 while a card slides in or folds away; 1 when it's settled.
	pub shown: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HudView {
	pub cards: Vec<Card>,
	pub hover: Option<(usize, Button)>,
	/// Seconds, drives the moving bar of jobs without a percentage.
	pub time: f32,
}

/// How far each card has opened its button row, 0 to 1. Kept outside the cards so the app can ease it.
pub type Expand = Vec<f32>;

fn row_buttons(card: &Card) -> Vec<Button> {
	card.buttons
		.iter()
		.copied()
		.filter(|b| !matches!(b, Button::Cancel | Button::Close))
		.collect()
}

fn corner_button(card: &Card) -> Button {
	if card.state == State::Running {
		Button::Cancel
	} else {
		Button::Close
	}
}

pub fn card_height(card: &Card, expand: f32) -> f32 {
	if row_buttons(card).is_empty() {
		CARD_H
	} else {
		CARD_H + ROW_H * ease_out(expand)
	}
}

/// Top of each card in logical pixels. A card that's folding away gives up its space gradually.
fn tops(cards: &[Card], expand: &[f32]) -> Vec<f32> {
	let mut y = MARGIN;
	cards
		.iter()
		.take(MAX_CARDS)
		.enumerate()
		.map(|(i, c)| {
			let top = y;
			y += (card_height(c, expand.get(i).copied().unwrap_or(0.0)) + GAP) * ease_out(c.shown);
			top
		})
		.collect()
}

pub fn size(cards: &[Card], expand: &[f32], scale: f32) -> (u32, u32) {
	let used: f32 = cards
		.iter()
		.take(MAX_CARDS)
		.enumerate()
		.map(|(i, c)| (card_height(c, expand.get(i).copied().unwrap_or(0.0)) + GAP) * ease_out(c.shown))
		.sum();
	let h = (used - GAP).max(CARD_H * 0.5) + MARGIN * 2.0;
	(
		((CARD_W + MARGIN * 2.0) * scale).ceil() as u32,
		(h * scale).ceil() as u32,
	)
}

const BTN_H: f32 = 30.0;

/// Button rectangles in logical pixels relative to the card's top-left.
fn buttons_at(card: &Card, expand: f32, fonts: &Fonts) -> Vec<(Button, f32, f32, f32, f32)> {
	let mut out = vec![(corner_button(card), CARD_W - 40.0, (CARD_H - 28.0) / 2.0, 28.0, 28.0)];
	if expand > 0.5 {
		let y = CARD_H + (ROW_H - BTN_H) / 2.0 - 6.0;
		let mut x = 56.0;
		for b in row_buttons(card) {
			let w = fonts.width(b.label(), 12.0, Weight::Regular) + 26.0;
			out.push((b, x, y, w, BTN_H));
			x += w + 8.0;
		}
	}
	out
}

pub fn hit(view: &HudView, expand: &[f32], fonts: &Fonts, x: f32, y: f32) -> Option<(usize, Option<Button>)> {
	for (i, (card, top)) in view.cards.iter().zip(tops(&view.cards, expand)).enumerate() {
		if card.shown < 1.0 {
			continue;
		}
		let e = expand.get(i).copied().unwrap_or(0.0);
		let (lx, ly) = (x - MARGIN, y - top);
		if !(0.0..=CARD_W).contains(&lx) || ly < 0.0 || ly > card_height(card, e) {
			continue;
		}
		for (b, bx, by, bw, bh) in buttons_at(card, e, fonts) {
			if lx >= bx && lx < bx + bw && ly >= by && ly < by + bh {
				return Some((i, Some(b)));
			}
		}
		return Some((i, None));
	}
	None
}

pub fn render(view: &HudView, expand: &[f32], theme: &Theme, fonts: &Fonts, scale: f32) -> Pixmap {
	let (w, h) = size(&view.cards, expand, scale);
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	let s = scale;
	for (i, (card, top)) in view.cards.iter().zip(tops(&view.cards, expand)).enumerate() {
		if card.shown <= 0.0 {
			continue;
		}
		let e = expand.get(i).copied().unwrap_or(0.0);
		let one = render_card(view, i, card, e, theme, fonts, s);
		let t = ease_out(card.shown);
		let paint = tiny_skia::PixmapPaint {
			opacity: t,
			quality: tiny_skia::FilterQuality::Bilinear,
			..Default::default()
		};
		// slides in from the right and settles with a slight scale
		let k = 0.96 + 0.04 * t;
		let (cw, ch) = (CARD_W * s, card_height(card, e) * s);
		let (ox, oy) = (MARGIN * s + cw / 2.0, (top + card_height(card, e) / 2.0) * s);
		let dx = (1.0 - t) * SLIDE * s;
		let tf = tiny_skia::Transform::from_translate(ox + dx, oy)
			.pre_scale(k, k)
			.pre_translate(-(MARGIN * s + cw / 2.0), -(MARGIN * s + ch / 2.0));
		pm.draw_pixmap(0, 0, one.as_ref(), &paint, tf, None);
	}
	pm
}

/// One card on its own canvas with room for its shadow; the card itself starts at (MARGIN, MARGIN).
fn render_card(view: &HudView, i: usize, card: &Card, expand: f32, theme: &Theme, fonts: &Fonts, s: f32) -> Pixmap {
	let ch_l = card_height(card, expand);
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
		shadow_rrect(&mut pm, x, y, cw, ch, r, 12.0 * s, p.shadow);
	}
	fill_rrect(&mut pm, x, y, cw, ch, r, p.surface);
	stroke_rrect(&mut pm, x, y, cw, ch, r, s.max(1.0), p.stroke.alpha(1.8));

	// the tool's tile, with the state shown on it
	let (tx, ty, ts) = (x + 14.0 * s, y + (CARD_H * s - 36.0 * s) / 2.0, 36.0 * s);
	let tile_r = if theme.win11 { 8.0 * s } else { 0.0 };
	let (tile_bg, tile_fg) = match card.state {
		State::Failed => (p.danger.alpha(0.16), p.danger),
		State::Done => (p.accent.alpha(0.16), p.accent),
		_ => (p.text.alpha(0.07), p.text),
	};
	fill_rrect(&mut pm, tx, ty, ts, ts, tile_r, tile_bg);
	let mark = match card.state {
		State::Failed => "error_circle",
		State::Cancelled => "dismiss_circle",
		_ if !card.icon.is_empty() => card.icon.as_str(),
		_ => "checkmark_circle",
	};
	if card.state == State::Running && card.progress.is_none() {
		// no percentage: a spinner takes the icon's place
		let (mx, my, mr) = (tx + ts / 2.0, ty + ts / 2.0, 9.0 * s);
		stroke_circle(&mut pm, mx, my, mr - s, 2.0 * s, p.text2.alpha(0.25));
		let start = view.time * 5.0;
		if let Some(seg) = ring_segment(mx, my, mr - 2.0 * s, mr, start, start + 1.8) {
			fill_path(&mut pm, &seg, p.accent);
		}
	} else {
		icon(&mut pm, mark, tx + 8.0 * s, ty + 8.0 * s, 20.0 * s, tile_fg);
	}
	if card.state == State::Done {
		// a small check badge on the tile's corner
		let (bx, by) = (tx + ts - 4.0 * s, ty + ts - 4.0 * s);
		fill_circle(&mut pm, bx, by, 8.0 * s, p.surface);
		fill_circle(&mut pm, bx, by, 6.5 * s, p.accent);
		check(&mut pm, bx - 4.5 * s, by - 4.5 * s, 9.0 * s, 1.6 * s, p.on_accent);
	}

	let text_x = x + 62.0 * s;
	let text_max = cw - 62.0 * s - 48.0 * s;
	let title_size = 13.5 * s;
	let detail_size = 12.0 * s;
	let block =
		fonts.line_height(title_size, Weight::Semibold) + 2.0 * s + fonts.line_height(detail_size, Weight::Regular);
	let top = y + (CARD_H * s - block) / 2.0;
	fonts.draw(
		&mut pm,
		&card.title,
		text_x,
		top,
		title_size,
		Weight::Semibold,
		p.text,
		Some(text_max),
	);
	let mut detail = card.detail.clone();
	if card.state == State::Running
		&& let Some(f) = card.progress
	{
		detail = format!("{}%  ·  {detail}", (f * 100.0).round() as u32);
	}
	let detail_color = if card.state == State::Failed { p.danger } else { p.text2 };
	fonts.draw(
		&mut pm,
		&detail,
		text_x,
		top + fonts.line_height(title_size, Weight::Semibold) + 2.0 * s,
		detail_size,
		Weight::Regular,
		detail_color,
		Some(text_max),
	);

	// progress runs along the card's bottom edge
	if card.state == State::Running {
		let (bx, by, bw, bh) = (x + 14.0 * s, y + ch - 7.0 * s, cw - 28.0 * s, 3.0 * s);
		bar(&mut pm, bx, by, bw, bh, p.text2.alpha(0.18));
		match card.progress {
			Some(f) => bar(&mut pm, bx, by, bw * f.clamp(0.0, 1.0), bh, p.accent),
			None => {
				let seg = bw * 0.3;
				let t = (view.time * 0.8).fract();
				let start = bx - seg + (bw + seg) * t;
				let (a, b) = (start.max(bx), (start + seg).min(bx + bw));
				if b > a {
					bar(&mut pm, a, by, b - a, bh, p.accent);
				}
			}
		}
	}

	let card_hovered = view.hover.is_some_and(|(j, _)| j == i);
	for (b, bx, by, bw, bh) in buttons_at(card, expand, fonts) {
		let hovered = view.hover == Some((i, b));
		let (bx, by, bw, bh) = (x + bx * s, y + by * s, bw * s, bh * s);
		if matches!(b, Button::Close | Button::Cancel) {
			// the corner button stays quiet until the card is pointed at, but Cancel is always there
			if !card_hovered && b == Button::Close {
				continue;
			}
			if hovered {
				fill_rrect(&mut pm, bx, by, bw, bh, 4.0 * s, p.text.alpha(0.08));
			}
			icon(&mut pm, "dismiss", bx + 6.0 * s, by + 6.0 * s, 16.0 * s, p.text2);
			continue;
		}
		let a = ((expand - 0.5) * 2.0).clamp(0.0, 1.0);
		let primary = b == Button::Open;
		let br = if theme.win11 { 4.0 * s } else { 0.0 };
		if primary {
			let fill = if hovered {
				p.accent.mix(p.surface, 0.12)
			} else {
				p.accent
			};
			fill_rrect(&mut pm, bx, by, bw, bh, br, fill.alpha(a));
		} else {
			let fill = if hovered { p.raised.mix(p.text, 0.06) } else { p.raised };
			fill_rrect(&mut pm, bx, by, bw, bh, br, fill.alpha(a));
			stroke_rrect(&mut pm, bx, by, bw, bh, br, s.max(1.0), p.stroke.alpha(1.6 * a));
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
			color.alpha(a),
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

	fn card(state: State, buttons: Vec<Button>, shown: f32) -> Card {
		Card {
			title: "a".into(),
			detail: "b".into(),
			state,
			progress: None,
			buttons,
			icon: String::new(),
			shown,
		}
	}

	#[test]
	fn sizes_read_naturally() {
		assert_eq!(human_size(900), "900 bytes");
		assert_eq!(human_size(1536), "1.5 KB");
		assert_eq!(human_size(4_400_000), "4.2 MB");
		assert_eq!(size_change(4_400_000, 696_320), "4.2 MB to 680 KB, 84% smaller");
		assert_eq!(size_change(1000, 1004), "1000 bytes to 1004 bytes");
	}

	#[test]
	fn cards_stay_compact_until_pointed_at() {
		let c = card(State::Done, vec![Button::Open, Button::Undo], 1.0);
		assert_eq!(card_height(&c, 0.0), CARD_H);
		assert_eq!(card_height(&c, 1.0), CARD_H + ROW_H);
		let plain = card(State::Done, vec![Button::Close], 1.0);
		assert_eq!(card_height(&plain, 1.0), CARD_H, "nothing to reveal, nothing grows");
		let cards = vec![plain.clone(), card(State::Running, vec![Button::Cancel], 1.0)];
		assert_eq!(tops(&cards, &[0.0, 0.0]), vec![MARGIN, MARGIN + CARD_H + GAP]);
	}

	#[test]
	fn a_folding_card_gives_up_its_space() {
		let mut cards = vec![card(State::Done, vec![], 0.0), card(State::Done, vec![], 1.0)];
		assert_eq!(tops(&cards, &[])[1], MARGIN);
		cards[0].shown = 1.0;
		assert_eq!(tops(&cards, &[])[1], MARGIN + CARD_H + GAP);
	}

	#[test]
	fn buttons_hit_only_once_revealed() {
		let f = Fonts::fallback();
		let view = HudView {
			cards: vec![card(State::Done, vec![Button::Open], 1.0)],
			hover: None,
			time: 0.0,
		};
		let below = MARGIN + CARD_H + 14.0;
		assert_eq!(hit(&view, &[0.0], &f, MARGIN + 70.0, below), None);
		assert_eq!(
			hit(&view, &[1.0], &f, MARGIN + 70.0, below),
			Some((0, Some(Button::Open)))
		);
		assert_eq!(hit(&view, &[0.0], &f, MARGIN + 100.0, MARGIN + 20.0), Some((0, None)));
	}
}
