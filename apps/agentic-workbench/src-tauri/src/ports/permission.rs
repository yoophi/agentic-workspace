#![allow(dead_code)]

use anyhow::Result;
use std::future::Future;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct PermissionDecision {
    pub option_id: String,
}

pub trait PermissionDecisionPort: Clone + Send + Sync + 'static {
    fn create_waiter(
        &self,
        run_id: String,
        permission_id: String,
    ) -> impl Future<Output = oneshot::Receiver<PermissionDecision>> + Send;

    fn respond(
        &self,
        permission_id: &str,
        decision: PermissionDecision,
    ) -> impl Future<Output = Result<()>> + Send;
}
