//! TapWheel in File Explorer's right-click menu, for the Store version. A packaged app can't
//! write the classic shell keys the portable version uses, so the package declares this
//! `IExplorerCommand` instead, and Windows 11 shows it at the top of the menu. Choosing it hands
//! the selected files to the running TapWheel, starting it first when it isn't running.

#[cfg(windows)]
mod command;

#[cfg(windows)]
pub use command::{CLSID, DllCanUnloadNow, DllGetClassObject};

/// The message TapWheel's main window reads to open the wheel on files without a drag.
pub fn pick_request<P: AsRef<std::path::Path>>(files: &[P]) -> String {
	let mut text = String::from("pick");
	for f in files {
		text.push('\n');
		text.push_str(&f.as_ref().to_string_lossy());
	}
	text
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn the_request_lists_one_file_per_line() {
		assert_eq!(pick_request::<&str>(&[]), "pick");
		assert_eq!(
			pick_request(&[r"C:\Users\you\Pictures\photo.jpg", r"D:\clip one.mp4"]),
			"pick\nC:\\Users\\you\\Pictures\\photo.jpg\nD:\\clip one.mp4"
		);
	}
}
