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
