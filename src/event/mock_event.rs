#![cfg(debug_assertions)]
#![allow(dead_code)]

use error::Error;
use event::Printable;
use runner::Counter;

use separator::Separatable;
use std::fmt;

pub struct MockEvent<'a> {
	name: &'a str,
	value: u64,
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
		Ok(42)
	}
}

impl<'a> fmt::Display for MockEvent<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"{:>30}: {:>14}",
			self.name,
			self.value.separated_string()
		)
	}
}

impl<'a> Printable for MockEvent<'a> {
	fn name(&self) -> &str {
		self.name
	}
	fn value(&self) -> u64 {
		self.value
	}
}

impl<'a> MockEvent<'a> {
	pub fn new(name: &'a str, value: u64) -> Self {
		MockEvent { name, value }
	}
}
