//! Writes what the wheel offers for each kind of file as JSON, so the website lists exactly
//! what the app does.
//!
//! cargo run -p tapwheel-core --example site_data > site/public/data/wheel.json

use serde_json::{Value, json};
use tapwheel_core::kind::{self, kind_of};
use tapwheel_core::ops::{Mode, Op, Wheel, choices};

fn slice(op: Op, kind: Option<tapwheel_core::Kind>) -> Value {
	json!({
		"id": op.id(),
		"label": op.label(),
		"icon": op.icon(kind),
		"desc": op.describe(kind),
		"ring": op.presets(kind).iter().map(|p| p.label).collect::<Vec<_>>(),
		"window": op.mode() == Mode::Editor,
	})
}

fn main() {
	let kinds = [
		(
			"images",
			"Images",
			"photo.heic",
			&["photo.heic", "shot.png"][..],
			kind::IMAGE_EXTS,
		),
		(
			"video",
			"Video",
			"clip.mp4",
			&["clip.mp4", "intro.mp4"][..],
			kind::VIDEO_EXTS,
		),
		(
			"audio",
			"Audio",
			"song.mp3",
			&["song.mp3", "voice.m4a"][..],
			kind::AUDIO_EXTS,
		),
		(
			"icons",
			"Icons",
			"logo.svg",
			&["logo.svg", "mark.svg"][..],
			kind::ICON_EXTS,
		),
		(
			"text",
			"Text",
			"notes.txt",
			&["notes.txt", "todo.txt"][..],
			kind::TEXT_EXTS,
		),
		("code", "Code", "app.ts", &["app.ts", "util.ts"][..], kind::CODE_EXTS),
		(
			"archives",
			"Archives",
			"album.zip",
			&["album.zip", "backup.zip"][..],
			kind::ARCHIVE_EXTS,
		),
	];
	let mut out = Vec::new();
	for (id, name, one, two, exts) in kinds {
		let k = Some(kind_of(std::path::Path::new(one)));
		let wheel = |w: Wheel, files: &[&str]| choices(w, files).into_iter().map(|op| slice(op, k)).collect::<Vec<_>>();
		let single: Vec<String> = choices(Wheel::Tools, &[one])
			.iter()
			.map(|o| o.id().to_string())
			.collect();
		let many: Vec<Value> = choices(Wheel::Tools, two)
			.into_iter()
			.filter(|o| !single.contains(&o.id().to_string()))
			.map(|op| slice(op, k))
			.collect();
		out.push(json!({
			"id": id,
			"name": name,
			"sample": one,
			"reads": exts,
			"convert": wheel(Wheel::Convert, &[one]),
			"tools": wheel(Wheel::Tools, &[one]),
			"many": many,
			"actions": wheel(Wheel::Actions, &[one]),
		}));
	}
	println!(
		"{}",
		serde_json::to_string_pretty(&json!({ "version": env!("CARGO_PKG_VERSION"), "kinds": out })).unwrap()
	);
}
