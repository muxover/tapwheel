pub mod archive;
pub mod kind;
pub mod ops;
pub mod plan;
pub mod pngquant;
pub mod run;
pub mod settings;
pub mod subs;

pub use kind::Kind;
pub use ops::{Format, Op, Wheel, choices};
pub use plan::{Job, plan};
pub use settings::{Key, Settings};

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
