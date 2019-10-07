use error::Error;
use event::Printable;
use runner::Counter;
use std::fmt::{self, Display};

use separator::Separatable;

/// `RSDPrinter` decorates a counter value with [relative standard deviation] of
/// multiple observed counter values.
///
/// A counter value is observed when [set] is called.
///
/// # Examples
/// ```text
///                unhalted-cycles:  7,002,094,130 ±4.2%
/// ```
///
/// [relative standard deviation]: https://en.wikipedia.org/wiki/Coefficient_of_variation  
/// [set]: #method.set
///
pub struct RSDPrinter<T: Counter + Printable + Display> {
	counter: T,
	values: Vec<u64>,
}

impl<T> Counter for RSDPrinter<T>
where
	T: Counter + Printable + Display,
{
	fn attach(&mut self, pid: u32) -> Result<(), Error> {
		self.counter.attach(pid)
	}
	fn start(&mut self) -> Result<(), Error> {
		self.counter.start()
	}
	fn stop(&mut self) -> Result<(), Error> {
		self.counter.stop()
	}
	fn set(&mut self, value: u64) -> Result<u64, Error> {
		// TODO: cache computed stats values and reset here?

		self.counter.set(value).and_then(|v| {
			self.values.push(v);
			Ok(v)
		})
	}
}

impl<T> Display for RSDPrinter<T>
where
	T: Counter + Printable + Display,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let rsd = if self.values.len() > 1 {
			format!("±{:<5}", format!("{:.1}%", self.rsd()))
		} else {
			String::from("      ")
		};

		write!(
			f,
			"{:>30}: {:>14} {}",
			self.counter.name(),
			self.value().separated_string(),
			rsd,
		)
	}
}

impl<T> Printable for RSDPrinter<T>
where
	T: Counter + Printable + Display,
{
	fn name(&self) -> &str {
		self.counter.name()
	}

	fn value(&self) -> u64 {
		self.mean()
	}
}

impl<T> RSDPrinter<T>
where
	T: Counter + Printable + Display,
{
	#[allow(dead_code)]
	pub fn new(counter: T) -> Self {
		RSDPrinter {
			counter,
			values: Vec::new(),
		}
	}

	/// rsd returns the relative standard deviation of the observed counter values.
	pub fn rsd(&self) -> f64 {
		if self.values.len() < 2 {
			// Don't panic on division of 0
			return 0.0;
		}

		(self.stddev() * f64::from(100)) / self.mean() as f64
	}

	/// mean returns the arithmetic mean of the observed counter values.
	fn mean(&self) -> u64 {
		if self.values.is_empty() {
			return 0;
		}

		self.values.iter().sum::<u64>() / self.values.len() as u64
	}

	/// variance returns the variance of the observed counter values.
	fn variance(&self) -> f64 {
		if self.values.len() < 2 {
			// Don't panic on division of (len - 1) below
			return 0.0;
		}

		let mean = self.mean() as f64;
		let total = self.values.iter().fold(0.0, |acc, v| {
			let v = *v as f64;
			let x = v - mean;
			acc + x * x
		});

		let divisor = (self.values.len() - 1) as f64;
		total / divisor
	}

	/// stddev returns the standard deviation of the observed counter values.
	fn stddev(&self) -> f64 {
		self.variance().sqrt()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	mod mock_event;
	use self::mock_event::MockEvent;

	#[test]
	fn stats() {
		let mut values = vec![0, 10, 20, 30, 40];
		let mut p = RSDPrinter::new(MockEvent::new("mock", &values.clone()));

		// Mock pops, so swap ordering
		values.reverse();

		for v in values.iter() {
			p.set(0).unwrap(); // drive the mock
			assert_eq!(p.counter.value(), *v);
		}

		// Average
		assert_eq!(p.value(), 20);

		// Variance
		assert_eq!(p.variance(), 250.0);
		assert_eq!(p.stddev() as f32, 15.811388);

		// RSD
		assert_eq!(p.rsd() as f32, 79.0569415);
	}

	#[test]
	fn div_zero() {
		let values = vec![];
		let p = RSDPrinter::new(MockEvent::new("mock", &values));

		assert_eq!(p.value(), 0);
		assert_eq!(p.variance(), 0.0);
		assert_eq!(p.stddev(), 0.0);
		assert_eq!(p.rsd(), 0.0);
	}

	#[test]
	fn div_zero_one_val() {
		let values = vec![42];
		let p = RSDPrinter::new(MockEvent::new("mock", &values));

		assert_eq!(p.value(), 0);
		assert_eq!(p.variance(), 0.0);
		assert_eq!(p.stddev(), 0.0);
		assert_eq!(p.rsd(), 0.0);
	}
}
