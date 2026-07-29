//! Delivery boundary for concise Child report notifications to Main.

#![allow(async_fn_in_trait)]

use crate::domain::agent_orchestration::{CoordinatorNotification, OrchestrationError};
use crate::ports::agent_worker::WorkerBinding;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CoordinatorNotificationReceipt {
    pub accepted: bool,
    pub reason: Option<String>,
}

pub trait CoordinatorNotificationPort: Send + Sync {
    async fn notify_coordinator(
        &self,
        binding: &WorkerBinding,
        notification: &CoordinatorNotification,
    ) -> Result<CoordinatorNotificationReceipt, OrchestrationError>;
}
