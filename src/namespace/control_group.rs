use ::error::*;
use ::Child;
use super::prelude::*;

/// Control group namespace representation.
///
/// Each process exists in a control group. A given control group can be
/// assigned resource limits. This ensures that the total amount of resources,
/// such as CPU time and system memory, used by all of the process in the group
/// is limited.
#[derive(Debug, Clone)]
pub struct ControlGroup {}

discarding_split!(ControlGroup);

impl ControlGroup {
    /// Configure a new Control Group namespace for creation.
    pub fn new() -> ControlGroup {
        ControlGroup {}
    }
}

impl Namespace for ControlGroup {
    fn clone_flag(&self) -> Option<CloneFlags> {
        Some(CloneFlags::CLONE_NEWCGROUP)
    }
}
