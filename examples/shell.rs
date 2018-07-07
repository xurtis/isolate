//! Enter a jailed shell.


extern crate isolate;
extern crate nix;

use std::process::Command;

use isolate::*;
use isolate::namespace::*;

fn main() -> isolate::Result<()> {
    let user_ns = User::new()
        .map_root_user()
        .map_root_group();

    let procfs = Mount::recursive_bind("/proc", "proc")
        .make_target_dir();
    let dev = Mount::recursive_bind("/dev", "dev")
        .make_target_dir();
    let sys = Mount::recursive_bind("/sys", "sys")
        .make_target_dir();

    let context = Context::new()
        .with(user_ns)
        .with(procfs)
        .with(dev)
        .with(sys);

    let child = context.spawn(shell)?;
    child.wait()?;

    Ok(())
}

fn shell() {
    eprintln!("Running shell...");
    Command::new("/bin/sh").status().unwrap();
}
