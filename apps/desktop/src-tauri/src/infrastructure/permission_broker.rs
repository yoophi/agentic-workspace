#![allow(dead_code)]

use anyhow::{Result, anyhow};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, oneshot};

use crate::ports::permission::{PermissionDecision, PermissionDecisionPort};

struct PendingPermission {
    run_id: String,
    sender: oneshot::Sender<PermissionDecision>,
}

#[derive(Clone, Default)]
pub struct PermissionBroker {
    pending: Arc<Mutex<HashMap<String, PendingPermission>>>,
}

impl PermissionDecisionPort for PermissionBroker {
    async fn create_waiter(
        &self,
        run_id: String,
        permission_id: String,
    ) -> oneshot::Receiver<PermissionDecision> {
        let (sender, receiver) = oneshot::channel();
        self.pending
            .lock()
            .await
            .insert(permission_id, PendingPermission { run_id, sender });
        receiver
    }

    async fn respond(&self, permission_id: &str, decision: PermissionDecision) -> Result<()> {
        self.respond_matching_run(permission_id, None, decision)
            .await
    }
}

impl PermissionBroker {
    pub async fn respond_for_run(
        &self,
        run_id: &str,
        permission_id: &str,
        decision: PermissionDecision,
    ) -> Result<()> {
        self.respond_matching_run(permission_id, Some(run_id), decision)
            .await
    }

    pub async fn clear_run(&self, run_id: &str) {
        self.pending
            .lock()
            .await
            .retain(|_, pending| pending.run_id != run_id);
    }

    async fn respond_matching_run(
        &self,
        permission_id: &str,
        expected_run_id: Option<&str>,
        decision: PermissionDecision,
    ) -> Result<()> {
        let mut pending_permissions = self.pending.lock().await;
        let Some(pending) = pending_permissions.get(permission_id) else {
            return Err(anyhow!(
                "unknown or already answered permission: {permission_id}"
            ));
        };
        if let Some(expected_run_id) = expected_run_id {
            if pending.run_id != expected_run_id {
                return Err(anyhow!(
                    "permission {permission_id} belongs to a different run"
                ));
            }
        }
        let pending = pending_permissions
            .remove(permission_id)
            .expect("pending permission exists");
        pending
            .sender
            .send(decision)
            .map_err(|_| anyhow!("permission waiter is no longer active"))
    }
}

#[cfg(test)]
mod tests {
    use super::PermissionBroker;
    use crate::ports::permission::{PermissionDecision, PermissionDecisionPort};

    #[tokio::test]
    async fn clearing_one_run_keeps_other_run_permission_waiters() {
        let broker = PermissionBroker::default();
        let first = broker
            .create_waiter("run-a".to_string(), "permission-a".to_string())
            .await;
        let second = broker
            .create_waiter("run-b".to_string(), "permission-b".to_string())
            .await;

        broker.clear_run("run-a").await;

        assert!(first.await.is_err());
        broker
            .respond(
                "permission-b",
                PermissionDecision {
                    option_id: "allow".to_string(),
                },
            )
            .await
            .expect("second run permission should remain active");
        assert_eq!(second.await.expect("decision").option_id, "allow");
    }

    #[tokio::test]
    async fn responding_with_wrong_run_id_keeps_permission_waiter() {
        let broker = PermissionBroker::default();
        let receiver = broker
            .create_waiter("run-a".to_string(), "permission-a".to_string())
            .await;

        broker
            .respond_for_run(
                "run-b",
                "permission-a",
                PermissionDecision {
                    option_id: "allow".to_string(),
                },
            )
            .await
            .expect_err("wrong run should be rejected");

        broker
            .respond_for_run(
                "run-a",
                "permission-a",
                PermissionDecision {
                    option_id: "allow".to_string(),
                },
            )
            .await
            .expect("matching run should answer permission");
        assert_eq!(receiver.await.expect("decision").option_id, "allow");
    }
}
