//! A picture of code: highlighted, in a window, on a backdrop. Drawn without a browser so the
//! Convert wheel can make one instantly; the Code shot window offers the same look with live controls.

use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use serde::{Deserialize, Serialize};
use tiny_skia::{
	Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, Point, Rect, SpreadMode, Transform,
};

use crate::Error;

const FONT: &[u8] = include_bytes!("../assets/JetBrainsMono-Regular.ttf");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Style {
	pub dark: bool,
	/// A backdrop name from [`backdrop`], a `#rrggbb` colour, or `none` for transparent.
	pub background: String,
	pub padding: f32,
	pub line_numbers: bool,
	pub frame: bool,
	pub title: String,
	pub font_size: f32,
	/// Pixels per point; 2 gives a sharp picture on any screen.
	pub scale: f32,
}

impl Default for Style {
	fn default() -> Self {
		Style {
			dark: true,
			background: "sunset".into(),
			padding: 56.0,
			line_numbers: true,
			frame: true,
			title: String::new(),
			font_size: 14.0,
			scale: 2.0,
		}
	}
}

/// Gradient stops for the named backdrops, shared with the editor windows.
pub fn backdrop(name: &str) -> Option<Vec<(f32, [u8; 3])>> {
	Some(match name {
		"sunset" => vec![
			(0.0, [0xff, 0x6b, 0x6b]),
			(0.5, [0xff, 0xa9, 0x4d]),
			(1.0, [0x84, 0x5e, 0xf7]),
		],
		"ocean" => vec![(0.0, [0x22, 0xd3, 0xee]), (1.0, [0x63, 0x66, 0xf1])],
		"lime" => vec![(0.0, [0xb6, 0xf2, 0x3a]), (1.0, [0x10, 0xb9, 0x81])],
		"grape" => vec![(0.0, [0xa8, 0x55, 0xf7]), (1.0, [0xec, 0x48, 0x99])],
		"dusk" => vec![(0.0, [0x1e, 0x29, 0x3b]), (1.0, [0x4c, 0x1d, 0x95])],
		"sand" => vec![(0.0, [0xfd, 0xe6, 0x8a]), (1.0, [0xfb, 0x92, 0x3c])],
		"mist" => vec![(0.0, [0xe2, 0xe8, 0xf0]), (1.0, [0xcb, 0xd5, 0xe1])],
		"ink" => vec![(0.0, [0x18, 0x18, 0x1b]), (1.0, [0x27, 0x27, 0x2a])],
		_ => return None,
	})
}

fn fill_backdrop(pm: &mut Pixmap, name: &str) {
	let (w, h) = (pm.width() as f32, pm.height() as f32);
	if let Some(stops) = backdrop(name) {
		let stops = stops
			.into_iter()
			.map(|(p, c)| GradientStop::new(p, Color::from_rgba8(c[0], c[1], c[2], 255)))
			.collect();
		if let Some(shader) = LinearGradient::new(
			Point::from_xy(0.0, 0.0),
			Point::from_xy(w, h),
			stops,
			SpreadMode::Pad,
			Transform::identity(),
		) {
			let paint = Paint {
				shader,
				..Paint::default()
			};
			pm.fill_rect(
				Rect::from_xywh(0.0, 0.0, w, h).expect("non-empty"),
				&paint,
				Transform::identity(),
				None,
			);
		}
	} else if let Some(c) = crate::image::parse_colour(name) {
		pm.fill(c);
	}
}

fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
	let mut pb = PathBuilder::new();
	pb.move_to(x + r, y);
	pb.line_to(x + w - r, y);
	pb.quad_to(x + w, y, x + w, y + r);
	pb.line_to(x + w, y + h - r);
	pb.quad_to(x + w, y + h, x + w - r, y + h);
	pb.line_to(x + r, y + h);
	pb.quad_to(x, y + h, x, y + h - r);
	pb.line_to(x, y + r);
	pb.quad_to(x, y, x + r, y);
	pb.close();
	pb.finish()
}

fn solid(r: u8, g: u8, b: u8, a: u8) -> Paint<'static> {
	let mut p = Paint::default();
	p.set_color_rgba8(r, g, b, a);
	p.anti_alias = true;
	p
}

fn draw_text(pm: &mut Pixmap, font: &FontRef, size: f32, x: f32, baseline: f32, text: &str, color: [u8; 4]) -> f32 {
	let scaled = font.as_scaled(PxScale::from(size));
	let mut pen = x;
	let (w, h) = (pm.width() as i32, pm.height() as i32);
	let data = pm.data_mut();
	for ch in text.chars() {
		let id = scaled.glyph_id(ch);
		let glyph = id.with_scale_and_position(PxScale::from(size), ab_glyph::point(pen, baseline));
		pen += scaled.h_advance(id);
		let Some(outline) = font.outline_glyph(glyph) else {
			continue;
		};
		let bounds = outline.px_bounds();
		outline.draw(|gx, gy, cov| {
			let px = bounds.min.x as i32 + gx as i32;
			let py = bounds.min.y as i32 + gy as i32;
			if px < 0 || py < 0 || px >= w || py >= h {
				return;
			}
			let i = ((py * w + px) * 4) as usize;
			let a = cov.clamp(0.0, 1.0) * color[3] as f32 / 255.0;
			// premultiplied "source over"
			for c in 0..3 {
				let src = color[c] as f32 * a;
				data[i + c] = (src + data[i + c] as f32 * (1.0 - a)).round() as u8;
			}
			data[i + 3] = (a * 255.0 + data[i + 3] as f32 * (1.0 - a)).round() as u8;
		});
	}
	pen
}

/// Draws the code. Long files are cut at 400 lines and lines at 160 characters.
pub fn render(code: &str, ext: &str, style: &Style) -> Result<Pixmap, Error> {
	let font = FontRef::try_from_slice(FONT).map_err(|_| Error::Engine("the code font couldn't be loaded".into()))?;
	let s = style.scale.clamp(1.0, 4.0);
	let size = style.font_size.clamp(8.0, 40.0) * s;
	let scaled = font.as_scaled(PxScale::from(size));
	let advance = scaled.h_advance(scaled.glyph_id('M'));
	let line_h = (size * 1.55).round();
	let code = code.replace('\t', "    ").replace("\r\n", "\n");
	let code = code.trim_end_matches('\n');
	let lines: Vec<&str> = code.lines().take(400).collect();
	let spans = crate::code::highlight(&lines.join("\n"), ext, style.dark);
	let cols = lines
		.iter()
		.map(|l| l.chars().count().min(160))
		.max()
		.unwrap_or(1)
		.max(20);
	let gutter = if style.line_numbers {
		(lines.len().to_string().len() as f32 + 2.0) * advance
	} else {
		0.0
	};
	let inner_pad = 20.0 * s;
	let bar = if style.frame { 40.0 * s } else { 0.0 };
	let win_w = gutter + cols as f32 * advance + inner_pad * 2.0;
	let win_h = bar + lines.len() as f32 * line_h + inner_pad * 2.0;
	let pad = if style.background == "none" {
		0.0
	} else {
		style.padding * s
	};
	let (w, h) = ((win_w + pad * 2.0).ceil() as u32, (win_h + pad * 2.0).ceil() as u32);
	let mut pm = Pixmap::new(w, h).ok_or_else(|| Error::Engine("the code is too long for one picture".into()))?;
	fill_backdrop(&mut pm, &style.background);

	let (wx, wy) = (pad, pad);
	let radius = 12.0 * s;
	// soft shadow from a few widening translucent layers
	if pad > 0.0 {
		for i in 0..12 {
			let spread = i as f32 * 2.2 * s;
			if let Some(p) = rounded(
				wx - spread / 2.0,
				wy - spread / 2.0 + 10.0 * s,
				win_w + spread,
				win_h + spread,
				radius + spread,
			) {
				pm.fill_path(&p, &solid(0, 0, 0, 7), FillRule::Winding, Transform::identity(), None);
			}
		}
	}
	let (bg, fg_dim, rule) = if style.dark {
		([0x28, 0x2c, 0x34], [0x63, 0x6d, 0x83, 255], [0x3b, 0x40, 0x48])
	} else {
		([0xfa, 0xfa, 0xfa], [0xa0, 0xa1, 0xa7, 255], [0xe5, 0xe5, 0xe6])
	};
	if let Some(p) = rounded(wx, wy, win_w, win_h, radius) {
		pm.fill_path(
			&p,
			&solid(bg[0], bg[1], bg[2], 255),
			FillRule::Winding,
			Transform::identity(),
			None,
		);
	}
	if style.frame {
		let cy = wy + bar / 2.0;
		let title = if style.title.is_empty() {
			""
		} else {
			style.title.as_str()
		};
		draw_text(&mut pm, &font, 12.5 * s, wx + inner_pad, cy + 4.5 * s, title, fg_dim);
		// Windows caption glyphs: minimise, maximise, close
		let gx = wx + win_w - inner_pad - 10.0 * s;
		let pen = solid(fg_dim[0], fg_dim[1], fg_dim[2], 255);
		let stroke = tiny_skia::Stroke {
			width: 1.2 * s,
			..Default::default()
		};
		let mut pb = PathBuilder::new();
		pb.move_to(gx - 5.0 * s, cy - 5.0 * s);
		pb.line_to(gx + 5.0 * s, cy + 5.0 * s);
		pb.move_to(gx + 5.0 * s, cy - 5.0 * s);
		pb.line_to(gx - 5.0 * s, cy + 5.0 * s);
		let mx = gx - 36.0 * s;
		pb.push_rect(Rect::from_xywh(mx - 5.0 * s, cy - 5.0 * s, 10.0 * s, 10.0 * s).expect("size"));
		let nx = mx - 36.0 * s;
		pb.move_to(nx - 5.0 * s, cy);
		pb.line_to(nx + 5.0 * s, cy);
		if let Some(p) = pb.finish() {
			pm.stroke_path(&p, &pen, &stroke, Transform::identity(), None);
		}
		let line = Rect::from_xywh(wx, wy + bar, win_w, s.max(1.0));
		if let Some(r) = line {
			pm.fill_rect(r, &solid(rule[0], rule[1], rule[2], 255), Transform::identity(), None);
		}
	}
	let ascent = scaled.ascent();
	for (i, line) in spans.iter().enumerate() {
		let top = wy + bar + inner_pad + i as f32 * line_h;
		let baseline = top + (line_h - size) / 2.0 + ascent;
		if style.line_numbers {
			let n = (i + 1).to_string();
			let nx = wx + inner_pad + gutter - (n.len() as f32 + 1.5) * advance;
			draw_text(&mut pm, &font, size, nx, baseline, &n, fg_dim);
		}
		let mut x = wx + inner_pad + gutter;
		let mut used = 0;
		for (st, text) in line {
			let room = 160usize.saturating_sub(used);
			let piece: String = text.chars().take(room).collect();
			used += piece.chars().count();
			let c = st.foreground;
			x = draw_text(&mut pm, &font, size, x, baseline, &piece, [c.r, c.g, c.b, 255]);
		}
	}
	Ok(pm)
}

pub fn render_png(code: &str, ext: &str, style: &Style, out: &std::path::Path) -> Result<(), Error> {
	let pm = render(code, ext, style)?;
	crate::image::write_png(out, pm.width(), pm.height(), &crate::image::unpremultiply(pm.data()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn code_picture_has_the_window_and_backdrop() {
		let pm = render("fn main() {\n\tprintln!(\"hi\");\n}\n", "rs", &Style::default()).unwrap();
		assert!(pm.width() > 400 && pm.height() > 200);
		// a corner is the backdrop, the middle is the dark window
		let corner = pm.pixel(2, 2).unwrap();
		assert!(corner.red() > 200);
		let mid = pm.pixel(pm.width() / 2, pm.height() - 2 * 56 - 30).unwrap();
		assert!(mid.red() < 0x40, "{mid:?}");
	}
}
