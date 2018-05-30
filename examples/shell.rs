//! Enter a jailed shell.


extern crate isolate;

use std::process::Command;

use isolate::*;

fn main() -> isolate::Result<()> {
	let context = Context::new()
		.with(Box::new(namespace::User::new()));

	let child = context.exec(shell)?;
	child.wait()
}

fn shell() {
	Command::new("/bin/sh").status().unwrap();
}
