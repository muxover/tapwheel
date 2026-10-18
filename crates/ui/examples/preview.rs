//! Renders every surface to PNG so the look can be checked without Windows.
//! cargo run -p tapwheel-ui --example preview -- <out dir> <regular font> <semibold font>

use std::path::PathBuf;

use tapwheel_ui::home::{self, HomeView};
use tapwheel_ui::hud::{self, Button, Card, HudView, State};
use tapwheel_ui::wheel::{self, Hover, Slice, WheelView};
use tapwheel_ui::{Fonts, Rgba, Theme, icon};

fn main() {
	let args: Vec<String> = std::env::args().collect();
	let out = PathBuf::from(&args[1]);
	std::fs::create_dir_all(&out).unwrap();
	let fonts = Fonts::new(std::fs::read(&args[2]).unwrap(), std::fs::read(&args[3]).unwrap()).unwrap();
	let scale: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.5);

	let themes = [
		(
			"win11-dark",
			Theme {
				dark: true,
				win11: true,
				accent: Rgba::hex(0x0078d4),
				high_contrast: false,
			},
		),
		(
			"win11-light",
			Theme {
				dark: false,
				win11: true,
				accent: Rgba::hex(0x0078d4),
				high_contrast: false,
			},
		),
		(
			"win10-dark",
			Theme {
				dark: true,
				win11: false,
				accent: Rgba::hex(0xe74856),
				high_contrast: false,
			},
		),
		(
			"hc",
			Theme {
				dark: true,
				win11: true,
				accent: Rgba::hex(0x0078d4),
				high_contrast: true,
			},
		),
	];
	let slices: Vec<Slice> = ["PNG", "JPG", "WebP", "AVIF", "GIF", "TIFF", "BMP", "ICO"]
		.iter()
		.map(|l| Slice {
			label: l.to_string(),
			desc: format!("Convert to {l}"),
		})
		.collect();
	let keys = home::key_hints([0, 1, 2], 0);
	for (name, theme) in themes {
		let v = WheelView {
			slices: slices.clone(),
			hover: Hover::Slice(2),
			keyboard: false,
			title: "Convert".into(),
			subtitle: "IMG_2041.heic".into(),
			keys: keys.clone(),
			message: None,
			appear: 1.0,
		};
		wheel::render(&v, &theme, &fonts, scale)
			.save_png(out.join(format!("wheel-{name}.png")))
			.unwrap();
		let v2 = WheelView {
			hover: Hover::None,
			keyboard: true,
			..v.clone()
		};
		wheel::render(&v2, &theme, &fonts, scale)
			.save_png(out.join(format!("wheel-idle-{name}.png")))
			.unwrap();
		let tools = WheelView {
			slices: ["Compress", "Half size", "Fit 1920", "Rotate", "Strip data", "ZIP"]
				.iter()
				.map(|l| Slice {
					label: l.to_string(),
					desc: "Remove location and camera data".into(),
				})
				.collect(),
			hover: Hover::Slice(4),
			keys: home::key_hints([0, 1, 2], 1),
			title: "Tools".into(),
			subtitle: "3 images".into(),
			..v.clone()
		};
		wheel::render(&tools, &theme, &fonts, scale)
			.save_png(out.join(format!("wheel-tools-{name}.png")))
			.unwrap();

		let hud = HudView {
			cards: vec![
				Card {
					title: "IMG_2041.png".into(),
					status: "Converting · 42%".into(),
					state: State::Running,
					progress: Some(0.42),
					buttons: vec![Button::Cancel],
					shown: 1.0,
				},
				Card {
					title: "holiday-video-small.mp4".into(),
					status: "4.2 MB to 680 KB, 84% smaller".into(),
					state: State::Done,
					progress: None,
					buttons: vec![Button::Undo, Button::Show, Button::Open, Button::Close],
					shown: 1.0,
				},
				Card {
					title: "broken.heic".into(),
					status: "Invalid data found when processing input".into(),
					state: State::Failed,
					progress: None,
					buttons: vec![Button::Close],
					shown: 1.0,
				},
			],
			hover: Some((1, Button::Open)),
			time: 0.4,
		};
		hud::render(&hud, &theme, &fonts, scale)
			.save_png(out.join(format!("hud-{name}.png")))
			.unwrap();

		let hv = HomeView {
			version: "0.1.0".into(),
			keys: [0, 1, 2],
			start_with_windows: true,
			learn_order: true,
			paused: false,
			hover: Some(home::Action::Samples),
			focus: None,
		};
		home::render(&hv, &theme, &fonts, scale, &icon::logo(128))
			.save_png(out.join(format!("home-{name}.png")))
			.unwrap();
	}
	icon::logo(256).save_png(out.join("logo-256.png")).unwrap();
	icon::logo(32).save_png(out.join("logo-32.png")).unwrap();
	icon::logo(16).save_png(out.join("logo-16.png")).unwrap();
}
