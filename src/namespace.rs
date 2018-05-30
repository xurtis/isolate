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

use libc;

use error::*;

use super::Child;

/// A trait that represents a namespace that can be created and entered.
///
/// This configures the environment before a namespace is entered and after is
/// has been entered and also provides flags for the `clone` syscall to create a
/// new instance of a given namespace.
pub trait Namespace {
	/// Get the flag needed for clone to create new namespace.
	///
	/// See `clone(2)` and `namespaces(7)` for more information.
	fn clone_flag(&self) -> libc::c_int;

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
	fn external_config(&self, child: &Child) -> Result<()> {
		Ok(())
	}
}

/// Control group namespace representation.
///
/// Each process exists in a control group. A given control group can be
/// assigned resource limits. This ensures that the total amount of resources,
/// such as CPU time and system memory, used by all of the process in the group
/// is limited.
pub struct ControlGroup {}

impl ControlGroup {
	/// Configure a new Control Group namespace for creation.
	pub fn new() -> ControlGroup {
		ControlGroup {}
	}
}

impl Namespace for ControlGroup {
	fn clone_flag(&self) -> libc::c_int {
		libc::CLONE_NEWCGROUP
	}
}

/// Inter-Process Communication.
///
/// There are two global IPC mechanisms that Linux supports; System-V IPC and
/// POSIX message queues. These, however, are globally accessible so any process
/// within an IPC namespace can connect to any other process in the same IPC
/// namespace that exposes one of these mechanisms without having any
/// information of these processes existing.
pub struct Ipc {}

impl Ipc {
	/// Configure a new IPC namespace for creation.
	pub fn new() -> Ipc {
		Ipc {}
	}
}

impl Namespace for Ipc {
	fn clone_flag(&self) -> libc::c_int {
		libc::CLONE_NEWIPC
	}
}

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
	fn clone_flag(&self) -> libc::c_int {
		libc::CLONE_NEWNS
	}
}

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
	fn clone_flag(&self) -> libc::c_int {
		libc::CLONE_NEWPID
	}
}

/// Users and Groups
///
/// User namespaces control the access privileges of UIDs and GIDs. When a new
/// user namespace is created, the initial process in that namespace is
/// considered user 0 within that namespace but has the privileges of the user
/// that created that namespace in the parent namespace.
///
/// Linux also allows for a range of UIDs and GIDs to be owned by a particular
/// user and a UID/GID namespace can be used to map one or more of these UID
/// ranges to a particular user in the global namespace. Every UID and GID will
/// have the effective capabilities of the user that created the namespace.
///
/// The root user of a user namespace can, for the purposes of that namespace
/// and child namespaces, act as user 0 for all system operations allowing for
/// operations such as mount and chroot.
pub struct User {}

impl User {
	/// Configure a new user namespace for creation.
	pub fn new() -> User {
		User {}
	}
}

impl Namespace for User {
	fn clone_flag(&self) -> libc::c_int {
		libc::CLONE_NEWUSER
	}
}

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
	fn clone_flag(&self) -> libc::c_int {
		libc::CLONE_NEWUTS
	}
}
