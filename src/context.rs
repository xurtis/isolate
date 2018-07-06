use std::ops::{Deref, DerefMut, BitOr};
use std::ptr::{NonNull, self};
use std::slice;
use std::panic::{PanicInfo, self};
use std::process::abort;

use libc::{size_t, c_int, off_t, c_void, SIGCHLD};
use nix::sched::{clone, CloneFlags};
use nix::sys::mman::{mmap, MapFlags, ProtFlags};
use nix::sys::signal::{kill, SIGSTOP, SIGCONT};
use nix::unistd::{getpid, sysconf, Pid, SysconfVar};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};

use error::*;
use ::namespace::Namespace;

#[derive(Debug, Clone)]
enum Location {
    Parent,
    Child,
}

/// A process execution context constructed of namespaces.
#[derive(Debug, Clone)]
pub struct Context {
    namespaces: Vec<Box<Namespace>>,
    location: Location,
}

impl Context {
    /// Create a new empty context.
    ///
    /// This will effectively be configured to be a context that executes code
    /// in a new process with the same privileges as the parent.
    pub fn new() -> Context {
        Context {
            namespaces: Vec::new(),
            location: Location::Parent,
        }
    }

    /// Add a namespace configuration to the context.
    pub fn with<N>(mut self, ns: N) -> Context
    where
        N: 'static + Namespace
    {
        self.push(ns);
        self
    }

    /// Push a new configuration into the context.
    pub fn push<N>(&mut self, ns: N)
    where
        N: 'static + Namespace
    {
        self.namespaces.push(Box::new(ns));
    }


    /// Create a process in a new private address space.
    ///
    /// The address space is copied and no references are shared.
    pub fn exec_private<C>(self, f: C) -> Result<Child>
    where
        C: FnMut() + Send + 'static
    {
        self.exec(f, Share::Private)
    }

    /// Create and enter the context, running the given function.
    ///
    /// The address space is shared with the child and the calling process
    /// allowing shared access to globals, etc.
    pub fn exec_shared<C>(self, f: C) -> Result<Child>
    where
        C: FnMut() + Send + 'static
    {
        self.exec(f, Share::Shared)
    }

    /// Execute a child with a given function.
    fn exec<C>(self, child: C, shared: Share) -> Result<Child>
    where
        C: FnMut() + Send + 'static
    {
        let flags = vec![self.clone_flag(), shared.addrspace()]
            .into_iter()
            .flat_map(|s| s.into_iter())
            .collect();

        // Send the closure to a new process.
        let child = Child::from_tid(clone(
            self.wrap(child),
            Stack::new(shared)?.region_mut(),
            flags,
            Some(SIGCHLD),
        )?);

        self.configure(&child)?;
        child.cont()?;

        Ok(child)
    }

    /// Initialise the child process.
    fn wrap<C>(&self, mut child: C) -> Box<FnMut() -> isize + Send + 'static>
    where
        C: FnMut() + Send + 'static
    {
        let mut context = self.child_copy();
        Box::new(move || {
            panic::set_hook(Box::new(Context::panic_hook));

            kill(getpid(), SIGSTOP).expect("Stop child before running");
            if let Err(err) = context.internal_config() {
                eprintln!(
                    "Failed to configure context internally: {}",
                    err
                );
                abort();
            }
            child();
            0
        })
    }

    /// Create a copy of the context for the child.
    fn child_copy(&self) -> Context {
        let mut copy = self.clone();
        copy.location = Location::Child;
        copy
    }

    /// A hook to catch panics within a child.
    fn panic_hook(info: &PanicInfo) {
        eprintln!("Context panic: {}", info);
        abort();
    }

    /// Configure the context of the child externally.
    fn configure(&self, child: &Child) -> Result<()> {
        for namespace in &self.namespaces {
            namespace.external_config(child)?;
        }

        Ok(())
    }
}

impl Namespace for Context {
    fn clone_flag(&self) -> Option<CloneFlags> {
        Some(
            self.namespaces.iter()
                .flat_map(|n| n.clone_flag())
                .collect()
        )
    }

    fn prepare(&self) -> Result<()> {
        for ns in &self.namespaces {
            ns.prepare()?;
        }

        Ok(())
    }

    fn internal_config(&mut self) -> Result<()> {
        for ns in &mut self.namespaces {
            ns.internal_config()?;
        }

        Ok(())
    }

    fn internal_cleanup(&mut self) {
        for ns in self.namespaces.iter_mut().rev() {
            ns.internal_cleanup();
        };
    }

    fn external_config(&self, child: &Child) -> Result<()> {
        for ns in &self.namespaces {
            ns.external_config(child)?;
        }

        Ok(())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        if let Location::Child = self.location {
            self.internal_cleanup();
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Share {
    Shared,
    Private
}

impl Share {
    fn map(&self) -> MapFlags {
        match *self {
            Share::Shared => MapFlags::MAP_SHARED,
            Share::Private => MapFlags::MAP_PRIVATE,
        }
    }

    fn addrspace(&self) -> Option<CloneFlags> {
        match *self {
            Share::Private => None,
            Share::Shared => Some(CloneFlags::CLONE_VM),
        }
    }
}

struct Stack {
    start: NonNull<u8>,
    size: usize,
}

impl Stack {
    const PAGES: size_t = 2 * 1024;
    const NO_FILE: c_int = -1;
    const NO_OFFSET: off_t = 0;

    fn new(share: Share) -> Result<Stack> {
        let prot = ProtFlags::PROT_WRITE | ProtFlags::PROT_READ;
        let flags =
            share.map() |
            MapFlags::MAP_ANONYMOUS |
            MapFlags::MAP_STACK;

        let size = Stack::PAGES * sysconf(SysconfVar::PAGE_SIZE)?
            .expect("Getting page size") as size_t;

        let address = unsafe {
            mmap(
                ptr::null_mut(),
                size,
                prot,
                flags,
                Stack::NO_FILE,
                Stack::NO_OFFSET
            )
        }?;

        Stack::from_ptr(address as *mut c_void, size)
    }

    fn from_ptr(ptr: *mut c_void, size: usize) -> Result<Stack> {
        match ptr as isize {
            -1 | 0 => Err(ErrorKind::StackAllocation.into()),
            ptr => unsafe {
                Ok(Stack {
                    start: NonNull::new_unchecked(ptr as *mut u8),
                    size: size,
                })
            },
        }
    }

    fn region_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self.start.as_ptr(), self.size)
        }
    }

    fn region(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.start.as_ptr(), self.size)
        }
    }
}

impl Deref for Stack {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.region()
    }
}

impl DerefMut for Stack {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.region_mut()
    }
}

/// The child thread that has been started in the context.
#[derive(Debug)]
pub struct Child(Pid);

impl Child {
    fn from_tid(tid: Pid) -> Child {
        Child(tid)
    }

    /// Wait for a the child process to exit.
    pub fn wait(self) -> Result<WaitStatus> {
        Ok(waitpid(self.pid(), None)?)
    }

    /// Get the PID of the child process.
    pub fn pid(&self) -> Pid {
        self.0.clone()
    }

    /// Tell the child to continue execution.
    fn cont(&self) -> Result<()> {
        waitpid(self.pid(), Some(WaitPidFlag::WSTOPPED))?;
        Ok(kill(self.0, SIGCONT)?)
    }
}
