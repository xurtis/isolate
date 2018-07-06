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
use ::namespace::{
    BoxedSplit,
    ExternalConfig,
    InternalConfig,
    Namespace,
    Split,
    SplitBox,
};

/// A process execution context constructed of namespaces.
#[derive(Debug)]
pub struct Context {
    namespaces: Vec<Box<BoxedSplit>>,
    share: Share,
}

/// The collection of external configrations of a context.
#[derive(Debug)]
pub struct ContextOuter {
    configs: Vec<Box<ExternalConfig>>,
}

/// The collection of internal configrations of a context.
#[derive(Debug)]
pub struct ContextInner {
    configs: Vec<Box<InternalConfig>>,
}

impl Context {
    /// Create a new empty context.
    ///
    /// This will effectively be configured to be a context that executes code
    /// in a new process with the same privileges as the parent.
    pub fn new() -> Context {
        Context {
            namespaces: Vec::new(),
            share: Share::Shared,
        }
    }

    /// Execute the child in a private address space.
    ///
    /// Executing the child in a private address space prevents it from modifying the address space
    /// of the parent or reading any data intorduced into the address space after the child has
    /// started executing.
    pub fn private(mut self) -> Context {
        self.share = Share::Private;
        self
    }

    /// Add a namespace configuration to the context.
    pub fn with<N>(mut self, ns: N) -> Context
    where
        N: 'static + Namespace + Split
    {
        self.push(ns);
        self
    }

    /// Push a new configuration into the context.
    pub fn push<N>(&mut self, ns: N)
    where
        N: 'static + Namespace + Split
    {
        self.namespaces.push(SplitBox::new(ns));
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

        self.prepare()?;

        let (mut external, internal) = self.split();

        // Send the closure to a new process.
        let child = Child::from_tid(clone(
            internal.wrap(child),
            Stack::new(shared)?.region_mut(),
            flags,
            Some(SIGCHLD),
        )?);

        external.configure(&child)?;
        child.cont()?;

        Ok(child)
    }
}

impl ContextInner {
    /// Initialise the child process.
    fn wrap<C>(mut self, mut child: C) -> Box<FnMut() -> isize + Send + 'static>
    where
        C: FnMut() + Send + 'static
    {
        Box::new(move || {
            panic::set_hook(Box::new(ContextInner::panic_hook));

            kill(getpid(), SIGSTOP).expect("Stop child before running");
            if let Err(err) = self.configure() {
                eprintln!(
                    "Failed to configure context internally: {}",
                    err
                );
                abort();
            }
            child();
            self.cleanup().expect("Cleaining up child");
            0
        })
    }

    /// A hook to catch panics within a child.
    fn panic_hook(info: &PanicInfo) {
        eprintln!("Context panic: {}", info);
        abort();
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
}

impl Split for Context {
    type ExternalConfig = ContextOuter;
    type InternalConfig = ContextInner;

    fn split(self) -> (ContextOuter, ContextInner) {
        let mut outer_configs = Vec::new();
        let mut inner_configs = Vec::new();

        for mut ns in self.namespaces {
            let (outer, inner) = ns.boxed_split();
            outer_configs.push(outer);
            inner_configs.push(inner);
        }

        (
            ContextOuter { configs: outer_configs },
            ContextInner { configs: inner_configs },
        )
    }
}

impl ExternalConfig for ContextOuter {
    fn configure(&mut self, child: &Child) -> Result<()> {
        for config in &mut self.configs {
            config.configure(child)?;
        }

        Ok(())
    }

    fn cleanup(&mut self, child: &Child) -> Result<()> {
        for config in &mut self.configs {
            config.cleanup(child)?;
        }

        Ok(())
    }
}

impl InternalConfig for ContextInner {
    fn configure(&mut self) -> Result<()> {
        for config in &mut self.configs {
            config.configure()?;
        }

        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        for config in &mut self.configs {
            config.cleanup()?;
        }

        Ok(())
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
