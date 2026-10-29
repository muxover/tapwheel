pub mod archive;
pub mod bg;
pub mod budget;
pub mod code;
pub mod codeshot;
pub mod device;
pub mod ff;
pub mod files;
pub mod format;
pub mod hash;
pub mod icons;
pub mod image;
pub mod kind;
pub mod media;
pub mod metadata;
pub mod ops;
pub mod plan;
pub mod pngquant;
pub mod recipe;
pub mod run;
pub mod settings;
pub mod speech;
pub mod subs;
pub mod text;
#[cfg(windows)]
mod winicon;

pub use format::Format;
pub use kind::Kind;
pub use ops::{Mode, Op, Preset, Wheel, catalog, choices};
pub use plan::{Job, plan};
pub use recipe::{Recipe, recipe_for};
pub use settings::{Defaults, Key, Layout, NoDrag, Settings, WheelSize};

#[derive(Debug)]
pub enum Error {
	Io(std::io::Error),
	Engine(String),
	Cancelled,
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::Io(e) => write!(f, "{e}"),
			Error::Engine(s) => f.write_str(s),
			Error::Cancelled => f.write_str("cancelled"),
		}
	}
}

impl std::error::Error for Error {}

/// Runs work that may panic inside a library, so a bad file fails its job instead of closing
/// the app. The panic hook still logs where it happened.
pub fn caught<T>(failed: &str, f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
	std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| Err(Error::Engine(failed.to_string())))
}

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Self {
		Error::Io(e)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_panic_becomes_an_error() {
		let r: Result<(), Error> = caught("this file can't be read", || panic!("deep inside a decoder"));
		assert_eq!(r.unwrap_err().to_string(), "this file can't be read");
	}
}
