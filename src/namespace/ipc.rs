use ::error::*;
use ::Child;
use super::prelude::*;

/// Inter-Process Communication.
///
/// There are two global IPC mechanisms that Linux supports; System-V IPC and
/// POSIX message queues. These, however, are globally accessible so any process
/// within an IPC namespace can connect to any other process in the same IPC
/// namespace that exposes one of these mechanisms without having any
/// information of these processes existing.
#[derive(Debug, Clone)]
pub struct Ipc {}

discarding_split!(Ipc);

impl Ipc {
    /// Configure a new IPC namespace for creation.
    pub fn new() -> Ipc {
        Ipc {}
    }
}

impl Namespace for Ipc {
    fn clone_flag(&self) -> Option<CloneFlags> {
        Some(CloneFlags::CLONE_NEWIPC)
    }
}
