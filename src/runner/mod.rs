mod exec;

use error::Error;

/// Counter abstracts an implementation of a process-attachable counter.
pub trait Counter {
	fn attach(&mut self, pid: u32) -> Result<(), Error>;
	fn start(&mut self) -> Result<(), Error>;
	fn stop(&mut self) -> Result<(), Error>;
	fn set(&mut self, value: u64) -> Result<u64, Error>;
}

/// Runner executes a given target process, attaches the provided counters and
/// runs them for the duration of target execution.
pub struct Runner<'a> {
	target: &'a str,
	args: Option<&'a [&'a str]>,
}

impl<'a> Runner<'a> {
	/// New creates a new Runner that executes target.
	pub fn new(target: &'a str) -> Self {
		Runner { target, args: None }
	}

	/// Specifies arguments to the target process.
	pub fn args(self, args: &'a [&'a str]) -> Self {
		Runner {
			args: Some(args),
			..self
		}
	}

	/// Run starts the execution of the configured target, attaching events to
	/// the child process.
	pub fn run<T: Counter + ?Sized>(&mut self, events: &mut [Box<T>]) -> Result<(), Error> {
		let child = exec::Exec::new(self.target)?
			.args(self.args.unwrap_or(&[]))?
			.exec();

		let pid = child
			.pid()
			.ok_or_else(|| Error::ExecError(String::from("failed to start child process")))?;

		// Attach counters to the child process in one go, then start running
		// them to have the start time delta as low as possible.
		for counter in events.iter_mut() {
			counter.attach(pid)?;
		}

		for counter in events.iter_mut() {
			counter.start().unwrap();
		}

		// Signal the child to start and check it's return value
		match child.run() {
			Some(0) => Ok(()),
			Some(_) => Err("non-zero exit status"),
			None => Err("failed to exec"),
		}.map_err(|e| Error::ExecError(e.to_string()))?;

		// Stop all counters and reset them
		for counter in events.iter_mut() {
			counter.stop().unwrap();
		}

		for counter in events.iter_mut() {
			counter.set(0)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	mod mock_event;

	use super::*;

	#[test]
	fn success() {
		let mut r = Runner::new("/usr/bin/true");

		let mut counters = vec![Box::new(mock_event::new())];

		assert!(r.run(&mut counters).is_ok());
	}

	#[test]
	fn args() {
		let args = vec!["one", "two"];

		let r = Runner::new("/usr/bin/true");
		assert_eq!(r.args, None);

		let r = r.args(&args);
		assert_eq!(r.args, Some(args.as_slice()));
	}

	#[test]
	fn bad_return_code() {
		let mut r = Runner::new("/usr/bin/false");

		assert_eq!(
			r.run(&mut vec![Box::new(mock_event::new())]).unwrap_err(),
			Error::ExecError(String::from("non-zero exit status"))
		);
	}

	#[test]
	fn bad_exec() {
		let mut r = Runner::new("not-a-thing");

		assert_eq!(
			r.run(&mut vec![Box::new(mock_event::new())]).unwrap_err(),
			Error::ExecError(String::from("failed to exec"))
		);
	}

	#[test]
	fn attach_err() {
		let mut err = mock_event::new();
		err.attach_err = Some(Error::MockError);

		let counters = &mut vec![
			Box::new(mock_event::new()),
			Box::new(err),
			Box::new(mock_event::new()),
		];

		let mut r = Runner::new("/usr/bin/true");
		assert_eq!(r.run(counters), Err(Error::MockError));
	}

	#[test]
	#[should_panic]
	fn start_err() {
		let mut err = mock_event::new();
		err.start_err = Some(Error::MockError);

		let counters = &mut vec![
			Box::new(mock_event::new()),
			Box::new(err),
			Box::new(mock_event::new()),
		];

		let _ = Runner::new("/usr/bin/true").run(counters);
	}

	#[test]
	#[should_panic]
	fn stop_err() {
		let mut err = mock_event::new();
		err.stop_err = Some(Error::MockError);

		let counters = &mut vec![
			Box::new(mock_event::new()),
			Box::new(err),
			Box::new(mock_event::new()),
		];

		let _ = Runner::new("/usr/bin/true").run(counters);
	}

	#[test]
	fn set_err() {
		let mut err = mock_event::new();
		err.set_err = Some(Error::MockError);

		let counters = &mut vec![Box::new(err)];
		let mut r = Runner::new("/usr/bin/true");

		assert_eq!(r.run(counters), Err(Error::MockError));
		assert_eq!(counters[0].value, Some(0));
	}

	#[test]
	fn set_ok() {
		let mut mock = mock_event::new();
		mock.set_ret = Some(42);

		let counters = &mut vec![Box::new(mock)];
		let mut r = Runner::new("/usr/bin/true");

		assert!(r.run(counters).is_ok());
		assert_eq!(counters[0].value, Some(0));
	}
}
