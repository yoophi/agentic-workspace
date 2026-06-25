#![allow(dead_code)]

use anyhow::Result;
use std::fmt;
use std::{future::Future, sync::Arc};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveRunError {
    DuplicateRunId { run_id: String },
    ConcurrentLimit { limit: usize },
}

impl fmt::Display for ReserveRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRunId { run_id } => write!(f, "duplicate run id: {run_id}"),
            Self::ConcurrentLimit { limit } => write!(
                f,
                "concurrent run limit ({limit}) reached; cancel an existing run before starting a new one"
            ),
        }
    }
}

impl std::error::Error for ReserveRunError {}

/// Storage for in-flight agent runs and their sessions.
///
/// The registry enforces identity/lifecycle invariants (unique run ids,
/// optional concurrency limit, cancellation) and owns the handles that allow
/// aborting a run. It is intentionally generic over the concrete session
/// type so the application layer can depend on this port without pulling in
/// adapter details.
pub trait SessionRegistry: Clone + Send + Sync + 'static {
    type Session: Send + Sync + 'static;

    fn reserve_run(
        &self,
        run_id: String,
        owner_window_label: Option<String>,
    ) -> impl Future<Output = Result<(), ReserveRunError>> + Send;

    fn attach_run_handle(
        &self,
        run_id: &str,
        handle: JoinHandle<()>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn attach_session(
        &self,
        run_id: &str,
        session: Arc<Self::Session>,
    ) -> impl Future<Output = Result<()>> + Send;

    fn active_session(
        &self,
        run_id: &str,
    ) -> impl Future<Output = Option<Arc<Self::Session>>> + Send;

    fn finish_run(&self, run_id: &str) -> impl Future<Output = ()> + Send;

    fn cancel_run(&self, run_id: &str) -> impl Future<Output = bool> + Send;
}
