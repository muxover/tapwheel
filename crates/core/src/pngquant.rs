use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

use crate::Error;

/// Lossy PNG compression: 256-colour palette with dithering, like pngquant.
pub fn compress(input: &Path, output: &Path) -> Result<(), Error> {
	let mut decoder = png::Decoder::new(BufReader::new(File::open(input)?));
	decoder.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
	let mut reader = decoder
		.read_info()
		.map_err(|e| Error::Engine(format!("can't read PNG: {e}")))?;
	let size = reader
		.output_buffer_size()
		.ok_or_else(|| Error::Engine("PNG is too large".into()))?;
	let mut buf = vec![0; size];
	let info = reader
		.next_frame(&mut buf)
		.map_err(|e| Error::Engine(format!("can't read PNG: {e}")))?;
	let (w, h) = (info.width as usize, info.height as usize);

	let pixels: Vec<imagequant::RGBA> = match info.color_type {
		png::ColorType::Rgba => buf[..w * h * 4]
			.as_chunks::<4>()
			.0
			.iter()
			.map(|p| imagequant::RGBA::new(p[0], p[1], p[2], p[3]))
			.collect(),
		png::ColorType::GrayscaleAlpha => buf[..w * h * 2]
			.as_chunks::<2>()
			.0
			.iter()
			.map(|p| imagequant::RGBA::new(p[0], p[0], p[0], p[1]))
			.collect(),
		other => return Err(Error::Engine(format!("unexpected PNG layout {other:?}"))),
	};

	let mut attr = imagequant::new();
	attr.set_speed(4).map_err(quant_err)?;
	attr.set_quality(0, 85).map_err(quant_err)?;
	let mut image = attr.new_image(pixels, w, h, 0.0).map_err(quant_err)?;
	let mut res = attr.quantize(&mut image).map_err(quant_err)?;
	res.set_dithering_level(1.0).map_err(quant_err)?;
	let (palette, indexes) = res.remapped(&mut image).map_err(quant_err)?;

	let mut encoder = png::Encoder::new(BufWriter::new(File::create(output)?), w as u32, h as u32);
	encoder.set_color(png::ColorType::Indexed);
	encoder.set_depth(png::BitDepth::Eight);
	encoder.set_compression(png::Compression::High);
	encoder.set_palette(palette.iter().flat_map(|c| [c.r, c.g, c.b]).collect::<Vec<u8>>());
	if palette.iter().any(|c| c.a < 255) {
		encoder.set_trns(palette.iter().map(|c| c.a).collect::<Vec<u8>>());
	}
	let mut writer = encoder.write_header().map_err(|e| Error::Engine(e.to_string()))?;
	writer
		.write_image_data(&indexes)
		.map_err(|e| Error::Engine(e.to_string()))?;
	writer.finish().map_err(|e| Error::Engine(e.to_string()))?;
	Ok(())
}

fn quant_err(e: imagequant::Error) -> Error {
	Error::Engine(format!("can't compress PNG: {e}"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn gradient_gets_smaller_and_stays_readable() {
		let dir = std::env::temp_dir().join(format!("tw-pq-{}", std::process::id()));
		std::fs::create_dir_all(&dir).unwrap();
		let src = dir.join("in.png");
		let (w, h) = (256u32, 256u32);
		let mut data = Vec::new();
		for y in 0..h {
			for x in 0..w {
				data.extend_from_slice(&[x as u8, y as u8, ((x * y) % 255) as u8, if x < 16 { 0 } else { 255 }]);
			}
		}
		let mut enc = png::Encoder::new(File::create(&src).unwrap(), w, h);
		enc.set_color(png::ColorType::Rgba);
		enc.set_depth(png::BitDepth::Eight);
		enc.write_header().unwrap().write_image_data(&data).unwrap();

		let out = dir.join("out.png");
		compress(&src, &out).unwrap();
		let before = std::fs::metadata(&src).unwrap().len();
		let after = std::fs::metadata(&out).unwrap().len();
		assert!(after < before, "{after} >= {before}");

		let reader = png::Decoder::new(BufReader::new(File::open(&out).unwrap()))
			.read_info()
			.unwrap();
		assert_eq!(reader.info().width, w);
		assert!(reader.info().trns.is_some());
		std::fs::remove_dir_all(dir).unwrap();
	}
}
