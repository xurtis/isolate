//! Interface for isolation.

#![warn(missing_docs)]
#![deny(unused_must_use)]
#![warn(missing_debug_implementations)]

#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate nix;

#[macro_use]
mod error;
#[macro_use]
pub mod namespace;
mod context;

pub use context::{Child, Context};
pub use error::*;



