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

    let mut context = Context::new()
        .private()
        .with(user_ns)
        .with(Pid::new())
        .with(ControlGroup::new())
        .with(Ipc::new())
        .with(Mount::new("proc", "/tmp/jail/proc", "proc").make_target_dir())
        .with(Mount::new("tmp", "/tmp/jail/tmp", "tmpfs").make_target_dir());

    let binds = &[
        ("/dev",   "/tmp/jail/dev"),
        ("/sys",   "/tmp/jail/sys"),
        ("/bin",   "/tmp/jail/bin"),
        ("/lib",   "/tmp/jail/lib"),
        ("/lib64", "/tmp/jail/lib64"),
        ("/usr",   "/tmp/jail/usr"),
        ("/etc",   "/tmp/jail/etc"),
        (".",      "/tmp/jail/root")
    ];

    for (src, dest) in binds {
        context.push(Mount::recursive_bind(src, dest).make_target_dir());
    }

    let child = context.spawn(|| {
        Command::new("/sbin/chroot").args(&["/tmp/jail", "/bin/sh"]).status().unwrap();
    })?;

    child.wait()?;
    Ok(())
}
