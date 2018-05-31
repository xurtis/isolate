//! Enter a jailed shell.


extern crate isolate;

use std::process::Command;

use isolate::*;
use isolate::namespace::*;

fn main() -> isolate::Result<()> {
	let user_ns = User::new()
		.map_root_user()
		.map_root_group();

	let procfs = Mount::recursive_bind("/proc", "proc")?
		.make_target_dir()
		.unmount();
	let dev = Mount::recursive_bind("/dev", "dev")?
		.make_target_dir()
		.unmount();
	let sys = Mount::recursive_bind("/sys", "sys")?
		.make_target_dir()
		.unmount();

	let context = Context::new()
		.with(user_ns)
		.with(procfs)
		.with(dev)
		.with(sys);

	let child = context.exec_private(shell)?;
	child.wait()
}

fn shell() {
	Command::new("/bin/sh").status().unwrap();
}
