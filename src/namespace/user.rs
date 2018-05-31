use std::fs::OpenOptions;
use std::io::Write;

use libc::{
	CLONE_NEWUSER,
	c_int,
	getgid,
	getuid,
};

use ::error::*;
use ::Child;
use super::Namespace;

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
#[derive(Clone)]
pub struct User {
	map_root_user: bool,
	map_root_group: bool,
}

impl User {
	/// Configure a new user namespace for creation.
	pub fn new() -> User {
		Default::default()
	}

	/// Map the root user to the creator of the namespace.
	pub fn map_root_user(self) -> User {
		User {
			map_root_user: true,
			..
			self
		}
	}

	/// Map the root group to the group of the creator of the namespace.
	pub fn map_root_group(self) -> User {
		User {
			map_root_group: true,
			..
			self
		}
	}

	/// Map root to the calling user.
	fn set_root_user(&self, child: &Child) -> Result<()> {
		let uid = unsafe { getuid() };
		let mut uid_map = OpenOptions::new()
			.append(true)
			.open(format!("/proc/{}/uid_map", child.pid()))?;
		uid_map.write_all(format!("0 {} 1", uid).as_bytes())?;

		Ok(())
	}

	/// Map root group to calling user gid.
	fn set_root_group(&self, child: &Child) -> Result<()> {
		SetGroups::Deny.write(child)?;

		let gid = unsafe { getgid() };
		let mut gid_map = OpenOptions::new()
			.append(true)
			.open(format!("/proc/{}/gid_map", child.pid()))?;
		gid_map.write_all(format!("0 {} 1", gid).as_bytes())?;

		Ok(())
	}
}

impl Default for User {
	fn default() -> User {
		User {
			map_root_user: false,
			map_root_group: false,
		}
	}
}

impl Namespace for User {
	fn clone_flag(&self) -> c_int {
		CLONE_NEWUSER
	}

	fn external_config(&self, child: &Child) -> Result<()> {
		if self.map_root_user {
			self.set_root_user(child)?;
		}

		if self.map_root_group {
			self.set_root_group(child)?;
		}

		Ok(())
	}
}

/// Set the ability for the child process to change its own group mappings.
enum SetGroups {
	Allow,
	Deny
}

impl SetGroups {
	fn write(&self, child: &Child) -> Result<()> {
		let mut setgroup = OpenOptions::new()
			.append(true)
			.open(format!("/proc/{}/setgroups", child.pid()))?;
		setgroup.write_all(format!("{}", self).as_bytes())?;

		Ok(())
	}
}

impl ::std::fmt::Display for SetGroups {
	fn fmt(&self, f: &mut ::std::fmt::Formatter)
		-> ::std::result::Result<(), ::std::fmt::Error>
	{
		match *self {
			SetGroups::Allow => write!(f, "allow"),
			SetGroups::Deny => write!(f, "deny"),
		}
	}
}
