//! Prints what every readable extension gets on each wheel, for checking the whole mapping at once.
//! cargo run -p tapwheel-core --example wheel_audit

use std::path::PathBuf;

use tapwheel_core::kind::*;
use tapwheel_core::ops::{Wheel, choices};

fn main() {
	let lists: [(&str, &[&str]); 9] = [
		("image", IMAGE_EXTS),
		("raw", RAW_EXTS),
		("video", VIDEO_EXTS),
		("audio", AUDIO_EXTS),
		("icon", ICON_EXTS),
		("program", PROGRAM_EXTS),
		("text", TEXT_EXTS),
		("code", CODE_EXTS),
		("archive", ARCHIVE_EXTS),
	];
	for (group, exts) in lists {
		for ext in exts {
			let f = vec![PathBuf::from(format!("file.{ext}"))];
			let names = |w| {
				choices(w, &f)
					.iter()
					.map(|o| o.label().to_string())
					.collect::<Vec<_>>()
					.join(", ")
			};
			println!(
				"{group}\t{ext}\t{:?}\tC: {}\tT: {}",
				kind_of_ext(ext),
				names(Wheel::Convert),
				names(Wheel::Tools)
			);
		}
	}
}
