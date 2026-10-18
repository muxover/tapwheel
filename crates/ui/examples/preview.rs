//! Renders every surface to PNG so the look can be checked without Windows.
//! cargo run -p tapwheel-ui --example preview -- <out dir> <regular font> <semibold font> [scale]

use std::path::PathBuf;

use tapwheel_ui::hud::{self, Card, HudView, State};
use tapwheel_ui::wheel::{self, Hover, KeyHint, Motion, Slice, WheelView};
use tapwheel_ui::{Fonts, Rgba, Theme};

fn slices(list: &[(&str, &str, &[&str])]) -> Vec<Slice> {
	list.iter()
		.map(|(label, icon, ring)| Slice {
			label: label.to_string(),
			icon: icon.to_string(),
			desc: format!("What {label} does"),
			ring: ring.iter().map(|r| r.to_string()).collect(),
		})
		.collect()
}

/// A busy stand-in for the screen behind the wheel: a wallpaper, windows and icons.
fn desktop(w: u32, h: u32) -> tiny_skia::Pixmap {
	use tapwheel_ui::draw::{fill_circle, fill_rrect};
	let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
	for y in 0..h {
		let t = y as f32 / h as f32;
		fill_rrect(
			&mut pm,
			0.0,
			y as f32,
			w as f32,
			1.0,
			0.0,
			Rgba::from_u8((15.0 + 50.0 * t) as u8, 50, (115.0 - 50.0 * t) as u8),
		);
	}
	let (fw, fh) = (w as f32, h as f32);
	fill_rrect(
		&mut pm,
		fw * 0.05,
		fh * 0.1,
		fw * 0.5,
		fh * 0.6,
		0.0,
		Rgba::hex(0xf3f3f3),
	);
	fill_rrect(
		&mut pm,
		fw * 0.08,
		fh * 0.2,
		fw * 0.4,
		fh * 0.03,
		0.0,
		Rgba::hex(0x202020),
	);
	fill_rrect(
		&mut pm,
		fw * 0.08,
		fh * 0.3,
		fw * 0.3,
		fh * 0.03,
		0.0,
		Rgba::hex(0x202020),
	);
	fill_rrect(
		&mut pm,
		fw * 0.6,
		fh * 0.55,
		fw * 0.35,
		fh * 0.35,
		0.0,
		Rgba::hex(0xff8c00),
	);
	for i in 0..6 {
		fill_circle(
			&mut pm,
			fw * (0.15 + 0.13 * i as f32),
			fh * 0.85,
			fw * 0.04,
			Rgba::hex(0x60cdff),
		);
	}
	pm
}

fn keys(active: usize) -> Vec<KeyHint> {
	["Convert", "Tools", "Actions"]
		.iter()
		.zip(["Shift", "Ctrl", "Alt"])
		.enumerate()
		.map(|(i, (name, key))| KeyHint {
			key: key.into(),
			name: name.to_string(),
			active: i == active,
		})
		.collect()
}

fn main() {
	let args: Vec<String> = std::env::args().collect();
	let out = PathBuf::from(&args[1]);
	std::fs::create_dir_all(&out).unwrap();
	let fonts = Fonts::new(std::fs::read(&args[2]).unwrap(), std::fs::read(&args[3]).unwrap()).unwrap();
	let scale: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.5);
	let themes = [
		("win11-dark", true, true),
		("win11-light", false, true),
		("win10-dark", true, false),
		("win10-light", false, false),
	];
	let tools = slices(&[
		("Crop", "crop", &[]),
		("Add BG", "image_add", &[]),
		("Remove BG", "image_sparkle", &[]),
		("Compress", "arrow_minimize", &["Balanced", "Smaller", "Best", "Custom"]),
		("Resize", "resize", &["50%", "1920", "1280", "1080", "Custom"]),
		("Adjust", "options", &[]),
		("Annotate", "pen", &[]),
		("Redact", "eye_off", &[]),
		(
			"Rotate",
			"arrow_rotate_clockwise",
			&["Right", "Left", "180°", "Mirror", "Flip"],
		),
		("Metadata", "info", &[]),
	]);
	let convert = slices(&[
		("PNG", "", &[]),
		("JPG", "", &[]),
		("WebP", "", &[]),
		("AVIF", "", &[]),
		("GIF", "", &[]),
		("TIFF", "", &[]),
		("BMP", "", &[]),
		("ICO", "", &[]),
	]);
	for (name, dark, win11) in themes {
		let theme = Theme {
			dark,
			win11,
			accent: Rgba::hex(0x0078d4),
			shade: None,
			high_contrast: false,
		};
		let base = WheelView {
			slices: tools.clone(),
			page: 0,
			hover: Hover::None,
			keyboard: false,
			title: "Tools".into(),
			subtitle: "IMG_2041.heic".into(),
			keys: keys(1),
			message: None,
			hint: None,
			motion: Motion::default(),
		};
		let states: Vec<(&str, WheelView)> = vec![
			("tools", base.clone()),
			(
				"ring",
				WheelView {
					hover: Hover::Ring(3, 1),
					hint: Some("About 640 KB".into()),
					motion: Motion {
						hover: vec![0.0, 0.0, 0.0, 1.0],
						..Motion::default()
					},
					..base.clone()
				},
			),
			(
				"convert",
				WheelView {
					slices: convert.clone(),
					title: "Convert".into(),
					keys: keys(0),
					hover: Hover::Slice(1),
					motion: Motion {
						hover: vec![0.0, 1.0],
						..Motion::default()
					},
					..base.clone()
				},
			),
			(
				"page2",
				WheelView {
					page: 1,
					hover: Hover::More,
					..base.clone()
				},
			),
		];
		for (state, v) in states {
			let pm = wheel::render(&v, &theme, &fonts, scale, None);
			pm.save_png(out.join(format!("wheel-{state}-{name}.png"))).unwrap();
			// on Windows 11 the disc is frosted glass over whatever is behind it
			if win11 {
				let (w, h) = wheel::size(scale);
				let glass = tapwheel_ui::draw::frost(desktop(w / 4, h / 4), w, h, dark);
				let pm = wheel::render(&v, &theme, &fonts, scale, glass.as_ref());
				pm.save_png(out.join(format!("wheel-{state}-{name}-glass.png")))
					.unwrap();
			}
		}
		let cards = vec![
			Card {
				title: "Compressing".into(),
				detail: "IMG_2041.heic".into(),
				state: State::Running,
				progress: Some(0.42),
				buttons: vec![hud::Button::Cancel],
				icon: "arrow_minimize".into(),
				shown: 1.0,
			},
			Card {
				title: "Converted to JPG".into(),
				detail: "4.1 MB to 1.2 MB, 71% smaller".into(),
				state: State::Done,
				progress: None,
				buttons: vec![hud::Button::Open, hud::Button::Show, hud::Button::Undo],
				icon: "".into(),
				shown: 1.0,
			},
			Card {
				title: "Copied the path".into(),
				detail: "IMG_2041.heic".into(),
				state: State::Done,
				progress: None,
				buttons: vec![],
				icon: "".into(),
				shown: 1.0,
			},
		];
		let view = HudView {
			cards,
			hover: None,
			time: 0.3,
		};
		hud::render(&view, &[0.0, 1.0, 0.0], &theme, &fonts, scale)
			.save_png(out.join(format!("cards-{name}.png")))
			.unwrap();
	}
}
