//! Run-scoped MCP capability registry.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::agent_orchestration::{
    MAIN_AGENT_NODE_ID, OrchestrationError, OrchestrationErrorCode,
};

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityActorKind {
    Coordinator,
    Child,
    LegacyRun,
}

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityPrincipal {
    pub actor_kind: CapabilityActorKind,
    pub workspace_id: Option<String>,
    pub window_label: Option<String>,
    pub node_id: Option<String>,
    pub run_id: String,
    pub task_id: Option<String>,
    pub generation_id: Option<String>,
}

impl CapabilityPrincipal {
    pub fn legacy_run(run_id: impl Into<String>) -> Self {
        Self {
            actor_kind: CapabilityActorKind::LegacyRun,
            workspace_id: None,
            window_label: None,
            node_id: None,
            run_id: run_id.into(),
            task_id: None,
            generation_id: None,
        }
    }

    pub fn coordinator(
        workspace_id: impl Into<String>,
        window_label: impl Into<String>,
        run_id: impl Into<String>,
        generation_id: impl Into<String>,
    ) -> Self {
        Self {
            actor_kind: CapabilityActorKind::Coordinator,
            workspace_id: Some(workspace_id.into()),
            window_label: Some(window_label.into()),
            node_id: Some(MAIN_AGENT_NODE_ID.into()),
            run_id: run_id.into(),
            task_id: None,
            generation_id: Some(generation_id.into()),
        }
    }

    pub fn child(
        workspace_id: impl Into<String>,
        window_label: impl Into<String>,
        node_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
    ) -> Self {
        Self {
            actor_kind: CapabilityActorKind::Child,
            workspace_id: Some(workspace_id.into()),
            window_label: Some(window_label.into()),
            node_id: Some(node_id.into()),
            run_id: run_id.into(),
            task_id: Some(task_id.into()),
            generation_id: None,
        }
    }
}

#[derive(Clone, Default)]
pub struct CapabilityRegistry {
    entries: Arc<RwLock<HashMap<String, CapabilityPrincipal>>>,
}

impl CapabilityRegistry {
    pub fn issue(&self, principal: CapabilityPrincipal) -> Result<String, OrchestrationError> {
        let token = format!("awcap_{}", Uuid::new_v4().simple());
        self.entries
            .write()
            .map_err(|_| registry_error())?
            .insert(token.clone(), principal);
        Ok(token)
    }

    pub fn resolve(&self, token: &str) -> Result<CapabilityPrincipal, OrchestrationError> {
        self.entries
            .read()
            .map_err(|_| registry_error())?
            .get(token)
            .cloned()
            .ok_or_else(|| {
                OrchestrationError::new(
                    OrchestrationErrorCode::Unauthorized,
                    "MCP capability is invalid or expired.",
                )
            })
    }

    pub fn revoke_run(&self, run_id: &str) -> Result<(), OrchestrationError> {
        self.entries
            .write()
            .map_err(|_| registry_error())?
            .retain(|_, principal| principal.run_id != run_id);
        Ok(())
    }

    pub fn bind_run(
        &self,
        run_id: &str,
        principal: CapabilityPrincipal,
    ) -> Result<usize, OrchestrationError> {
        if principal.run_id != run_id {
            return Err(OrchestrationError::new(
                OrchestrationErrorCode::Unauthorized,
                "Capability principal run does not match the requested run.",
            ));
        }
        let mut entries = self.entries.write().map_err(|_| registry_error())?;
        let mut updated = 0;
        for current in entries.values_mut() {
            if current.run_id == run_id {
                *current = principal.clone();
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub fn revoke_generation(&self, generation_id: &str) -> Result<(), OrchestrationError> {
        self.entries
            .write()
            .map_err(|_| registry_error())?
            .retain(|_, principal| principal.generation_id.as_deref() != Some(generation_id));
        Ok(())
    }
}

fn registry_error() -> OrchestrationError {
    OrchestrationError::new(
        OrchestrationErrorCode::WorkerUnavailable,
        "MCP capability registry is unavailable.",
    )
    .retryable()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issues_run_scoped_capabilities_and_derives_the_principal() {
        let registry = CapabilityRegistry::default();
        let principal =
            CapabilityPrincipal::child("workspace-1", "window-1", "child-1", "run-1", "task-1");
        let token = registry.issue(principal.clone()).unwrap();

        assert_ne!(token, "run-1");
        assert_eq!(registry.resolve(&token).unwrap(), principal);
        assert!(registry.resolve("run-1").is_err());
    }

    #[test]
    fn revokes_stale_run_and_generation_capabilities() {
        let registry = CapabilityRegistry::default();
        let old = registry
            .issue(CapabilityPrincipal::coordinator(
                "workspace-1",
                "window-1",
                "run-old",
                "generation-old",
            ))
            .unwrap();
        let current = registry
            .issue(CapabilityPrincipal::coordinator(
                "workspace-1",
                "window-1",
                "run-current",
                "generation-current",
            ))
            .unwrap();

        registry.revoke_generation("generation-old").unwrap();
        assert!(registry.resolve(&old).is_err());
        assert!(registry.resolve(&current).is_ok());
        registry.revoke_run("run-current").unwrap();
        assert!(registry.resolve(&current).is_err());
    }
}
