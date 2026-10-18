use std::f32::consts::{PI, TAU};

use tiny_skia::Pixmap;

use crate::draw::*;
use crate::theme::Rgba;

pub const INK: Rgba = Rgba::hex(0x15171b);
pub const SEGMENT: Rgba = Rgba::hex(0x3d424b);
/// Windows blue, as the dark theme shows it.
pub const BLUE: Rgba = Rgba::hex(0x60cdff);

/// The TapWheel mark: a dark disc, a ring of slices with one lit, and the tap in the middle.
pub fn logo(size: u32) -> Pixmap {
	let mut pm = Pixmap::new(size, size).expect("non-zero size");
	let s = size as f32;
	let (cx, cy) = (s / 2.0, s / 2.0);
	let small = size <= 24;
	fill_circle(&mut pm, cx, cy, s * 0.5, INK);
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
		let color = if i == 0 { BLUE } else { SEGMENT };
		if let Some(seg) = ring_segment(cx, cy, r_in, r_out, a0, a1) {
			fill_path(&mut pm, &seg, color);
		}
	}
	fill_circle(
		&mut pm,
		cx,
		cy,
		if small { s * 0.11 } else { s * 0.1 },
		Rgba::hex(0xf4f3ef),
	);
	pm
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
