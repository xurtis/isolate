//! Enter a jailed shell.


extern crate isolate;

use std::process::Command;

use isolate::*;

fn main() -> isolate::Result<()> {
	let user_ns = namespace::User::new()
		.map_root_user()
		.map_root_group();
	let context = Context::new()
		.with(user_ns)
		.with(namespace::Mount::new());

	let child = context.exec(shell)?;
	child.wait()
}

fn shell() {
	Command::new("/bin/sh").status().unwrap();
}
