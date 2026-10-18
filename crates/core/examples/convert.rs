//! Runs one wheel choice on files, the way the app does. Useful for checking a conversion by hand.
//! cargo run -p tapwheel-core --example convert -- <ffmpeg> <op id> <file>...

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use tapwheel_core::plan;
use tapwheel_core::run::{Engines, run};
use tapwheel_core::settings::op_from_id;

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let ffmpeg = PathBuf::from(&args[0]);
	let op = op_from_id(&args[1]).expect("unknown op id, e.g. to-jpg or compressimage");
	let files: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
	let engines = Engines {
		ffmpeg,
		scratch: std::env::temp_dir().join("tapwheel-example"),
	};
	for job in plan(op, &files) {
		match run(&job, &engines, &mut |_| {}, &AtomicBool::new(false)) {
			Ok(out) => println!("{}\t{}\t{}", out.output.display(), out.bytes_in, out.bytes_out),
			Err(e) => eprintln!("failed: {e}"),
		}
	}
}
