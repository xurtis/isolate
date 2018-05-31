use libc::{
	CLONE_NEWNS,
	c_int,
};

use ::error::*;
use ::Child;
use super::Namespace;

/// Mounts
///
/// Each process exists in a particular mount namespace which specifies which
/// *additional* mount mappings exist over the base file-system. This means that
/// if a set of processes exists in a separate mount namespace, they can have
/// directory mounts applied that are not visible to processes in any other
/// namespace. These processes are also unable to affect the mounts on external
/// namespaces.
///
/// Mount namespaces are copied on creation.
pub struct Mount {}

impl Mount {
	/// Configure a new mount namespace for creation.
	pub fn new() -> Mount {
		Mount {}
	}
}

impl Namespace for Mount {
	fn clone_flag(&self) -> c_int {
		CLONE_NEWNS
	}
}
