#![cfg(debug_assertions)]

use error::Error;
use event::Printable;
use runner::Counter;

use std::fmt;

pub struct MockEvent<'a> {
	name: &'a str,
	value: Option<u64>,
	values: Vec<u64>,
}

impl<'a> Counter for MockEvent<'a> {
	fn attach(&mut self, _pid: u32) -> Result<(), Error> {
		Ok(())
	}
	fn start(&mut self) -> Result<(), Error> {
		Ok(())
	}
	fn stop(&mut self) -> Result<(), Error> {
		Ok(())
	}
	fn set(&mut self, _value: u64) -> Result<u64, Error> {
		self.value = self.values.pop();
		Ok(self.value.unwrap())
	}
}

impl<'a> fmt::Display for MockEvent<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "mock",)
	}
}

impl<'a> Printable for MockEvent<'a> {
	fn name(&self) -> &str {
		self.name
	}
	fn value(&self) -> u64 {
		self.value.unwrap()
	}
}

impl<'a> MockEvent<'a> {
	pub fn new(name: &'a str, values: &[u64]) -> Self {
		let v = values.to_vec();
		MockEvent {
			name,
			values: v,
			value: None,
		}
	}
}
