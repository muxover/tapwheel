use std::f32::consts::{PI, TAU};

use tiny_skia::Pixmap;

use crate::draw::*;
use crate::theme::Rgba;

pub const INK: Rgba = Rgba::hex(0x15171b);
pub const SEGMENT: Rgba = Rgba::hex(0x3d424b);
/// Windows blue, as the dark theme shows it.
pub const BLUE: Rgba = Rgba::hex(0x60cdff);

/// The light theme's disc and the slices on it.
const PAPER: Rgba = Rgba::hex(0xf9f9f9);
const SEGMENT_LIGHT: Rgba = Rgba::hex(0xc9ccd2);
const DOT: Rgba = Rgba::hex(0xf4f3ef);

/// The TapWheel mark: a dark disc, a ring of slices with one lit, and the tap in the middle.
pub fn logo(size: u32) -> Pixmap {
	themed(size, BLUE, true)
}

/// The lit slice's colour: the accent, pushed lighter on the dark disc or darker on the light one
/// until it stands out from the disc at 3 to 1, so any accent Windows or Settings picks reads.
pub fn lit(accent: Rgba, dark: bool) -> Rgba {
	let disc = if dark { INK } else { PAPER };
	let toward = if dark { Rgba::hex(0xffffff) } else { Rgba::hex(0x000000) };
	let mut c = accent;
	let mut t = 0.0;
	while c.contrast(disc) < 3.0 && t < 1.0 {
		t += 0.05;
		c = accent.mix(toward, t);
	}
	c
}

/// The mark in the Windows theme: the lit slice takes the accent, and the disc is dark or light.
pub fn themed(size: u32, accent: Rgba, dark: bool) -> Pixmap {
	let (disc, segment, dot) = if dark {
		(INK, SEGMENT, DOT)
	} else {
		(PAPER, SEGMENT_LIGHT, INK)
	};
	let lit = lit(accent, dark);
	let mut pm = Pixmap::new(size, size).expect("non-zero size");
	let s = size as f32;
	let (cx, cy) = (s / 2.0, s / 2.0);
	let small = size <= 24;
	fill_circle(&mut pm, cx, cy, s * 0.5, disc);
	if !dark {
		// a hairline keeps the light disc apart from a light taskbar
		stroke_circle(&mut pm, cx, cy, s * 0.5 - 0.5, 1.0, SEGMENT_LIGHT);
	}
	let n = if small { 4 } else { 6 };
	let gap = if small { 0.22 } else { 0.12 };
	let (r_in, r_out) = if small {
		(s * 0.2, s * 0.42)
	} else {
		(s * 0.22, s * 0.4)
	};
	for i in 0..n {
		let step = TAU / n as f32;
		let mid = -PI / 2.0 + step * i as f32 + step / 2.0;
		let (a0, a1) = (mid - step / 2.0 + gap / 2.0, mid + step / 2.0 - gap / 2.0);
		let color = if i == 0 { lit } else { segment };
		if let Some(seg) = ring_segment(cx, cy, r_in, r_out, a0, a1) {
			fill_path(&mut pm, &seg, color);
		}
	}
	fill_circle(&mut pm, cx, cy, if small { s * 0.11 } else { s * 0.1 }, dot);
	pm
}

/// Pixels as Windows wants them for an icon: blue, green, red, alpha, premultiplied, top row first.
pub fn bgra(pm: &Pixmap) -> Vec<u8> {
	pm.data()
		.chunks_exact(4)
		.flat_map(|p| [p[2], p[1], p[0], p[3]])
		.collect()
}

/// A Windows .ico with PNG images at the usual sizes.
pub fn ico(sizes: &[u32]) -> Vec<u8> {
	let images: Vec<Vec<u8>> = sizes.iter().map(|s| logo(*s).encode_png().expect("png")).collect();
	let mut out = Vec::new();
	out.extend_from_slice(&[0, 0, 1, 0]);
	out.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
	let mut offset = 6 + 16 * sizes.len() as u32;
	for (size, png) in sizes.iter().zip(&images) {
		let dim = if *size >= 256 { 0 } else { *size as u8 };
		out.extend_from_slice(&[dim, dim, 0, 0]);
		out.extend_from_slice(&1u16.to_le_bytes());
		out.extend_from_slice(&32u16.to_le_bytes());
		out.extend_from_slice(&(png.len() as u32).to_le_bytes());
		out.extend_from_slice(&offset.to_le_bytes());
		offset += png.len() as u32;
	}
	for png in images {
		out.extend_from_slice(&png);
	}
	out
}

/// The logo centred on a transparent canvas, for Store tiles that want padding.
pub fn padded(w: u32, h: u32, logo_size: u32) -> Pixmap {
	let mut pm = Pixmap::new(w, h).expect("non-zero size");
	let l = logo(logo_size);
	pm.draw_pixmap(
		((w - logo_size) / 2) as i32,
		((h - logo_size) / 2) as i32,
		l.as_ref(),
		&tiny_skia::PixmapPaint::default(),
		tiny_skia::Transform::identity(),
		None,
	);
	pm
}

#[cfg(test)]
mod tests {

	#[test]
	fn every_accent_lights_its_slice_on_both_discs() {
		// Windows' own blue, a dark navy, a pale yellow, a mid grey, red, and pure black and white
		for accent in [0x0078d4, 0x001f4d, 0xfff4b0, 0x808080, 0xe81123, 0x000000, 0xffffff] {
			for dark in [true, false] {
				let disc = if dark { INK } else { PAPER };
				let c = lit(Rgba::hex(accent), dark);
				assert!(
					c.contrast(disc) >= 3.0,
					"#{accent:06x}, dark {dark}: {:.2}",
					c.contrast(disc)
				);
				assert_eq!(themed(32, Rgba::hex(accent), dark).width(), 32);
			}
		}
		// an accent that already stands out is used as it is
		assert_eq!(lit(BLUE, true), BLUE);
	}

	#[test]
	fn icon_pixels_come_out_in_windows_order() {
		let pm = themed(16, Rgba::hex(0xff0000), true);
		let px = bgra(&pm);
		assert_eq!(px.len(), 16 * 16 * 4);
		let rgba = pm.data();
		assert_eq!((px[0], px[2]), (rgba[2], rgba[0]));
	}

	use super::*;

	#[test]
	fn ico_header_lists_every_size() {
		let data = ico(&[16, 32, 256]);
		assert_eq!(&data[..6], &[0, 0, 1, 0, 3, 0]);
		assert_eq!(data[6], 16);
		assert_eq!(data[6 + 32], 0, "256 is written as 0");
		let first = u32::from_le_bytes(data[18..22].try_into().unwrap()) as usize;
		assert_eq!(&data[first..first + 4], b"\x89PNG");
	}
}
