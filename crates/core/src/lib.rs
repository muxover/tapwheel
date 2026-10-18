pub mod archive;
pub mod bg;
pub mod code;
pub mod codeshot;
pub mod ff;
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
pub use ops::{Mode, Op, Preset, Wheel, choices};
pub use plan::{Job, plan};
pub use recipe::{Recipe, recipe_for};
pub use settings::{Defaults, Key, Settings};

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

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Self {
		Error::Io(e)
	}
}
