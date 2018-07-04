use ::error::*;
use ::Child;
use super::{Namespace, CloneFlags};

/// Networking
///
/// The networking namespace encapsulates an entire network stack shared between
/// processes. Each physical network device lives in (usually) the global
/// networking namespace as does the networking stack that communicates with
/// them.
///
/// A set of processes can be placed in a separate networking namespace to
/// isolate them from networking or to provide some filtered access to the
/// global networking namespace (and external network) using virtual network
/// devices.
#[derive(Debug, Clone)]
pub struct Network {}

impl Network {
    /// Configure a new IPC namespace for creation.
    pub fn new() -> Network {
        Network {}
    }
}

impl Namespace for Network {
    fn clone_flag(&self) -> Option<CloneFlags> {
        Some(CloneFlags::CLONE_NEWNET)
    }
}
