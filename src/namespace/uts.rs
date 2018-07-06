use ::error::*;
use ::Child;
use super::prelude::*;

/// Unix Timesharing System (UTS)
///
/// The Unix Timesharing System provides the domain and hostname of the system.
/// This is given its own namespace and can be changed within that namespace.
#[derive(Debug, Clone)]
pub struct Uts {}

discarding_split!(Uts);

impl Uts {
    /// Configure a new UTS namespace for creation.
    pub fn new() -> Uts {
        Uts {}
    }
}

impl Namespace for Uts {
    fn clone_flag(&self) -> Option<CloneFlags> {
        Some(CloneFlags::CLONE_NEWUTS)
    }
}
