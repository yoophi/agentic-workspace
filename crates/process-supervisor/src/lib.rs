//! Shared process-supervision primitives for server-owned child processes.
//!
//! Stage 045 starts with an isolated platform feasibility gate. Production
//! consumers must not depend on this crate until the target capability matrix
//! proves the containment contract.

pub mod platform;
pub mod spec;
pub mod state;
