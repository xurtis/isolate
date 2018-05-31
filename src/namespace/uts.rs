use libc::{
	c_int,
	CLONE_NEWUTS,
};

use ::error::*;
use ::Child;
use super::Namespace;

/// Unix Timesharing System (UTS)
///
/// The Unix Timesharing System provides the domain and hostname of the system.
/// This is given its own namespace and can be changed within that namespace.
pub struct Uts {}

impl Uts {
	/// Configure a new UTS namespace for creation.
	pub fn new() -> Uts {
		Uts {}
	}
}

impl Namespace for Uts {
	fn clone_flag(&self) -> c_int {
		CLONE_NEWUTS
	}
}
