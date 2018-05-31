use std::fs::create_dir_all;
use std::ptr;

use libc::{
	CLONE_NEWNS,
	MS_BIND,
	MS_DIRSYNC,
	MS_MANDLOCK,
	MS_MOVE,
	MS_NOATIME,
	MS_NODEV,
	MS_NODIRATIME,
	MS_NOEXEC,
	MS_NOSUID,
	MS_PRIVATE,
	MS_RDONLY,
	MS_REC,
	MS_RELATIME,
	MS_REMOUNT,
	MS_SHARED,
	MS_SILENT,
	MS_SLAVE,
	MS_STRICTATIME,
	MS_SYNCHRONOUS,
	MS_UNBINDABLE,
	c_int,
	c_ulong,
	c_char,
	mount,
};

// TODO: MS_LAZYATIME (not currently in libc)

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
#[derive(Clone, Debug)]
pub struct Mount {
	mounts: Vec<DirMount>,
}

impl Mount {
	/// Configure a new mount namespace for creation.
	///
	/// This will create a duplicate mount space of the parent process.
	pub fn new() -> Mount {
		Default::default()
	}

	/// Add a mountpoint to be added once the namespace is entered.
	pub fn mount(mut self, mount: DirMount) -> Mount {
		self.mounts.push(mount);
		self
	}
}

impl Default for Mount {
	fn default() -> Mount {
		Mount {
			mounts: Vec::new(),
		}
	}
}

impl Namespace for Mount {
	fn clone_flag(&self) -> c_int {
		CLONE_NEWNS
	}

	fn internal_config(&self) -> Result<()> {
		for mount in &self.mounts {
			mount.mount()?;
		}

		Ok(())
	}
}

/// An entry in the mount table.
///
/// This is simply a wrapper for `mount(2)` in Linux.
///
/// ```rust
/// DirMount::bind("/proc", "/tmp/jail/proc").read_only().mount();
/// ```
#[derive(Clone, Debug)]
pub struct DirMount {
	src: Option<String>,
	target: String,
	fstype: Option<String>,
	flags: c_ulong,
	mk_target: bool,
}

macro_rules! mount_flag {
	($self:ident $(| $flag:ident).*) => (
		DirMount {
			flags: $self.flags $(| $flag)*,
			..
			$self
		}
	)
}

impl DirMount {
	/// Create a new mount from `src` to `target`.
	///
	/// The file system type must be explicitly provided as along with the
	/// target and the source.
	///
	/// ```rust
	/// DirMount::new("/dev/sda1", "/mnt", "ext4").mount();
	/// ```
	pub fn new(src: &str, target: &str, fstype: &str) -> DirMount {
		DirMount {
			src: Some(src.to_owned()),
			target: target.to_owned(),
			fstype: Some(fstype.to_owned()),
			flags: 0,
			mk_target: false,
		}
	}

	/// Update the mount flags on an existing mount.
	///
	/// ```rust
	/// DirMount::remount("/home").read_only().mount();
	/// ```
	pub fn remount(target: &str) -> DirMount {
		DirMount {
			src: None,
			target: target.to_owned(),
			fstype: None,
			flags: MS_REMOUNT,
			mk_target: false,
		}
	}

	/// Bind a directory to a new mount point.
	///
	/// ```rust
	/// DirMount::bind("/lib", "/tmp/jail/lib").mount();
	/// ```
	pub fn bind(src: &str, target: &str) -> DirMount {
		DirMount {
			src: Some(src.to_owned()),
			target: target.to_owned(),
			fstype: None,
			flags: MS_BIND,
			mk_target: false,
		}
	}


	/// Bind a directory and all mounts in its subtree to a new mount point.
	///
	/// ```rust
	/// DirMount::recursive_bind("/proc", "/tmp/jail/proc").mount();
	/// ```
	pub fn recursive_bind(src: &str, target: &str) -> DirMount {
		DirMount {
			src: Some(src.to_owned()),
			target: target.to_owned(),
			fstype: None,
			flags: MS_BIND | MS_REC,
			mk_target: false,
		}
	}

	/// Update an existing mount point to be _shared_.
	///
	/// This ensures that _mount_ and _unmount_ events that occur within the
	/// subtree of this mount point may propogate to peer mounts within the
	/// namespace.
	pub fn shared(target: &str) -> DirMount {
		DirMount {
			src: None,
			target: target.to_owned(),
			fstype: None,
			flags: MS_SHARED,
			mk_target: false,
		}
	}


	/// Update an existing mount point to be _private_.
	///
	/// This ensures that _mount_ and _unmount_ events that occur within the
	/// subtree of this mountpoint will not propogate to peer mounts within the
	/// namespace.
	pub fn private(target: &str) -> DirMount {
		DirMount {
			src: None,
			target: target.to_owned(),
			fstype: None,
			flags: MS_PRIVATE,
			mk_target: false,
		}
	}

	/// Update an existing mount point to be a _slave_.
	///
	/// This ensures that _mount_ and _unmount_ events never propogate out of
	/// the subtree from the mount point but events will propogate into it.
	pub fn slave(target: &str) -> DirMount {
		DirMount {
			src: None,
			target: target.to_owned(),
			fstype: None,
			flags: MS_SLAVE,
			mk_target: false,
		}
	}

	/// Update an existing mount point to be a _unbindable_.
	///
	/// This has the same effect as [`DirMount::private`](#method.provate) but
	/// also ensures the mount point, and its children, can't be mounted as a
	/// bind. Recursive bind mounts will simply have _unbindable_ mounts pruned.
	pub fn unbindable(target: &str) -> DirMount {
		DirMount {
			src: None,
			target: target.to_owned(),
			fstype: None,
			flags: MS_UNBINDABLE,
			mk_target: false,
		}
	}

	/// Move a mount from an existing mount point to a new mount point.
	pub fn relocate(src: &str, target: &str) -> DirMount {
		DirMount {
			src: Some(src.to_owned()),
			target: target.to_owned(),
			fstype: None,
			flags: MS_MOVE,
			mk_target: false,
		}
	}

	/// This simply takes a non-bind mount and adds the bind flag.
	///
	/// This is useful if remounting bind mounts.
	pub fn as_bind(self) -> DirMount {
		mount_flag!(self | MS_BIND)
	}

	/// Make directory changes on this filesystem synchronous.
	pub fn synchronous_directories(self) -> DirMount {
		mount_flag!(self | MS_DIRSYNC)
	}

	/// Reduce on-disk updates of inode timestamps (atime, mtime, ctime) by
	/// maintaining these changes only in memory.  The on-disk timestamps are
	/// updated only when:
	///
	/// * the inode needs to be updated for some change unrelated to file
	///   timestamps;
	/// * the application employs fsync(2), syncfs(2), or sync(2);
	/// * an undeleted inode is evicted from memory; or
	/// * more than 24 hours have passed since the inode was written to disk.
	///
	/// This mount option significantly reduces writes needed to update the
	/// inode's timestamps, especially mtime and atime.  However, in the event
	/// of a system crash, the atime  and mtime fields on disk might be out of
	/// date by up to 24 hours.
	///
	/// Examples  of  workloads  where  this  option  could be of significant
	/// benefit include frequent random writes to preallocated files, as well as
	/// cases where the MS_STRICTATIME mount option is also enabled.
	#[cfg(not(any))]
	pub fn lazy_access_time(self) -> DirMount {
		// mount_flag!(self | MS_LAZYATIME)
		unimplemented!()
	}

	/// Do not update access times for (all types of) files on this mount.
	pub fn mandatory_locking(self) -> DirMount {
		mount_flag!(self | MS_MANDLOCK)
	}

	/// Do not allow access to devices (special files) on this mount.
	pub fn no_access_time(self) -> DirMount {
		mount_flag!(self | MS_NOATIME)
	}

	/// Do not allow access to devices (special files) on this mount.
	pub fn no_devices(self) -> DirMount {
		mount_flag!(self | MS_NODEV)
	}

	/// Do not update access times for directories on this mount.
	pub fn no_directory_access_time(self) -> DirMount {
		mount_flag!(self | MS_NODIRATIME)
	}

	/// Do not allow programs to be executed from this mount.
	pub fn no_execute(self) -> DirMount {
		mount_flag!(self | MS_NOEXEC)
	}

	/// Do not honor set-user-ID and set-group-ID bits or file capabilities when
	/// executing programs from this mount.
	pub fn no_setuid(self) -> DirMount {
		mount_flag!(self | MS_NOSUID)
	}

	/// Mount read-only.
	pub fn read_only(self) -> DirMount {
		mount_flag!(self | MS_RDONLY)
	}

	/// Update access time on files only if newer than the modification time.
	///
	/// When a file on this mount is accessed, update the file's last
	/// access time (atime) only if the current value of atime is less than or
	/// equal to the file's last modification time (mtime) or last status change
	/// time (ctime).
	///
	/// This option is useful for programs, such as mutt(1), that need to know
	/// when a file has been read since it was last modified.
	pub fn relative_access_time(self) -> DirMount {
		mount_flag!(self | MS_RELATIME)
	}

	/// Suppress the display of certain warning messages in the kernel log.
	pub fn silent(self) -> DirMount {
		mount_flag!(self | MS_SILENT)
	}

	/// Always update the last access time.
	pub fn strict_access_time(self) -> DirMount {
		mount_flag!(self | MS_STRICTATIME)
	}

	/// Make writes on this mount synchronous.
	pub fn synchronous(self) -> DirMount {
		mount_flag!(self | MS_SYNCHRONOUS)
	}

	/// If the target directory does not exist, create it.
	pub fn make_target_dir(self) -> DirMount {
		DirMount {
			mk_target: true,
			..
			self
		}
	}

	/// Mount using the given specification.
	pub fn mount(&self) -> Result<()> {
		if self.mk_target {
			create_dir_all(&self.target)?;
		}

		unsafe {
			match mount(
				self.src(),
				self.target(),
				self.fstype(),
				self.flags,
				ptr::null()
			) {
				-1 => Err(errno!(Mount, self.clone())),
				_ => Ok(())
			}
		}
	}

	fn src(&self) -> *const c_char {
		match self.src {
			Some(ref src) => src.as_ptr() as *const c_char,
			None => ptr::null(),
		}
	}

	fn target(&self) -> *const c_char {
		self.target.as_ptr() as *const c_char
	}

	fn fstype(&self) -> *const c_char {
		match self.fstype {
			Some(ref fstype) => fstype.as_ptr() as *const c_char,
			None => ptr::null(),
		}
	}
}
