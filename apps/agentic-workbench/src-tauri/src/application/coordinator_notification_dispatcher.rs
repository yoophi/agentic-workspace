//! Durable Child report notification delivery to the active Main generation.

use crate::{
    domain::agent_orchestration::{
        CommandFailure, CoordinatorGenerationStatus, CoordinatorNotification,
        CoordinatorNotificationStatus, MAIN_AGENT_NODE_ID, OrchestrationError,
        OrchestrationErrorCode, OrchestrationSession,
    },
    ports::{
        agent_worker::WorkerBinding, coordinator_notification::CoordinatorNotificationPort,
        orchestration_repository::OrchestrationRepository,
    },
};

pub struct CoordinatorNotificationDispatcher<R, N> {
    repository: R,
    notifier: N,
}

impl<R, N> CoordinatorNotificationDispatcher<R, N>
where
    R: OrchestrationRepository,
    N: CoordinatorNotificationPort,
{
    pub fn new(repository: R, notifier: N) -> Self {
        Self {
            repository,
            notifier,
        }
    }

    pub async fn dispatch_pending(
        &self,
        window_label: &str,
    ) -> Result<Vec<CoordinatorNotification>, OrchestrationError> {
        let mut delivered = Vec::new();
        let mut reactivate_failed = true;
        loop {
            let mut sessions = self.repository.load_sessions()?;
            let session = session_for_window_mut(&mut sessions, window_label)?;
            supersede_stale_notifications(session);
            if reactivate_failed {
                for notification in &mut session.coordinator_notifications {
                    if notification.status == CoordinatorNotificationStatus::Failed
                        && notification
                            .failure
                            .as_ref()
                            .is_some_and(|failure| failure.retryable)
                    {
                        notification.transition(CoordinatorNotificationStatus::Pending, now())?;
                    }
                }
                reactivate_failed = false;
            }
            let Some((notification_id, binding)) = next_delivery(session, window_label)? else {
                self.repository.save_sessions(&sessions)?;
                break;
            };
            {
                let notification = session
                    .coordinator_notifications
                    .iter_mut()
                    .find(|notification| notification.id == notification_id)
                    .ok_or_else(|| not_found("Coordinator notification"))?;
                notification.attempt_count += 1;
                notification.transition(CoordinatorNotificationStatus::Dispatching, now())?;
            }
            touch(session);
            let snapshot = session
                .coordinator_notifications
                .iter()
                .find(|notification| notification.id == notification_id)
                .cloned()
                .ok_or_else(|| not_found("Coordinator notification"))?;
            self.repository.save_sessions(&sessions)?;
            let receipt = self.notifier.notify_coordinator(&binding, &snapshot).await;

            let mut sessions = self.repository.load_sessions()?;
            let session = session_for_window_mut(&mut sessions, window_label)?;
            let notification = {
                let notification = session
                    .coordinator_notifications
                    .iter_mut()
                    .find(|candidate| candidate.id == notification_id)
                    .ok_or_else(|| not_found("Coordinator notification"))?;
                if notification.status != CoordinatorNotificationStatus::Processed {
                    match receipt {
                        Ok(receipt) if receipt.accepted => {
                            notification.failure = None;
                            let status = if notification.collected_at.is_some() {
                                CoordinatorNotificationStatus::Processed
                            } else {
                                CoordinatorNotificationStatus::Delivered
                            };
                            notification.transition(status, now())?;
                        }
                        Ok(receipt) => {
                            notification.failure = Some(CommandFailure {
                                code: OrchestrationErrorCode::WorkerUnavailable,
                                message: receipt.reason.unwrap_or_else(|| {
                                    "Main rejected the report notification.".into()
                                }),
                                retryable: true,
                            });
                            notification
                                .transition(CoordinatorNotificationStatus::Failed, now())?;
                        }
                        Err(error) => {
                            notification.failure = Some(CommandFailure {
                                code: error.code,
                                message: error.message,
                                retryable: error.retryable,
                            });
                            notification
                                .transition(CoordinatorNotificationStatus::Failed, now())?;
                        }
                    }
                }
                notification.clone()
            };
            touch(session);
            self.repository.save_sessions(&sessions)?;
            delivered.push(notification);
        }
        Ok(delivered)
    }

    pub fn recover_interrupted(
        &self,
        window_label: &str,
    ) -> Result<Vec<CoordinatorNotification>, OrchestrationError> {
        let mut sessions = self.repository.load_sessions()?;
        let session = session_for_window_mut(&mut sessions, window_label)?;
        let mut recovered = Vec::new();
        for notification in &mut session.coordinator_notifications {
            if notification.status == CoordinatorNotificationStatus::Dispatching {
                notification.status = CoordinatorNotificationStatus::Pending;
                notification.failure = Some(CommandFailure {
                    code: OrchestrationErrorCode::RuntimeLost,
                    message: "Main notification delivery was interrupted.".into(),
                    retryable: true,
                });
                notification.updated_at = now();
                recovered.push(notification.clone());
            }
        }
        if !recovered.is_empty() {
            touch(session);
            self.repository.save_sessions(&sessions)?;
        }
        Ok(recovered)
    }
}

fn next_delivery(
    session: &OrchestrationSession,
    window_label: &str,
) -> Result<Option<(String, WorkerBinding)>, OrchestrationError> {
    let Some(active_generation_id) = session.active_coordinator_generation_id.as_deref() else {
        return Ok(None);
    };
    let Some(main) = session
        .nodes
        .iter()
        .find(|node| node.id == MAIN_AGENT_NODE_ID)
    else {
        return Err(not_found("Main node"));
    };
    let Some(main_run_id) = main.current_run_id.clone() else {
        return Ok(None);
    };
    let Some(notification) = session
        .coordinator_notifications
        .iter()
        .find(|notification| {
            notification.status == CoordinatorNotificationStatus::Pending
                && notification.generation_id == active_generation_id
        })
    else {
        return Ok(None);
    };
    Ok(Some((
        notification.id.clone(),
        WorkerBinding {
            workspace_id: session.id.clone(),
            window_label: window_label.into(),
            node_id: MAIN_AGENT_NODE_ID.into(),
            task_id: notification.task_id.clone(),
            run_id: main_run_id,
        },
    )))
}

fn supersede_stale_notifications(session: &mut OrchestrationSession) {
    let active_generation_id = session.active_coordinator_generation_id.as_deref();
    for notification in &mut session.coordinator_notifications {
        let generation_is_active = session.generations.iter().any(|generation| {
            generation.id == notification.generation_id
                && generation.status == CoordinatorGenerationStatus::Active
        });
        if matches!(
            notification.status,
            CoordinatorNotificationStatus::Pending
                | CoordinatorNotificationStatus::Accepted
                | CoordinatorNotificationStatus::Failed
        ) && (!generation_is_active
            || active_generation_id != Some(notification.generation_id.as_str()))
        {
            notification.status = CoordinatorNotificationStatus::Superseded;
            notification.updated_at = now();
        }
    }
}

fn session_for_window_mut<'a>(
    sessions: &'a mut [OrchestrationSession],
    window_label: &str,
) -> Result<&'a mut OrchestrationSession, OrchestrationError> {
    let session = sessions
        .iter_mut()
        .find(|session| session.bound_window_label.as_deref() == Some(window_label))
        .ok_or_else(|| not_found("Orchestration workspace"))?;
    session.assert_scope(window_label)?;
    Ok(session)
}

fn touch(session: &mut OrchestrationSession) {
    session.revision += 1;
    session.updated_at = now();
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn not_found(subject: &str) -> OrchestrationError {
    OrchestrationError::new(
        OrchestrationErrorCode::NotFound,
        format!("{subject} was not found."),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::{
        domain::agent_orchestration::{CoordinatorGeneration, TaskReportType},
        ports::coordinator_notification::CoordinatorNotificationReceipt,
    };

    #[derive(Clone)]
    struct MemoryRepository(Arc<Mutex<Vec<OrchestrationSession>>>);

    impl OrchestrationRepository for MemoryRepository {
        fn load_sessions(&self) -> Result<Vec<OrchestrationSession>, OrchestrationError> {
            Ok(self.0.lock().unwrap().clone())
        }

        fn save_sessions(
            &self,
            sessions: &[OrchestrationSession],
        ) -> Result<(), OrchestrationError> {
            *self.0.lock().unwrap() = sessions.to_vec();
            Ok(())
        }
    }

    #[derive(Clone)]
    struct FakeNotifier(Arc<Mutex<Vec<String>>>);

    impl CoordinatorNotificationPort for FakeNotifier {
        async fn notify_coordinator(
            &self,
            _binding: &WorkerBinding,
            notification: &CoordinatorNotification,
        ) -> Result<CoordinatorNotificationReceipt, OrchestrationError> {
            self.0.lock().unwrap().push(notification.id.clone());
            Ok(CoordinatorNotificationReceipt {
                accepted: true,
                reason: None,
            })
        }
    }

    #[derive(Clone)]
    struct UnavailableNotifier;

    impl CoordinatorNotificationPort for UnavailableNotifier {
        async fn notify_coordinator(
            &self,
            _binding: &WorkerBinding,
            _notification: &CoordinatorNotification,
        ) -> Result<CoordinatorNotificationReceipt, OrchestrationError> {
            Err(
                OrchestrationError::new(OrchestrationErrorCode::WorkerUnavailable, "Main is busy.")
                    .retryable(),
            )
        }
    }

    #[derive(Clone)]
    struct CollectingNotifier(MemoryRepository);

    impl CoordinatorNotificationPort for CollectingNotifier {
        async fn notify_coordinator(
            &self,
            _binding: &WorkerBinding,
            notification: &CoordinatorNotification,
        ) -> Result<CoordinatorNotificationReceipt, OrchestrationError> {
            let mut sessions = self.0.load_sessions()?;
            let collected_at = now();
            let notification = sessions[0]
                .coordinator_notifications
                .iter_mut()
                .find(|candidate| candidate.id == notification.id)
                .unwrap();
            notification.collected_at = Some(collected_at.clone());
            notification.updated_at = collected_at;
            self.0.save_sessions(&sessions)?;
            Ok(CoordinatorNotificationReceipt {
                accepted: true,
                reason: None,
            })
        }
    }

    fn repository(main_available: bool) -> MemoryRepository {
        let now = "2026-07-27T00:00:00Z".to_string();
        let mut session =
            OrchestrationSession::new("workspace-1", "/repo", "window-1", now.clone());
        session.active_coordinator_generation_id = Some("generation-1".into());
        session.generations.push(CoordinatorGeneration {
            id: "generation-1".into(),
            ordinal: 1,
            main_node_id: MAIN_AGENT_NODE_ID.into(),
            run_id: "main-run".into(),
            previous_generation_id: None,
            status: CoordinatorGenerationStatus::Active,
            started_at: now.clone(),
            ended_at: None,
            handoff_summary: None,
            successor_generation_id: None,
        });
        if main_available {
            session.nodes[0].current_run_id = Some("main-run".into());
        }
        session
            .coordinator_notifications
            .push(CoordinatorNotification {
                id: "notification-1".into(),
                report_id: "report-1".into(),
                task_id: "task-1".into(),
                report_type: TaskReportType::Result,
                generation_id: "generation-1".into(),
                main_run_id: main_available.then(|| "main-run".into()),
                status: CoordinatorNotificationStatus::Pending,
                attempt_count: 0,
                failure: None,
                collected_at: None,
                created_at: now.clone(),
                updated_at: now,
            });
        MemoryRepository(Arc::new(Mutex::new(vec![session])))
    }

    #[tokio::test]
    async fn delivers_each_report_notification_exactly_once() {
        let repository = repository(true);
        let calls = Arc::new(Mutex::new(vec![]));
        let dispatcher =
            CoordinatorNotificationDispatcher::new(repository.clone(), FakeNotifier(calls.clone()));
        let delivered = dispatcher.dispatch_pending("window-1").await.unwrap();
        assert_eq!(delivered.len(), 1);
        assert_eq!(
            delivered[0].status,
            CoordinatorNotificationStatus::Delivered
        );
        assert!(
            dispatcher
                .dispatch_pending("window-1")
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(calls.lock().unwrap().as_slice(), ["notification-1"]);
    }

    #[tokio::test]
    async fn keeps_notifications_pending_while_main_is_unavailable() {
        let repository = repository(false);
        let dispatcher = CoordinatorNotificationDispatcher::new(
            repository.clone(),
            FakeNotifier(Arc::new(Mutex::new(vec![]))),
        );
        assert!(
            dispatcher
                .dispatch_pending("window-1")
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            repository.load_sessions().unwrap()[0].coordinator_notifications[0].status,
            CoordinatorNotificationStatus::Pending
        );
    }

    #[tokio::test]
    async fn retries_a_retryable_failed_notification_on_the_next_dispatch_pass() {
        let repository = repository(true);
        let first = CoordinatorNotificationDispatcher::new(repository.clone(), UnavailableNotifier);
        let failed = first.dispatch_pending("window-1").await.unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].status, CoordinatorNotificationStatus::Failed);

        let calls = Arc::new(Mutex::new(vec![]));
        let retry =
            CoordinatorNotificationDispatcher::new(repository.clone(), FakeNotifier(calls.clone()));
        let delivered = retry.dispatch_pending("window-1").await.unwrap();
        assert_eq!(delivered.len(), 1);
        assert_eq!(
            delivered[0].status,
            CoordinatorNotificationStatus::Delivered
        );
        assert_eq!(calls.lock().unwrap().as_slice(), ["notification-1"]);
    }

    #[tokio::test]
    async fn processed_collection_wins_over_late_delivery_completion() {
        let repository = repository(true);
        let dispatcher = CoordinatorNotificationDispatcher::new(
            repository.clone(),
            CollectingNotifier(repository.clone()),
        );

        let delivered = dispatcher.dispatch_pending("window-1").await.unwrap();

        assert_eq!(delivered.len(), 1);
        assert_eq!(
            delivered[0].status,
            CoordinatorNotificationStatus::Processed
        );
        assert_eq!(
            repository.load_sessions().unwrap()[0].coordinator_notifications[0].status,
            CoordinatorNotificationStatus::Processed
        );
    }
}
