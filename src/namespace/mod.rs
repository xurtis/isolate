//! Namespace representations and implementations.
//!
//! Linux provides a namespaces API. This allows for a process to place itself
//! (and its children) into a context isolated from other processes in some
//! respect. Each of the following namespaces can be individually isolated.
//!
//! The following groups of items each have their own namespace defined in
//! Linux:
//!
//! * [Inter-Process Communication](struct.Ipc.html)
//! * [Networking](struct.Network.html)
//! * [Mounts](struct.Mount.html)
//! * [Process IDs](struct.Pid.html)
//! * [Users and Groups](struct.User.html)
//! * [Unix Timesharing System](struct.Uts.html)

macro_rules! discarding_split {
    ($ty:ident) => {
        impl $crate::namespace::Split for $ty {
            type ExternalConfig = ();
            type InternalConfig = ();

            fn split(self) -> ((), ()) {
                ((), ())
            }
        }
    }
}

mod control_group;
mod ipc;
mod mount;
mod network;
mod pid;
mod user;
mod uts;

use nix::sched::CloneFlags;

pub use self::control_group::ControlGroup;
pub use self::ipc::Ipc;
pub use self::mount::{Mount, EmptyMount};
pub use self::network::Network;
pub use self::pid::Pid;
pub use self::user::User;
pub use self::uts::Uts;

use ::error::*;
use ::Child;

mod prelude {
    pub(super) use super::CloneFlags;
    pub(super) use super::Namespace;
    pub(super) use super::InternalConfig;
    pub(super) use super::ExternalConfig;
    pub(super) use super::Split;
}

/// A trait that represents a namespace that can be created and entered.
///
/// This configures the environment before a namespace is entered and after is
/// has been entered and also provides flags for the `clone` syscall to create a
/// new instance of a given namespace.
pub trait Namespace: ::std::fmt::Debug {
    /// Get the flag needed for clone to create new namespace.
    ///
    /// See `clone(2)` and `namespaces(7)` for more information.
    fn clone_flag(&self) -> Option<CloneFlags> {
        None
    }

    /// Configure system prior to creating the namespace.
    ///
    /// This executes all of the changes needed to be made external to the
    /// namespace in order for it to operate as desired.
    fn prepare(&self) -> Result<()> {
        Ok(())
    }
}

pub trait Split{
    /// The external configuration for the namespace.
    type ExternalConfig: ExternalConfig;
    type InternalConfig: InternalConfig;

    /// Split the configuration into the internal and external configuration objects.
    ///
    /// This splits the configuration into an internal and external configuration. Once the thread
    /// has been started, the external configuration is executed, followed by the child execution.
    /// After this, the provided closure is executed. Once the closure returns, the internal
    /// cleanup is performed and the result handed back to the parent thread. The parent thread
    /// then captures the result and runs the external cleanup.
    fn split(self) -> (Self::ExternalConfig, Self::InternalConfig);
}

/// Representation the configuration of the namespace intrenal to the child thread.
///
/// This is used to configure the thread internally before the child executes and clean up the
/// thread after the child has finished executing.
pub trait InternalConfig: ::std::fmt::Debug + Send {
    /// Configure the system from within the namespace after creation.
    ///
    /// This executes all of the changes needed to be made internal to the
    /// namespace in order for it to operate as desired.
    fn configure(&mut self) -> Result<()> {
        Ok(())
    }

    /// Cleanup the system from within the namespace at the end of the process.
    ///
    /// This executes all of the changes that need to be made internal to the
    /// namespace when the process ends.
    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
}

impl InternalConfig for () {}

/// Representation of the configuration of the namespace external to the child thread.
///
/// This is used to configure the thread externally after the thread is created but before the
/// internal configuration has occured.
pub trait ExternalConfig: ::std::fmt::Debug {
    /// Configure the system from outside the namespace after creation.
    ///
    /// This excutes all of the changes needed to be made externally to the
    /// namespace in order for it to operate as desired.
    fn configure(&mut self, &Child) -> Result<()> {
        Ok(())
    }

    /// Cleanup the system externally to the thread after it has exited.
    ///
    /// This is executed after the thread is completed executing (either successfully or
    /// unsuccessfully).
    fn cleanup(&mut self, &Child) -> Result<()> {
        Ok(())
    }
}

pub(crate) trait BoxedSplit: Namespace {
    /// Splits the internal type and wraps the result in boxes.
    fn boxed_split(&mut self) -> (Box<ExternalConfig>, Box<InternalConfig>);
}

#[derive(Debug)]
pub(crate) enum SplitBox<T> {
    Together(T),
    Split,
}

impl<S, E, I> SplitBox<S>
where
    S: Namespace + Split<ExternalConfig = E, InternalConfig = I> + 'static,
    E: ExternalConfig + 'static,
    I: InternalConfig + 'static,
{
    pub(crate) fn new(namespace: S) -> Box<BoxedSplit> {
        Box::new(SplitBox::Together(namespace))
    }
}

impl<T: Namespace> Namespace for SplitBox<T> {
    fn clone_flag(&self) -> Option<CloneFlags> {
        match self {
            SplitBox::Together(ns) => ns.clone_flag(),
            SplitBox::Split => None,
        }
    }

    fn prepare(&self) -> Result<()> {
        match self {
            SplitBox::Together(ns) => ns.prepare(),
            SplitBox::Split => Ok(()),
        }
    }
}

impl<S, E, I> BoxedSplit for SplitBox<S>
where
    S: Namespace + Split<ExternalConfig = E, InternalConfig = I>,
    E: ExternalConfig + 'static,
    I: InternalConfig + 'static,
{
    fn boxed_split(&mut self) -> (Box<ExternalConfig>, Box<InternalConfig>) {
        let mut swapped = SplitBox::Split;
        ::std::mem::swap(self, &mut swapped);
        if let SplitBox::Together(split) = swapped {
            let (outer, inner) = split.split();
            (Box::new(outer), Box::new(inner))
        } else {
            panic!("Attempted to SplitBox twice")
        }
    }
}

impl ExternalConfig for () {}
