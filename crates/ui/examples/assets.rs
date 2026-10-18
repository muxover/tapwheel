//! Writes the app icon and Store tile images. cargo run -p tapwheel-ui --example assets -- <out dir>

use std::path::PathBuf;

use tapwheel_ui::icon;

fn main() {
	let out = PathBuf::from(std::env::args().nth(1).expect("out dir"));
	std::fs::create_dir_all(&out).unwrap();
	let save = |name: &str, pm: tiny_skia::Pixmap| pm.save_png(out.join(name)).unwrap();
	for (scale, mul) in [(100, 1), (200, 2)] {
		save(&format!("StoreLogo.scale-{scale}.png"), icon::padded(50 * mul, 50 * mul, 44 * mul));
		save(&format!("Square150x150Logo.scale-{scale}.png"), icon::padded(150 * mul, 150 * mul, 96 * mul));
		save(&format!("Square44x44Logo.scale-{scale}.png"), icon::padded(44 * mul, 44 * mul, 36 * mul));
		save(&format!("Wide310x150Logo.scale-{scale}.png"), icon::padded(310 * mul, 150 * mul, 96 * mul));
	}
	for size in [16, 24, 32, 48, 256] {
		save(&format!("Square44x44Logo.targetsize-{size}.png"), icon::logo(size));
		save(&format!("Square44x44Logo.altform-unplated_targetsize-{size}.png"), icon::logo(size));
		save(&format!("Square44x44Logo.altform-lightunplated_targetsize-{size}.png"), icon::logo(size));
	}
	std::fs::write(out.join("tapwheel.ico"), icon::ico(&[16, 20, 24, 32, 40, 48, 64, 256])).unwrap();
	icon::logo(512).save_png(out.join("logo-512.png")).unwrap();
}
