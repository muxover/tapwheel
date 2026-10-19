//! Builds the website's sample files and what each tool window is given for them, using the
//! app's own engines, so the site's live tool windows behave like the app.
//!
//! TAPWHEEL_FFMPEG=/usr/bin cargo run -p tapwheel-core --example site_samples -- site/samples site/public/try
//!
//! `site/samples` holds the drawings (`photo.svg`, `subject.svg`), `song.wav` made by `synth.py`
//! and `app.ts`. Everything in the output folder is ours to publish.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::AtomicBool;

use exif::experimental::Writer;
use exif::{Field, In, Rational, Tag, Value};
use serde_json::{Value as Json, json};
use tapwheel_core::run::Engines;
use tapwheel_core::{Format, archive, code, ff, image, media, metadata};

fn main() {
	let args: Vec<String> = std::env::args().collect();
	let src = PathBuf::from(args.get(1).expect("samples folder"));
	let out = PathBuf::from(args.get(2).expect("output folder"));
	std::fs::create_dir_all(&out).unwrap();
	let bin = PathBuf::from(std::env::var("TAPWHEEL_FFMPEG").unwrap_or_else(|_| "/usr/bin".into()));
	let scratch = std::env::temp_dir().join("tapwheel-site-samples");
	std::fs::create_dir_all(&scratch).unwrap();
	let e = Engines::in_dir(&bin, scratch.clone());
	let ffmpeg = |args: &[&str]| {
		let ok = Command::new(&e.ffmpeg)
			.args(["-hide_banner", "-loglevel", "error", "-y"])
			.args(args)
			.status()
			.unwrap()
			.success();
		assert!(ok, "ffmpeg {args:?}");
	};
	let p = |name: &str| out.join(name);
	let s = |path: &Path| path.to_str().unwrap().to_string();

	// a photo, with the details a phone would add
	let (w, h, rgba) = image::render_svg_bytes(&std::fs::read(src.join("photo.svg")).unwrap(), 0, None).unwrap();
	let png = scratch.join("photo.png");
	image::write_png(&png, w, h, &rgba).unwrap();
	let plain = scratch.join("photo.jpg");
	ffmpeg(&["-i", &s(&png), "-q:v", "3", &s(&plain)]);
	let jpg = metadata::insert_exif(&std::fs::read(&plain).unwrap(), &exif_block()).unwrap();
	std::fs::write(p("photo.jpg"), &jpg).unwrap();

	// something to put on a backdrop
	let (sw, sh, rgba) = image::render_svg_bytes(&std::fs::read(src.join("subject.svg")).unwrap(), 0, None).unwrap();
	image::write_png(&p("plant.png"), sw, sh, &rgba).unwrap();

	// music, and a short film made from the photo with that music under it
	ffmpeg(&[
		"-i",
		&s(&src.join("song.wav")),
		"-c:a",
		"aac",
		"-b:a",
		"160k",
		"-metadata",
		"title=Lakeside",
		"-metadata",
		"artist=TapWheel samples",
		"-metadata",
		"album=Made in code",
		"-metadata",
		"date=2026",
		&s(&p("song.m4a")),
	]);
	ffmpeg(&[
		"-loop",
		"1",
		"-i",
		&s(&png),
		"-i",
		&s(&src.join("song.wav")),
		"-filter_complex",
		"[0:v]scale=3200:-1,zoompan=z='1.0+0.0009*on':x='iw/2-(iw/zoom/2)+on*0.6':y='ih/2-(ih/zoom/2)':d=360:s=1280x720:fps=30,format=yuv420p[v];[1:a]atrim=0:12,afade=t=out:st=10.5:d=1.5[a]",
		"-map",
		"[v]",
		"-map",
		"[a]",
		"-t",
		"12",
		"-c:v",
		"libx264",
		"-preset",
		"slow",
		"-crf",
		"24",
		"-c:a",
		"aac",
		"-b:a",
		"128k",
		"-movflags",
		"+faststart",
		&s(&p("clip.mp4")),
	]);
	media::filmstrip(
		&e,
		&p("clip.mp4"),
		ff::probe(&e.ffprobe, &p("clip.mp4")).unwrap().duration,
		14,
		72,
		&p("strip.png"),
	)
	.unwrap();

	// code, and an archive holding a few of these
	let ts = src.join("app.ts");
	std::fs::copy(&ts, p("app.ts")).unwrap();
	let album = scratch.join("album");
	let _ = std::fs::remove_dir_all(&album);
	std::fs::create_dir_all(album.join("photos")).unwrap();
	std::fs::copy(p("photo.jpg"), album.join("photos/lake.jpg")).unwrap();
	std::fs::copy(p("plant.png"), album.join("photos/plant.png")).unwrap();
	std::fs::copy(p("song.m4a"), album.join("song.m4a")).unwrap();
	std::fs::write(album.join("notes.txt"), "Packed by TapWheel for the website.\n").unwrap();
	let zip = p("album.zip");
	let _ = std::fs::remove_file(&zip);
	archive::pack(&[album], Format::Zip, &zip, &AtomicBool::new(false)).unwrap();

	// what each window is given, as the app prepares it
	let probe = |f: &Path| ff::probe(&e.ffprobe, f).unwrap();
	let size = |f: &Path| std::fs::metadata(f).unwrap().len();
	let file = |name: &str| {
		let path = p(name);
		json!({ "name": name, "path": format!("C:\\Users\\you\\Pictures\\{name}"), "ext": name.rsplit('.').next().unwrap(), "size": size(&path) })
	};
	let clip = probe(&p("clip.mp4"));
	let song = probe(&p("song.m4a"));
	let text = std::fs::read_to_string(&ts).unwrap();
	let spans = |dark| -> Json {
		code::highlight(&text, "ts", dark)
			.into_iter()
			.map(|line| {
				line.into_iter()
					.map(|(st, t)| {
						let c = st.foreground;
						json!([format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b), t])
					})
					.collect::<Vec<_>>()
			})
			.collect::<Vec<_>>()
			.into()
	};
	let (tags, cover) = ff::tags(&e.ffprobe, &p("song.m4a")).unwrap();
	let samples = json!({
		"defaults": tapwheel_core::Defaults::default(),
		"files": {
			"photo": file("photo.jpg"),
			"plant": file("plant.png"),
			"clip": file("clip.mp4"),
			"song": file("song.m4a"),
			"code": file("app.ts"),
			"album": file("album.zip"),
		},
		"prepared": {
			"photo": { "url": "photo.jpg", "width": w, "height": h, "fields": metadata::fields(&jpg), "location": metadata::has_location(&jpg) },
			"plant": { "url": "plant.png", "width": sw, "height": sh },
			"clip": {
				"url": "clip.mp4", "duration": clip.duration, "width": clip.shown_size().0, "height": clip.shown_size().1,
				"fps": clip.fps, "hasVideo": true, "hasAudio": true, "filmstrip": "strip.png",
				"wave": round(media::waveform(&e, &p("clip.mp4"), clip.duration, 900).unwrap()),
			},
			"song": {
				"url": "song.m4a", "duration": song.duration, "hasVideo": false, "hasAudio": true,
				"wave": round(media::waveform(&e, &p("song.m4a"), song.duration, 900).unwrap()),
				"tags": tags.into_iter().collect::<std::collections::BTreeMap<_, _>>(), "hasCover": cover,
			},
			"code": { "dark": spans(true), "light": spans(false), "language": code::language("ts") },
			"album": { "items": archive::list(&zip).unwrap() },
		},
	});
	std::fs::write(p("samples.json"), serde_json::to_string(&samples).unwrap()).unwrap();
	let _ = std::fs::remove_dir_all(&scratch);
	println!("samples written to {}", out.display());
}

fn round(v: Vec<f32>) -> Vec<f32> {
	v.into_iter().map(|x| (x * 1000.0).round() / 1000.0).collect()
}

/// Camera details and a location, the way a phone writes them, clearly marked as a sample.
fn exif_block() -> Vec<u8> {
	let ascii = |s: &str| Value::Ascii(vec![s.as_bytes().to_vec()]);
	let dms = |deg: f64| {
		let d = deg.abs();
		let m = (d - d.floor()) * 60.0;
		let sec = (m - m.floor()) * 60.0;
		Value::Rational(vec![
			Rational::from((d.floor() as u32, 1)),
			Rational::from((m.floor() as u32, 1)),
			Rational::from(((sec * 100.0).round() as u32, 100)),
		])
	};
	let fields = [
		Field {
			tag: Tag::Make,
			ifd_num: In::PRIMARY,
			value: ascii("TapWheel"),
		},
		Field {
			tag: Tag::Model,
			ifd_num: In::PRIMARY,
			value: ascii("Sample photo"),
		},
		Field {
			tag: Tag::Software,
			ifd_num: In::PRIMARY,
			value: ascii("Drawn in code"),
		},
		Field {
			tag: Tag::DateTimeOriginal,
			ifd_num: In::PRIMARY,
			value: ascii("2026:09:20 18:42:07"),
		},
		Field {
			tag: Tag::ExposureTime,
			ifd_num: In::PRIMARY,
			value: Value::Rational(vec![Rational::from((1, 250))]),
		},
		Field {
			tag: Tag::FNumber,
			ifd_num: In::PRIMARY,
			value: Value::Rational(vec![Rational::from((18, 10))]),
		},
		Field {
			tag: Tag::PhotographicSensitivity,
			ifd_num: In::PRIMARY,
			value: Value::Short(vec![100]),
		},
		Field {
			tag: Tag::GPSLatitudeRef,
			ifd_num: In::PRIMARY,
			value: ascii("N"),
		},
		Field {
			tag: Tag::GPSLatitude,
			ifd_num: In::PRIMARY,
			value: dms(51.4254),
		},
		Field {
			tag: Tag::GPSLongitudeRef,
			ifd_num: In::PRIMARY,
			value: ascii("W"),
		},
		Field {
			tag: Tag::GPSLongitude,
			ifd_num: In::PRIMARY,
			value: dms(116.1773),
		},
	];
	let mut writer = Writer::new();
	for f in &fields {
		writer.push_field(f);
	}
	let mut buf = Cursor::new(Vec::new());
	writer.write(&mut buf, false).unwrap();
	buf.into_inner()
}
