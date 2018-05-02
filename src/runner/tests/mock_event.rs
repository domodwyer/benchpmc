use error::Error;
use runner::Counter;

pub struct Event {
	pub value: Option<u64>,
	pub attach_err: Option<Error>,
	pub start_err: Option<Error>,
	pub stop_err: Option<Error>,
	pub set_err: Option<Error>,
	pub set_ret: Option<u64>,
}

pub fn new() -> Event {
	Event {
		value: None,
		attach_err: None,
		start_err: None,
		stop_err: None,
		set_err: None,
		set_ret: None,
	}
}

// Helper to convert a Some(Error) into Err(Error), or None into Ok(())
macro_rules! some_to_err {
	($self_:ident, $field:ident) => {
		match $self_.$field.take() {
			Some(v) => Err(v),
			None => Ok(()),
		}
	};
}

impl Counter for Event {
	fn attach(&mut self, _pid: u32) -> Result<(), Error> {
		some_to_err!(self, attach_err)
	}

	fn start(&mut self) -> Result<(), Error> {
		some_to_err!(self, start_err)
	}

	fn stop(&mut self) -> Result<(), Error> {
		some_to_err!(self, stop_err)
	}

	fn set(&mut self, value: u64) -> Result<u64, Error> {
		self.value = Some(value);
		some_to_err!(self, set_err)?;
		Ok(self.set_ret.unwrap_or(0))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn macro_err() {
		let mut e = new();
		e.attach_err = Some(Error::ExecError("!".to_string()));
		assert_eq!(e.attach(42), Err(Error::ExecError("!".to_string())));
	}

	#[test]
	fn macro_ok() {
		let mut e = new();
		e.attach_err = None;
		assert!(e.attach(42).is_ok());
	}
}
