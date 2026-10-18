//! Draws the wheels exactly as the app builds them (real tool lists, rings and keys) for the
//! Store and website pictures.
//! cargo run -p tapwheel-ui --example store_wheels -- <out dir> <regular font> <semibold font> [scale]

use std::path::PathBuf;

use tapwheel_core::ops::{Wheel, choices, selection_kind};
use tapwheel_ui::wheel::{self, Hover, KeyHint, Motion, Slice, WheelView};
use tapwheel_ui::{Fonts, Rgba, Theme};

fn view(file: &str, w: Wheel, hover: Hover) -> WheelView {
	let files = vec![PathBuf::from(file)];
	let kind = selection_kind(&files);
	let slices: Vec<Slice> = choices(w, &files)
		.into_iter()
		.map(|op| Slice {
			label: op.label().to_string(),
			icon: op.icon(kind).to_string(),
			desc: op.describe(kind),
			ring: op.presets(kind).iter().map(|p| p.label.to_string()).collect(),
		})
		.collect();
	let mut hover_amounts = vec![0.0; 8];
	if let Hover::Slice(i) | Hover::Ring(i, _) = hover {
		hover_amounts[i] = 1.0;
	}
	let active = [Wheel::Convert, Wheel::Tools, Wheel::Actions].iter().position(|x| *x == w).unwrap_or(0);
	WheelView {
		slices,
		page: 0,
		hover,
		keyboard: false,
		title: w.title().into(),
		subtitle: file.into(),
		keys: ["Convert", "Tools", "Actions"]
			.iter()
			.zip(["Shift", "Ctrl", "Alt"])
			.enumerate()
			.map(|(i, (name, key))| KeyHint {
				key: key.into(),
				name: name.to_string(),
				active: i == active,
			})
			.collect(),
		message: None,
		hint: None,
		motion: Motion {
			hover: hover_amounts,
			..Motion::default()
		},
	}
}

fn main() {
	let args: Vec<String> = std::env::args().collect();
	let out = PathBuf::from(&args[1]);
	std::fs::create_dir_all(&out).unwrap();
	let fonts = Fonts::new(std::fs::read(&args[2]).unwrap(), std::fs::read(&args[3]).unwrap()).unwrap();
	let scale: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2.0);
	let find = |v: &WheelView, label: &str| v.slices.iter().position(|s| s.label == label).expect(label);
	let tools = view("photo.jpg", Wheel::Tools, Hover::None);
	let compress = find(&tools, "Compress");
	let actions = view("photo.jpg", Wheel::Actions, Hover::None);
	let copy_path = find(&actions, "Copy path");
	let convert = view("photo.heic", Wheel::Convert, Hover::None);
	let jpg = find(&convert, "JPG");
	let shots = [
		("tools-ring", view("photo.jpg", Wheel::Tools, Hover::Ring(compress, 0))),
		("convert", view("photo.heic", Wheel::Convert, Hover::Slice(jpg))),
		("actions-ring", view("photo.jpg", Wheel::Actions, Hover::Ring(copy_path, 1))),
	];
	for dark in [true, false] {
		let theme = Theme {
			dark,
			win11: true,
			accent: Rgba::hex(0x0078d4),
			shade: Some(if dark { Rgba::hex(0x60cdff) } else { Rgba::hex(0x005fb8) }),
			high_contrast: false,
		};
		for (name, v) in &shots {
			let pm = wheel::render(v, &theme, &fonts, scale, None);
			pm.save_png(out.join(format!("{name}-{}.png", if dark { "dark" } else { "light" }))).unwrap();
		}
	}
}
