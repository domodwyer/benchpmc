#![cfg(target_os = "freebsd")]

extern crate pmc;

use event::Printable;

use std::fmt;
use error::Error;
use runner::Counter;
use separator::Separatable;

#[derive(Debug)]
/// `PmcEvent` interfaces with a [`Counter`] in [`pmc-rs`], and provides output
/// formatting of the counter values.
///
/// A `PmcEvent` records the counter value every time the [set] method is
/// called.
///
/// [set]: #method.set  
/// [`pmc-rs`]: https://crates.io/crates/pmc-rs  
/// [`Counter`]: https://itsallbroken.com/code/docs/pmc-rs/pmc/struct.Counter.html  
///
pub struct PmcEvent<'a> {
	spec: &'a str,
	alias: Option<&'a str>,
	value: Option<u64>,
	counter: pmc::Counter<'a>,
}

impl<'a> PmcEvent<'a> {
	pub fn new(spec: &'a str) -> Result<Self, Error> {
		let counter = pmc::Counter::new(spec, &pmc::Scope::Process, pmc::CPU_ANY)?;

		Ok(PmcEvent {
			spec,
			counter,
			alias: None,
			value: None,
		})
	}

	/// Set an alternative (human friendly) name for the configured event,
	/// displayed when printing the counter value instead of the raw event name.
	pub fn alias(mut self, alias: &'a str) -> Self {
		self.alias = Some(alias);
		self
	}
}

impl<'a> Counter for PmcEvent<'a> {
	fn attach(&mut self, pid: u32) -> Result<(), Error> {
		self.counter.attach(pid).map_err(Error::PmcError)?;

		// Another hwpmc quirk? This process has to allocate and run a PMC after
		// attaching PMCs to the child, otherwise the PMCs attached to the child
		// process don't always run.
		//
		// LOCK.FAILED is guaranteed to exist as it's part of the pmc.soft(3)
		// class.
		//
		// This PMC is released when it drops from this scope.
		if let Ok(mut counter) =
			pmc::Counter::new("LOCK.FAILED", &pmc::Scope::Process, pmc::CPU_ANY)
		{
			let _ = counter.attach(0);
			let _ = counter.start();
			let _ = counter.stop();
		}

		Ok(())
	}

	fn start(&mut self) -> Result<(), Error> {
		self.counter.start().map_err(Error::PmcError)
	}

	fn stop(&mut self) -> Result<(), Error> {
		self.counter.stop().map_err(Error::PmcError)
	}

	fn set(&mut self, value: u64) -> Result<u64, Error> {
		self.counter
			.set(value)
			.and_then(|v| {
				self.value = Some(v);
				Ok(v)
			})
			.map_err(Error::PmcError)
	}
}

impl<'a> fmt::Display for PmcEvent<'a> {
	/// Prints the counter name (or alias) and value in the format:
	///
	/// ```text
	///                   instructions: 19,031,333,328
	/// ```
	///
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"{:>30}: {:>14}",
			self.alias.unwrap_or(self.spec),
			self.value.unwrap_or(0).separated_string(),
		)
	}
}

impl<'a> Printable for PmcEvent<'a> {
	fn name(&self) -> &str {
		self.alias.unwrap_or(self.spec)
	}
	fn value(&self) -> u64 {
		self.value.unwrap_or(0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[ignore]
	fn test_event() {
		let mut event = PmcEvent::new("instructions").unwrap();

		assert_eq!(event.spec, "instructions");
		assert_eq!(event.alias, None);
		assert_eq!(event.value(), 0);

		assert!(event.attach(0).is_ok());
		assert!(event.start().is_ok());
		assert!(event.stop().is_ok());

		let v = event.set(0).unwrap();
		assert!(v > 0);
		assert_eq!(event.value(), v);
	}

	#[test]
	#[ignore]
	fn test_alias() {
		let event = PmcEvent::new("instructions").unwrap().alias("alias");

		assert_eq!(event.spec, "instructions");
		assert_eq!(event.alias, Some("alias"));
	}
}
