use error::Error;
use event::Printable;
use runner::Counter;
use std::fmt::{self, Display};

/// `RelativePrinter` decorates a counter value with a percentage relative to
/// the configured absolute value when formatting for output.
///
/// # Examples
/// ```text
///                   instructions: 19,031,333,328
///                unhalted-cycles:  7,002,094,130    ( 36.8% of instructions)
/// ```
///
#[allow(dead_code)]
pub struct RelativePrinter<T: Printable + Counter + Display> {
	absolute: T,
	relatives: Vec<T>,
}

impl<T> fmt::Display for RelativePrinter<T>
where
	T: Printable + Counter + Display,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.absolute.fmt(f)?;
		writeln!(f)?;

		for r in &self.relatives {
			r.fmt(f)?;
			let rel = (r.value() as f64 / self.absolute.value() as f64) * f64::from(100);
			writeln!(f, "    ({: >5.1}% of {})", rel, self.absolute.name(),)?;
		}

		Ok(())
	}
}

impl<T> Counter for RelativePrinter<T>
where
	T: Printable + Counter + Display,
{
	fn attach(&mut self, pid: u32) -> Result<(), Error> {
		self.absolute.attach(pid)?;
		for c in &mut self.relatives {
			c.attach(pid)?;
		}
		Ok(())
	}
	fn start(&mut self) -> Result<(), Error> {
		self.absolute.start()?;
		for c in &mut self.relatives {
			c.start()?;
		}
		Ok(())
	}
	fn stop(&mut self) -> Result<(), Error> {
		self.absolute.stop()?;
		for c in &mut self.relatives {
			c.stop()?;
		}
		Ok(())
	}
	fn set(&mut self, value: u64) -> Result<u64, Error> {
		self.absolute.set(value)?;
		for c in &mut self.relatives {
			c.set(value)?;
		}
		Ok(0)
	}
}

impl<T> RelativePrinter<T>
where
	T: Printable + Counter + Display,
{
	#[allow(dead_code)]
	pub fn new(absolute: T, relatives: Vec<T>) -> Self {
		RelativePrinter {
			absolute,
			relatives,
		}
	}
}
