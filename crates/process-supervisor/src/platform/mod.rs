//! Target-specific containment feasibility and, after the prerequisite gate,
//! production launch adapters.

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;
