use std::process;
use std::ffi::{CString, NulError};
use std::os::unix::io::RawFd;

use nix::unistd::{close, execvp, fork, read, write, ForkResult, Pid};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::sys::socket::{socketpair, AddressFamily, SockFlag, SockType};
use nix::sys::signal::{kill, Signal};

/// `BAD_EXEC` is returned when the child fails to execute the target process.
const BAD_EXEC: i32 = 42;

/// Exec executes the target process, returning a Child process that blocks for
/// a start signal.
pub struct Exec {
	target: CString,
	args: Vec<CString>,
}

/// Child represents the forked process that is blocking for the start signal.
///
/// The forked child process will not call exec() until run is called.
pub struct Child {
	socket: RawFd,
	pid: Option<Pid>,
}

/// Exec handles the execution of a child process.
///
/// To obtain the most accurate results (and to avoid an inherently racy
/// initialisation in the Runner) Exec forks a process and blocks, waiting for a
/// signal from the parent process - this allows the parent to observe the newly
/// created child's PID to which the Runner can attach the counters to, and then
/// signals the child when initialisation is complete. The child process then
/// executes the target by calling exec().
impl Exec {
	pub fn new(target: &str) -> Result<Self, NulError> {
		Ok(Exec {
			target: CString::new(target)?,
			args: Vec::new(),
		})
	}

	/// Execute a child process.
	///
	/// If the target process fails to run, the child process exists with
	/// an exit code of `BAD_EXEC`.
	pub fn exec(self) -> Child {
		let mut c = Child {
			pid: None,
			socket: 0,
		};

		// Create socket pair to signal the child
		let (parent_sock, child_sock) = socketpair(
			AddressFamily::Unix,
			SockType::Stream,
			None,
			SockFlag::empty(),
		).unwrap();

		let mut buf = [0];
		match fork() {
			Ok(ForkResult::Parent { child, .. }) => {
				c.socket = parent_sock;
				c.pid = Some(child);
			}
			Ok(ForkResult::Child) => {
				let _ = close(parent_sock);

				// Wait for the "start" signal and go
				let _ = read(child_sock, &mut buf);
				let _ = close(child_sock);
				let _ = execvp(&self.target, &self.args);
				process::exit(BAD_EXEC);
			}
			Err(_) => panic!("fork failed"),
		};

		c
	}

	/// Append args to the list of arguments passed to the target process.
	pub fn args(mut self, args: &[&str]) -> Result<Self, NulError> {
		for arg in args.iter() {
			self.args.push(CString::new(*arg)?);
		}
		Ok(self)
	}
}

impl Child {
	pub fn pid(&self) -> Option<u32> {
		// Horrible hack to get the raw pid out - nix::unistd::Pid is a
		// tuple with a private field containing the libc::pid_t with no
		// accessors.
		self.pid
			.map(|pid| format!("{}", pid).parse::<u32>().unwrap())
	}

	pub fn run(self) -> Option<i32> {
		self.pid?;

		// Send the "start" signal to the child
		let _ = write(self.socket, b"!");

		// Block while it runs
		match waitpid(self.pid, None) {
			Ok(WaitStatus::Exited(_, BAD_EXEC)) => None,
			Ok(WaitStatus::Exited(_, val)) => Some(val),
			_ => None,
		}
	}
}

impl Drop for Child {
	/// When a Child is dropped, the child PID is sent a `SIGTERM` signal (if
	/// still alive).
	fn drop(&mut self) {
		let _ = close(self.socket);

		if let Some(pid) = self.pid {
			let _ = kill(pid, Signal::SIGTERM);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn success() {
		let c = Exec::new("/usr/bin/true")
			.unwrap()
			.args(&vec!["test"])
			.unwrap()
			.exec();

		assert!(c.pid().is_some());
		assert_eq!(c.run(), Some(0));
	}

	#[test]
	fn bad_exit_status() {
		let c = Exec::new("/usr/bin/false")
			.unwrap()
			.args(&vec!["test"])
			.unwrap()
			.exec();

		assert!(c.pid().is_some());
		assert_eq!(c.run(), Some(1));
	}

	#[test]
	fn missing_binary() {
		let c = Exec::new("not-a-thing")
			.unwrap()
			.args(&vec!["test"])
			.unwrap()
			.exec();

		assert!(c.pid().is_some());
		assert_eq!(c.run(), None);
	}
}
