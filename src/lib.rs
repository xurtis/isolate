//! Interface for isolation.

#![warn(missing_docs)]
#![deny(unused_must_use)]

#[macro_use]
extern crate error_chain;
extern crate errno;
extern crate libc;

#[macro_use]
mod error;
mod context;
pub mod namespace;

pub use context::{Child, Context};
pub use error::*;



