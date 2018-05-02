#[cfg(target_os = "freebsd")]
use pmc;

use std::fmt;
use std::ffi::NulError;

#[derive(Debug, PartialEq)]
pub enum Error {
	#[cfg(test)]
	MockError,
	#[cfg(target_os = "freebsd")]
	PmcError(pmc::error::Error),
	ExecError(String),
}

#[cfg(target_os = "freebsd")]
impl From<pmc::error::Error> for Error {
	fn from(error: pmc::error::Error) -> Self {
		Error::PmcError(error)
	}
}

impl From<NulError> for Error {
	fn from(_error: NulError) -> Self {
		Error::ExecError(String::from("input contained unexpected null character"))
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Error::ExecError(ref e) => write!(f, "{}", e),

			#[cfg(target_os = "freebsd")]
			Error::PmcError(ref e) => e.fmt(f),

			#[cfg(test)]
			_ => write!(f, "unknown error"),
		}
	}
}
