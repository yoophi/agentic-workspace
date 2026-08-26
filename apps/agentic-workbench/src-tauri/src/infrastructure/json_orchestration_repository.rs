//! Atomic JSON implementation of the orchestration repository.

use std::{collections::HashSet, fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    domain::agent_orchestration::{
        CommandFailure, CoordinatorNotificationStatus, ORCHESTRATION_SCHEMA_VERSION,
        OrchestrationError, OrchestrationErrorCode, OrchestrationSession, TaskCommandStatus,
    },
    infrastructure::json_store::{load_json, save_json},
    ports::orchestration_repository::OrchestrationRepository,
};

const STORE_LABEL: &str = "orchestration sessions";
type PendingOutboxEntry = (String, Vec<String>, Vec<String>);

#[derive(Clone)]
pub struct JsonOrchestrationRepository {
    store_path: PathBuf,
}

impl JsonOrchestrationRepository {
    pub fn from_app(app: &AppHandle) -> Result<Self, String> {
        let directory = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("Failed to resolve app data directory: {error}"))?;
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Failed to create app data directory: {error}"))?;
        Ok(Self::from_path(
            directory.join("orchestration-sessions.json"),
        ))
    }

    pub fn from_path(store_path: PathBuf) -> Self {
        Self { store_path }
    }

    fn persistence_error(error: String) -> OrchestrationError {
        OrchestrationError::new(OrchestrationErrorCode::WorkerUnavailable, error).retryable()
    }

    /// Returns durable work that must be reconciled after a process restart.
    pub fn pending_outbox(&self) -> Result<Vec<PendingOutboxEntry>, OrchestrationError> {
        self.load_sessions().map(|sessions| {
            sessions
                .into_iter()
                .map(|session| {
                    let command_ids = session
                        .commands
                        .iter()
                        .filter(|command| {
                            matches!(
                                command.status,
                                TaskCommandStatus::Pending | TaskCommandStatus::Dispatching
                            )
                        })
                        .map(|command| command.id.clone())
                        .collect();
                    let notification_ids = session
                        .coordinator_notifications
                        .iter()
                        .filter(|notification| {
                            matches!(
                                notification.status,
                                CoordinatorNotificationStatus::Pending
                                    | CoordinatorNotificationStatus::Dispatching
                            )
                        })
                        .map(|notification| notification.id.clone())
                        .collect();
                    (session.id, command_ids, notification_ids)
                })
                .collect()
        })
    }
}

impl OrchestrationRepository for JsonOrchestrationRepository {
    fn load_sessions(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
        let mut sessions: Vec<OrchestrationSession> =
            load_json(&self.store_path, STORE_LABEL).map_err(Self::persistence_error)?;
        for session in &mut sessions {
            session.schema_version = ORCHESTRATION_SCHEMA_VERSION;
            for notification in &mut session.coordinator_notifications {
                if notification.status == CoordinatorNotificationStatus::Accepted {
                    notification.status = CoordinatorNotificationStatus::Pending;
                    notification.failure = Some(CommandFailure {
                        code: OrchestrationErrorCode::RuntimeLost,
                        message:
                            "Legacy acceptance did not confirm Main delivery; notification will be retried."
                                .into(),
                        retryable: true,
                    });
                }
            }
            session.validate()?;
        }
        Ok(sessions)
    }

    fn save_sessions(&self, sessions: &[OrchestrationSession]) -> Result<(), OrchestrationError> {
        let mut ids = HashSet::new();
        for session in sessions {
            session.validate()?;
            if !ids.insert(&session.id) {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::DuplicateConflict,
                    "Orchestration session ids must be unique.",
                ));
            }
        }
        save_json(&self.store_path, STORE_LABEL, sessions).map_err(Self::persistence_error)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::{
        domain::agent_orchestration::{
            CoordinatorNotification, OrchestrationSession, TaskReportType,
        },
        ports::orchestration_repository::OrchestrationRepository,
    };

    #[test]
    fn saves_current_session_and_recovers_the_previous_revision_from_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orchestration-sessions.json");
        let repository = JsonOrchestrationRepository::from_path(path.clone());
        let mut session = OrchestrationSession::new("workspace-1", "/repo", "window-1", "now");

        repository.save_sessions(&[session.clone()]).unwrap();
        session.revision = 1;
        repository.save_sessions(&[session]).unwrap();
        fs::write(&path, "{ broken").unwrap();

        let recovered = repository.load_sessions().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].revision, 0);
    }

    #[test]
    fn rejects_duplicate_session_ids_and_invalid_topology() {
        let dir = tempdir().unwrap();
        let repository = JsonOrchestrationRepository::from_path(dir.path().join("sessions.json"));
        let session = OrchestrationSession::new("workspace-1", "/repo", "window-1", "now");

        let error = repository
            .save_sessions(&[session.clone(), session])
            .unwrap_err();
        assert_eq!(error.code, OrchestrationErrorCode::DuplicateConflict);
    }

    #[test]
    fn migrates_legacy_snapshots_and_recovers_pending_outbox() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sessions.json");
        let repository = JsonOrchestrationRepository::from_path(path.clone());
        fs::write(
            &path,
            r#"[{
              "schemaVersion": 1,
              "id": "workspace-1",
              "worktreePath": "/repo",
              "boundWindowLabel": "window-1",
              "mainNodeId": "main-agent-run",
              "activeCoordinatorGenerationId": null,
              "nodes": [{
                "id": "main-agent-run",
                "kind": "main",
                "parentNodeId": null,
                "role": {"id":"main","name":"Main","responsibility":"Coordinate","expectedOutput":"Result","systemInstructions":null},
                "currentRunId": null,
                "assignedTaskId": null,
                "executionStatus": "unassigned",
                "presentationStatus": "panel",
                "promotionPolicy": "always",
                "runtimeProfile": null,
                "lastActivityAt": null,
                "createdBy": "user",
                "createdAt": "now"
              }],
              "generations": [],
              "tasks": [],
              "reports": [],
              "dispatches": [],
              "idempotencyRecords": [],
              "revision": 0,
              "createdAt": "now",
              "updatedAt": "now"
            }]"#,
        )
        .unwrap();

        let sessions = repository.load_sessions().unwrap();
        assert_eq!(sessions[0].schema_version, ORCHESTRATION_SCHEMA_VERSION);
        assert!(sessions[0].commands.is_empty());
        assert!(sessions[0].coordinator_notifications.is_empty());
        let pending = repository.pending_outbox().unwrap();
        assert_eq!(pending, vec![("workspace-1".into(), vec![], vec![])]);
    }

    #[test]
    fn legacy_accepted_notifications_are_retried_instead_of_claiming_delivery() {
        let dir = tempdir().unwrap();
        let repository = JsonOrchestrationRepository::from_path(dir.path().join("sessions.json"));
        let mut session = OrchestrationSession::new("workspace-1", "/repo", "window-1", "now");
        session
            .coordinator_notifications
            .push(CoordinatorNotification {
                id: "notification-1".into(),
                report_id: "report-1".into(),
                task_id: "task-1".into(),
                report_type: TaskReportType::Result,
                generation_id: "generation-1".into(),
                main_run_id: Some("main-run".into()),
                status: CoordinatorNotificationStatus::Accepted,
                attempt_count: 1,
                failure: None,
                collected_at: None,
                created_at: "now".into(),
                updated_at: "now".into(),
            });
        repository.save_sessions(&[session]).unwrap();

        let loaded = repository.load_sessions().unwrap();
        let notification = &loaded[0].coordinator_notifications[0];

        assert_eq!(notification.status, CoordinatorNotificationStatus::Pending);
        assert!(
            notification
                .failure
                .as_ref()
                .is_some_and(|failure| failure.retryable)
        );
    }
}
