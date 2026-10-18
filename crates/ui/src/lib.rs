#![allow(clippy::too_many_arguments)]

pub mod draw;
pub mod home;
pub mod hud;
pub mod icon;
pub mod text;
pub mod theme;
pub mod wheel;

pub use text::{Fonts, Weight};
pub use theme::{Rgba, Theme};

/// Word-wraps into at most `max_lines`; the last line keeps the rest and fades if it's too long.
pub fn wrap(fonts: &Fonts, text: &str, size: f32, w: Weight, width: f32, max_lines: usize) -> Vec<String> {
	let mut lines: Vec<String> = Vec::new();
	let mut cur = String::new();
	for word in text.split_whitespace() {
		let trial = if cur.is_empty() {
			word.to_string()
		} else {
			format!("{cur} {word}")
		};
		if fonts.width(&trial, size, w) <= width || cur.is_empty() || lines.len() + 1 == max_lines {
			cur = trial;
		} else {
			lines.push(std::mem::take(&mut cur));
			cur = word.to_string();
		}
	}
	if !cur.is_empty() {
		lines.push(cur);
	}
	// a lone short last word reads badly; pull one word down to keep it company
	if lines.len() >= 2 {
		let last = lines.len() - 1;
		let words_last = lines[last].split_whitespace().count();
		let words_prev = lines[last - 1].split_whitespace().count();
		if words_last == 1 && words_prev > 2 {
			let prev = lines[last - 1].clone();
			if let Some((head, tail)) = prev.rsplit_once(' ') {
				let joined = format!("{tail} {}", lines[last]);
				if fonts.width(&joined, size, w) <= width {
					lines[last - 1] = head.to_string();
					lines[last] = joined;
				}
			}
		}
	}
	lines
}
