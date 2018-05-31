//! Namespace representations and implementations.
//!
//! Linux provides a namespaces API. This allows for a process to place itself
//! (and its children) into a context isolated from other processes in some
//! respect. Each of the following namespaces can be individually isolated.
//!
//! The following groups of items each have their own namespace defined in
//! Linux:
//!
//! * [Inter-Process Communication](struct.Ipc.html)
//! * [Networking](struct.Network.html)
//! * [Mounts](struct.Mount.html)
//! * [Process IDs](struct.Pid.html)
//! * [Users and Groups](struct.User.html)
//! * [Unix Timesharing System](struct.Uts.html)

mod control_group;
mod ipc;
mod mount;
mod network;
mod pid;
mod user;
mod uts;

use libc::{
	c_int,
};

pub use self::control_group::ControlGroup;
pub use self::ipc::Ipc;
pub use self::mount::{Mount, DirMount};
pub use self::network::Network;
pub use self::pid::Pid;
pub use self::user::User;
pub use self::uts::Uts;

use ::error::*;
use ::Child;

/// A trait that represents a namespace that can be created and entered.
///
/// This configures the environment before a namespace is entered and after is
/// has been entered and also provides flags for the `clone` syscall to create a
/// new instance of a given namespace.
pub trait Namespace: NamespaceClone {
	/// Get the flag needed for clone to create new namespace.
	///
	/// See `clone(2)` and `namespaces(7)` for more information.
	fn clone_flag(&self) -> c_int {
		0
	}

	/// Configure system prior to creating the namespace.
	///
	/// This executes all of the changes needed to be made external to the
	/// namespace in order for it to operate as desired.
	fn prepare(&self) -> Result<()> {
		Ok(())
	}

	/// Configure the system from within the namespace after creation.
	///
	/// This executes all of the changes needed to be made internal to the
	/// namespace in order for it to operate as desired.
	fn internal_config(&self) -> Result<()> {
		Ok(())
	}

	/// Configure the system from outside the namespace after creation.
	///
	/// This excutes all of the changes needed to be made externally to the
	/// namespace in order for it to operate as desired.
	fn external_config(&self, _child: &Child) -> Result<()> {
		Ok(())
	}
}

/// This is a trait that allows for a `Namespace` to clone itself into a new
/// box.
///
/// This is needed to allow for cloning of `Context`s.
pub trait NamespaceClone {
	fn box_clone(&self) -> Box<Namespace>;
}

impl<N> NamespaceClone for N
where
	N: Namespace + Clone + 'static
{
	fn box_clone(&self) -> Box<Namespace> {
		Box::new(self.clone())
	}
}

impl Clone for Box<Namespace> {
	fn clone(&self) -> Box<Namespace> {
		self.box_clone()
	}
}
