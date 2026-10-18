//! Runs one wheel choice on files, the way the app does. Useful for checking a tool by hand.
//! cargo run -p tapwheel-core --example convert -- <engines dir> <op id>[:<ring choice>] <file>...
//! For example `to-jpg`, `compress:smaller`, `rotate:ccw` or `remove_background`.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use tapwheel_core::ops::selection_kind;
use tapwheel_core::run::{Engines, run};
use tapwheel_core::{Defaults, Op, plan, recipe_for};

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let engines = Engines::in_dir(&PathBuf::from(&args[0]), std::env::temp_dir().join("tapwheel-example"));
	let (id, preset) = match args[1].split_once(':') {
		Some((id, p)) => (id, Some(p)),
		None => (args[1].as_str(), None),
	};
	let op = Op::from_id(id).expect("unknown op id, e.g. to-jpg or compress");
	let files: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
	let d = Defaults::default();
	let recipe = recipe_for(op, preset, selection_kind(&files), &d).expect("this one opens a window in the app");
	for job in plan(op, &recipe, &files) {
		match run(&job, &engines, &d, &mut |_| {}, &AtomicBool::new(false)) {
			Ok(out) => println!("{}\t{}\t{}", out.output.display(), out.bytes_in, out.bytes_out),
			Err(e) => eprintln!("failed: {e}"),
		}
	}
}
