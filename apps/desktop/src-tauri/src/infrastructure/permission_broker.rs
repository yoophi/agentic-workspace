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
        let Some(pending) = self.pending.lock().await.remove(permission_id) else {
            return Err(anyhow!(
                "unknown or already answered permission: {permission_id}"
            ));
        };
        pending
            .sender
            .send(decision)
            .map_err(|_| anyhow!("permission waiter is no longer active"))
    }
}

impl PermissionBroker {
    pub async fn clear_run(&self, run_id: &str) {
        self.pending
            .lock()
            .await
            .retain(|_, pending| pending.run_id != run_id);
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
}
