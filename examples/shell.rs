//! Enter a jailed shell.


extern crate isolate;

use std::process::Command;

use isolate::*;
use isolate::namespace::*;

fn main() -> isolate::Result<()> {
	let user_ns = User::new()
		.map_root_user()
		.map_root_group();

	let mount_ns = Mount::new()
		.mount(DirMount::recursive_bind("/proc", "proc")
			.make_target_dir()
		);

	let context = Context::new()
		.with(user_ns)
		.with(mount_ns);

	let child = context.exec_private(shell)?;
	child.wait()
}

fn shell() {
	Command::new("/bin/sh").status().unwrap();
}
