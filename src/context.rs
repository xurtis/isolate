use std::ops::Deref;
use std::ptr::{NonNull, self};

use libc::{
	SIGCONT,
	clone,
	c_int,
	kill,
	pid_t,
	waitpid,
	SIGCHLD,
	c_void,
	MAP_SHARED,
	MAP_PRIVATE,
	size_t,
	off_t,
	PROT_WRITE,
	PROT_READ,
	MAP_ANONYMOUS,
	MAP_STACK,
	_SC_PAGE_SIZE,
	sysconf,
	mmap,
	SIGSTOP,
	getpid,
	EXIT_SUCCESS,
};

use error::*;
use ::namespace::Namespace;

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
				self.clone_flag() | SIGCHLD,
				Box::into_raw(close) as *mut c_void,
			))
		}?;

		self.configure(&child)?;
		child.cont()?;

		Ok(child)
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
	fn clone_flag(&self) -> c_int {
		self.namespaces.iter().fold(0, |f, n| f | n.clone_flag())
	}

	fn prepare(&self) -> Result<()> {
		for ns in &self.namespaces {
			ns.prepare()?;
		}

		Ok(())
	}

	fn internal_config(&self) -> Result<()> {
		for ns in &self.namespaces {
			ns.internal_config()?;
		}

		Ok(())
	}

	fn external_config(&self, child: &Child) -> Result<()> {
		for ns in &self.namespaces {
			ns.external_config(child)?;
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
			Share::Shared => MAP_SHARED,
			Share::Private => MAP_PRIVATE,
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
	let prot = PROT_WRITE | PROT_READ;
	let flags =
		share.map() |
		MAP_ANONYMOUS |
		MAP_STACK;

	unsafe {
		let size = STACK_PAGES * sysconf(_SC_PAGE_SIZE) as size_t;
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
		if kill(getpid(), SIGSTOP) != 0 {
			panic!("Could not stop child before running")
		}
	}

	let close: Box<fn()> = unsafe {
		Box::from_raw(closure as *mut fn())
	};
	close();
	return EXIT_SUCCESS;
}

/// The child thread that has been started in the context.
pub struct Child(pid_t);

impl Child {
	fn from_tid(tid: c_int) -> Result<Child> {
		match tid {
			-1 => Err(errno!(Clone)),
			tid => Ok(Child(tid)),
		}
	}

	/// Wait for a the child process to exit.
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

	/// Get the PID of the child process.
	pub fn pid(&self) -> i32 {
		self.0
	}

	/// Tell the child to continue execution.
	fn cont(&self) -> Result<()> {
		match unsafe { kill(self.pid(), SIGCONT) } {
			-1 => Err(errno!(ChildContinue)),
			_ => Ok(())
		}
	}
}
