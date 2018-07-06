use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use nix::NixPath;
use nix::mount::{mount, umount, MsFlags};

// TODO: MS_LAZYATIME (not currently in libc)

use ::error::*;
use super::prelude::*;

/// A new mount namespace with no immediate mounts.
///
/// Mount namespaces are copied on creation.
#[derive(Clone, Debug)]
pub struct EmptyMount();

impl EmptyMount {
    /// Configure a new mount namespace for creation.
    ///
    /// This will create a duplicate mount space of the parent process.
    pub fn new() -> EmptyMount {
        EmptyMount()
    }
}

impl Namespace for EmptyMount {
    fn clone_flag(&self) -> Option<CloneFlags> {
        Some(CloneFlags::CLONE_NEWNS)
    }
}

/// A new mountpoint within a mount namespace.
///
/// Each process exists in a particular mount namespace which specifies which
/// *additional* mount mappings exist over the base file-system. This means that
/// if a set of processes exists in a separate mount namespace, they can have
/// directory mounts applied that are not visible to processes in any other
/// namespace. These processes are also unable to affect the mounts on external
/// namespaces.
///
/// This is simply a wrapper for `mount(2)` in Linux.
///
/// ```rust
/// DirMount::bind("/proc", "/tmp/jail/proc").read_only().mount();
/// ```
#[derive(Clone, Debug)]
pub struct Mount {
    src: Option<PathBuf>,
    target: PathBuf,
    fstype: Option<PathBuf>,
    flags: Option<MsFlags>,
    mk_target: bool,
    umount: bool,
    mounted: Option<PathBuf>,
}

impl Mount {
    /// Create a new mount from `src` to `target`.
    ///
    /// The file system type must be explicitly provided as along with the
    /// target and the source.
    ///
    /// ```rust
    /// Mount::new("/dev/sda1", "/mnt", "ext4").mount();
    /// ```
    pub fn new<P: AsRef<Path>>(src: P, target: P, fstype: P) -> Mount {
        Mount {
            src: Some(src.as_ref().to_owned()),
            target: target.as_ref().to_owned(),
            fstype: Some(fstype.as_ref().to_owned()),
            flags: None,
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }

    /// Update the mount flags on an existing mount.
    ///
    /// ```rust
    /// Mount::remount("/home").read_only().mount();
    /// ```
    pub fn remount<P: AsRef<Path>>(target: P) -> Mount {
        Mount {
            src: None,
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_REMOUNT),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }

    /// Bind a directory to a new mount point.
    ///
    /// ```rust
    /// Mount::bind("/lib", "/tmp/jail/lib").mount();
    /// ```
    pub fn bind<P: AsRef<Path>>(src: P, target: P) -> Mount {
        Mount {
            src: Some(src.as_ref().to_owned()),
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_BIND),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }


    /// Bind a directory and all mounts in its subtree to a new mount point.
    ///
    /// ```rust
    /// Mount::recursive_bind("/proc", "/tmp/jail/proc").mount();
    /// ```
    pub fn recursive_bind<P: AsRef<Path>>(src: P, target: P) -> Mount {
        Mount {
            src: Some(src.as_ref().to_owned()),
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_BIND | MsFlags::MS_REC),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }

    /// Update an existing mount point to be _shared_.
    ///
    /// This ensures that _mount_ and _unmount_ events that occur within the
    /// subtree of this mount point may propogate to peer mounts within the
    /// namespace.
    pub fn shared<P: AsRef<Path>>(target: P) -> Mount {
        Mount {
            src: None,
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_SHARED),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }


    /// Update an existing mount point to be _private_.
    ///
    /// This ensures that _mount_ and _unmount_ events that occur within the
    /// subtree of this mountpoint will not propogate to peer mounts within the
    /// namespace.
    pub fn private<P: AsRef<Path>>(target: P) -> Mount {
        Mount {
            src: None,
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_PRIVATE),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }

    /// Update an existing mount point to be a _slave_.
    ///
    /// This ensures that _mount_ and _unmount_ events never propogate out of
    /// the subtree from the mount point but events will propogate into it.
    pub fn slave<P: AsRef<Path>>(target: P) -> Mount {
        Mount {
            src: None,
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_SLAVE),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }

    /// Update an existing mount point to be a _unbindable_.
    ///
    /// This has the same effect as [`Mount::private`](#method.provate) but
    /// also ensures the mount point, and its children, can't be mounted as a
    /// bind. Recursive bind mounts will simply have _unbindable_ mounts pruned.
    pub fn unbindable<P: AsRef<Path>>(target: P) -> Mount {
        Mount {
            src: None,
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_UNBINDABLE),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }

    /// Move a mount from an existing mount point to a new mount point.
    pub fn relocate<P: AsRef<Path>>(src: P, target: P) -> Mount {
        Mount {
            src: Some(src.as_ref().to_owned()),
            target: target.as_ref().to_owned(),
            fstype: None,
            flags: Some(MsFlags::MS_MOVE),
            mk_target: false,
            umount: false,
            mounted: None,
        }
    }
}

impl Mount {
    fn add_flag(mut self, flag: MsFlags) -> Mount {
        let current: MsFlags = self.flags.into_iter().collect();
        self.flags = Some(current | flag);
        self
    }

    /// This simply takes a non-bind mount and adds the bind flag.
    ///
    /// This is useful if remounting bind mounts.
    pub fn as_bind(self) -> Mount {
        self.add_flag(MsFlags::MS_BIND)
    }

    /// Make directory changes on this filesystem synchronous.
    pub fn synchronous_directories(self) -> Mount {
        self.add_flag(MsFlags::MS_DIRSYNC)
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
    pub fn lazy_access_time(self) -> Mount {
        // self.add_flag(MsFlags::MS_LAZYATIME);
        self
    }

    /// Do not update access times for (all types of) files on this mount.
    pub fn mandatory_locking(self) -> Mount {
        self.add_flag(MsFlags::MS_MANDLOCK)
    }

    /// Do not allow access to devices (special files) on this mount.
    pub fn no_access_time(self) -> Mount {
        self.add_flag(MsFlags::MS_NOATIME)
    }

    /// Do not allow access to devices (special files) on this mount.
    pub fn no_devices(self) -> Mount {
        self.add_flag(MsFlags::MS_NODEV)
    }

    /// Do not update access times for directories on this mount.
    pub fn no_directory_access_time(self) -> Mount {
        self.add_flag(MsFlags::MS_NODIRATIME)
    }

    /// Do not allow programs to be executed from this mount.
    pub fn no_execute(self) -> Mount {
        self.add_flag(MsFlags::MS_NOEXEC)
    }

    /// Do not honor set-user-ID and set-group-ID bits or file capabilities when
    /// executing programs from this mount.
    pub fn no_setuid(self) -> Mount {
        self.add_flag(MsFlags::MS_NOSUID)
    }

    /// Mount read-only.
    pub fn read_only(self) -> Mount {
        self.add_flag(MsFlags::MS_RDONLY)
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
    pub fn relative_access_time(self) -> Mount {
        self.add_flag(MsFlags::MS_RELATIME)
    }

    /// Suppress the display of certain warning messages in the kernel log.
    pub fn silent(self) -> Mount {
        self.add_flag(MsFlags::MS_SILENT)
    }

    /// Always update the last access time.
    pub fn strict_access_time(self) -> Mount {
        self.add_flag(MsFlags::MS_STRICTATIME)
    }

    /// Make writes on this mount synchronous.
    pub fn synchronous(self) -> Mount {
        self.add_flag(MsFlags::MS_SYNCHRONOUS)
    }

    /// If the target directory does not exist, create it.
    pub fn make_target_dir(mut self) -> Mount {
        self.mk_target = true;
        self
    }

    /// Unmount the target when finished.
    pub fn unmount(mut self) -> Mount {
        self.umount = true;
        self
    }
}

impl Mount {
    /// Mount using the given specification.
    pub fn mount(&mut self) -> Result<()> {
        let target = self.target.with_nix_path(|s| {
            Path::new(s.to_string_lossy().as_ref()).to_path_buf()
        })?;

        if self.mk_target {
            create_dir_all(&target)?;
        }

        let data: Option<&PathBuf> = None;

        mount(
            self.src.as_ref(),
            &self.target,
            self.fstype.as_ref(),
            self.flags.into_iter().collect(),
            data
        )?;

        self.mounted = Some(target.canonicalize()?);

        Ok(())
    }
}

impl Namespace for Mount {
    fn clone_flag(&self) -> Option<CloneFlags> {
        Some(CloneFlags::CLONE_NEWNS)
    }
}

impl Split for Mount {
    type ExternalConfig = ();
    type InternalConfig = Self;

    fn split(self) -> ((), Mount) {
        ((), self)
    }
}

impl InternalConfig for Mount {
    fn configure(&mut self) -> Result<()> {
        self.mount()
    }

    fn cleanup(&mut self) -> Result<()> {
        match (&self.mounted, self.umount) {
            (Some(ref path), true) => Ok(umount(path)?),
            _ => Ok(())
        }
    }
}
