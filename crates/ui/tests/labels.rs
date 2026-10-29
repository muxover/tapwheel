// Every label on every wheel the app can show stays inside its petal or ring chip, for every
// kind of file, every page and every ring, in the Windows 10 and Windows 11 shapes. The fonts
// are Selawik, Microsoft's open font with Segoe UI's widths.

use std::path::PathBuf;

use tapwheel_core::ops::{Wheel, choices, selection_kind};
use tapwheel_ui::Fonts;
use tapwheel_ui::wheel::{self, Hover, Slice, WheelView, label_shapes};
use tiny_skia::{FillRule, Mask, Transform};

fn fonts() -> Fonts {
	Fonts::new(
		include_bytes!("fonts/Selawik-Regular.ttf").to_vec(),
		include_bytes!("fonts/Selawik-Bold.ttf").to_vec(),
	)
	.expect("test fonts")
}

/// Files of every kind, alone and in pairs, so every slice and multi-file tool shows up.
fn selections() -> Vec<Vec<PathBuf>> {
	let one = [
		"photo.heic",
		"photo.jpg",
		"shot.png",
		"clip.mp4",
		"song.mp3",
		"voice.m4a",
		"logo.svg",
		"app.ico",
		"setup.exe",
		"notes.txt",
		"data.json",
		"table.csv",
		"page.html",
		"subs.srt",
		"app.ts",
		"style.css",
		"main.rs",
		"album.zip",
		"backup.7z",
	];
	let mut out: Vec<Vec<PathBuf>> = one.iter().map(|f| vec![PathBuf::from(f)]).collect();
	for (a, b) in [
		("a.jpg", "b.png"),
		("a.mp4", "b.mp4"),
		("a.mp3", "b.wav"),
		("a.txt", "b.txt"),
		("a.ts", "b.ts"),
	] {
		out.push(vec![PathBuf::from(a), PathBuf::from(b)]);
	}
	out
}

fn mask(path: &tiny_skia::Path) -> Mask {
	let (w, h) = wheel::size(1.0);
	let mut m = Mask::new(w, h).unwrap();
	m.fill_path(path, FillRule::Winding, true, Transform::identity());
	m
}

#[test]
fn every_label_stays_inside_its_shape() {
	let fonts = fonts();
	let mut bad = Vec::new();
	let mut checked = 0;
	for files in selections() {
		let kind = selection_kind(&files);
		for w in [Wheel::Convert, Wheel::Tools, Wheel::Actions] {
			let slices: Vec<Slice> = choices(w, &files)
				.into_iter()
				.map(|op| Slice {
					label: op.label().to_string(),
					icon: op.icon(kind).to_string(),
					desc: op.describe(kind),
					ring: op.presets(kind).iter().map(|p| p.label.to_string()).collect(),
				})
				.collect();
			if slices.is_empty() {
				continue;
			}
			for page in 0..wheel::pages(slices.len()) {
				let on_page = wheel::page_items(slices.len(), page).0.len();
				let mut hovers = vec![Hover::None];
				hovers.extend(
					(0..on_page)
						.filter(|&i| !slices[wheel::page_items(slices.len(), page).0[i]].ring.is_empty())
						.map(Hover::Slice),
				);
				for hover in hovers {
					let v = WheelView {
						slices: slices.clone(),
						page,
						hover,
						keyboard: false,
						title: w.title().into(),
						subtitle: String::new(),
						keys: Vec::new(),
						message: None,
						hint: None,
						motion: Default::default(),
					};
					for win11 in [true, false] {
						for (label, text, region) in label_shapes(&v, &fonts, win11) {
							checked += 1;
							let (t, r) = (mask(&text), mask(&region));
							let outside = t
								.data()
								.iter()
								.zip(r.data())
								.filter(|(t, r)| **t > 40 && **r < 128)
								.count();
							if outside > 0 {
								bad.push(format!(
									"{label:?} ({w:?}, {}, {} px outside)",
									files[0].display(),
									outside
								));
							}
						}
					}
				}
			}
		}
	}
	bad.sort();
	bad.dedup();
	assert!(checked > 400, "only {checked} labels checked");
	assert!(
		bad.is_empty(),
		"{} labels leave their shape:\n{}",
		bad.len(),
		bad.join("\n")
	);
}

/// A ring with three choices of your own, at the longest name Settings allow, still fits: names
/// made of the widest letters and of ordinary words.
#[test]
fn rings_with_your_own_choices_stay_inside_their_chips() {
	let fonts = fonts();
	let longest = tapwheel_core::settings::MAX_CHOICE_LABEL;
	let mut bad = Vec::new();
	for own in [
		vec!["W".repeat(longest), "M".repeat(longest), "Q".repeat(longest)],
		vec![
			"To send by email".into(),
			"Phone wallpaper".into(),
			"Small for chats".into(),
		],
	] {
		let mut ring = vec!["Balanced".to_string(), "Smaller".into(), "Best".into()];
		ring.extend(own.iter().map(|s| s.chars().take(longest).collect::<String>()));
		ring.push("Custom".into());
		let slices: Vec<Slice> = (0..6)
			.map(|i| Slice {
				label: if i == 0 { "Compress".into() } else { format!("Tool {i}") },
				icon: "arrow_minimize".into(),
				desc: String::new(),
				ring: if i == 0 { ring.clone() } else { Vec::new() },
			})
			.collect();
		for at in 0..slices.len() {
			let mut slices = slices.clone();
			slices.rotate_right(at);
			let v = WheelView {
				slices,
				page: 0,
				hover: Hover::Slice(at),
				keyboard: false,
				title: "Tools".into(),
				subtitle: String::new(),
				keys: Vec::new(),
				message: None,
				hint: None,
				motion: Default::default(),
			};
			for win11 in [true, false] {
				for (label, text, region) in label_shapes(&v, &fonts, win11) {
					let (t, r) = (mask(&text), mask(&region));
					let outside = t
						.data()
						.iter()
						.zip(r.data())
						.filter(|(t, r)| **t > 40 && **r < 128)
						.count();
					if outside > 0 {
						bad.push(format!("{label:?} at slice {at} ({outside} px outside)"));
					}
				}
			}
		}
	}
	bad.sort();
	bad.dedup();
	assert!(
		bad.is_empty(),
		"{} labels leave their shape:\n{}",
		bad.len(),
		bad.join("\n")
	);
}
