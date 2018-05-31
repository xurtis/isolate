//! Interface for isolation.

#![warn(missing_docs)]

#[macro_use]
extern crate error_chain;
extern crate errno;
extern crate libc;

#[macro_use]
mod error;
pub mod namespace;

use std::ptr::{NonNull, self};
use std::ops::Deref;

pub use error::*;
use namespace::Namespace;

use libc::{c_void, c_int, clone, mmap, off_t, pid_t, size_t, sysconf, waitpid, kill, getpid};
use errno::errno;


/// A child thread.
pub struct Child(pid_t);

impl Child {
	fn from_tid(tid: c_int) -> Result<Child> {
		match tid {
			-1 => Err(ErrorKind::Clone(errno()).into()),
			tid => Ok(Child(tid)),
		}
	}

	pub fn wait(self) -> Result<()> {
		let Child(pid) = self;

		let mut wstatus = 0;

		unsafe {
			match waitpid(pid, &mut wstatus as *mut c_int, 0) {
				-1 => Err(errno!(ChildWait)),
				_ => Ok(())
			}
		}
	}

	pub fn pid(&self) -> i32 {
		self.0
	}

	/// Tell the child to continue execution.
	fn cont(&self) -> Result<()> {
		match unsafe { kill(self.pid(), libc::SIGCONT) } {
			-1 => Err(errno!(ChildContinue)),
			_ => Ok(())
		}
	}
}

/// A process execution context constructed of namespaces.
pub struct Context {
	namespaces: Vec<Box<Namespace>>,
}

impl Context {
	/// Create a new empty context.
	///
	/// This will effectively be configured to be a context that executes code
	/// in a new process with the same privileges as the parent.
	pub fn new() -> Context {
		Context {
			namespaces: Vec::new()
		}
	}

	/// Add a namespace configuration to the context.
	pub fn with<N>(mut self, ns: N) -> Context
	where
		N: 'static + Namespace
	{
		self.namespaces.push(Box::new(ns));
		self
	}

	/// Create and enter the context, running the given function.
	pub fn exec(&self, f: fn()) -> Result<Child>
	{
		// Send the closure to a new process.
		let close = Box::new(f);
		let child = unsafe {
			Child::from_tid(clone(
				exec_closure,
				create_stack(Share::Shared)?.as_ptr(),
				self.flags() | libc::SIGCHLD,
				Box::into_raw(close) as *mut c_void,
			))
		}?;

		self.configure(&child)?;
		child.cont();

		Ok(child)
	}

	/// Get the clone flags for the context.
	fn flags(&self) -> c_int {
		self.namespaces.iter().fold(0, |f, n| f | n.clone_flag())
	}

	/// Configure the context of the child externally.
	fn configure(&self, child: &Child) -> Result<()> {
		for namespace in &self.namespaces {
			namespace.external_config(child)?;
		}

		Ok(())
	}
}

enum Share {
	Shared,
	Private
}

impl Share {
	fn map(&self) -> c_int {
		match *self {
			Share::Shared => libc::MAP_SHARED,
			Share::Private => libc::MAP_PRIVATE,
		}
	}
}

struct Stack(NonNull<c_void>);

impl Stack {
	fn from_ptr(ptr: *mut c_void, size: size_t) -> Result<Stack> {
		match ptr as isize {
			-1 | 0 => Err(errno!(StackAllocation)),
			ptr => unsafe {
				Ok(Stack(NonNull::new_unchecked(
					(ptr + size as isize) as *mut c_void
				)))
			},
		}
	}
}

impl Deref for Stack {
	type Target = NonNull<c_void>;

	fn deref(&self) -> &NonNull<c_void> {
		&self.0
	}
}

const STACK_PAGES: size_t = 2 * 1024;
const NO_FILE: c_int = -1;
const NO_OFFSET: off_t = 0;

/// Create a new stack in which to execute a child function.
fn create_stack(share: Share) -> Result<Stack> {
	let prot = libc::PROT_WRITE | libc::PROT_READ;
	let flags =
		share.map() |
		libc::MAP_ANONYMOUS |
		libc::MAP_STACK;

	unsafe {
		let size = STACK_PAGES * sysconf(libc::_SC_PAGE_SIZE) as size_t;
		Stack::from_ptr(
			mmap(ptr::null_mut(), size, prot, flags, NO_FILE, NO_OFFSET),
			size
		)
	}
}

/// Execute a function from a closure.
extern "C"
fn exec_closure(closure: *mut c_void) -> c_int {
	// Stop and wait for parent to finish config.
	unsafe {
		if kill(getpid(), libc::SIGSTOP) != 0 {
			panic!("Could not stop child before running")
		}
	}

	let close: Box<fn()> = unsafe {
		Box::from_raw(closure as *mut fn())
	};
	close();
	return libc::EXIT_SUCCESS;
}
