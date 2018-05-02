mod pmc_event;
mod printers;

#[cfg(debug_assertions)]
mod mock_event;
#[cfg(debug_assertions)]
pub use self::mock_event::MockEvent;

#[cfg(target_os = "freebsd")]
pub use self::pmc_event::PmcEvent;

pub use self::printers::RelativePrinter;
pub use self::printers::RSDPrinter;

pub trait Printable {
	fn name(&self) -> &str;
	fn value(&self) -> u64;
}
