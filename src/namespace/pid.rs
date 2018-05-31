use libc::{
	CLONE_NEWPID,
	c_int,
};

use ::error::*;
use ::Child;
use super::Namespace;

/// Process IDs
///
/// Process IDs are unique and specific to a PID namespace. Processes from
/// different namespaces are unable to determine any information about processes
/// in other PID namespaces.
pub struct Pid {}

impl Pid {
	/// Configure a new PID namespace to for creation.
	pub fn new() -> Pid {
		Pid {}
	}
}

impl Namespace for Pid {
	fn clone_flag(&self) -> c_int {
		CLONE_NEWPID
	}
}
