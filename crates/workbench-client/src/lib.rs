//! Existing-instance Workbench caller, independent of the server runtime.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod ports;

#[cfg(test)]
extern crate self as workbench_client;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod fixture;
