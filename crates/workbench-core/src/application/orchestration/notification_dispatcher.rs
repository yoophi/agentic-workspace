//! Durable Child report notification delivery to the active Main generation.

use std::sync::Arc;

use crate::{
    application::work_gate::{Reservation, WorkGate},
    domain::agent_orchestration::{
        CommandFailure, CoordinatorGenerationStatus, CoordinatorNotification,
        CoordinatorNotificationStatus, OrchestrationError, OrchestrationErrorCode,
        OrchestrationSession, MAIN_AGENT_NODE_ID,
    },
    ports::{
        agent_worker::WorkerBinding,
        coordinator_notification::CoordinatorNotificationPort,
        orchestration_repository::{OrchestrationRepository, OrchestrationTransaction},
    },
};

/// 전달 한 바퀴의 관측 지점(044 T039 시험). 운영에서는 probe가 없다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchPoint {
    /// 전달 한 바퀴의 첫 poll(저장소를 읽기 전).
    FirstPoll,
    /// `Dispatching{attemptId}` 저장 commit 직후·coordinator 전달 전.
    AfterDispatchingSaved,
    /// coordinator 전달이 돌아온 뒤·결과 저장 transaction 직전.
    BeforeResultSave,
}

/// probe가 지점에서 돌려주는 지시.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchAction {
    Continue,
    /// 결과 저장이 실패한 것처럼 저장하지 않고 오류로 끝낸다.
    FailResultSave,
}

pub type DispatchProbe = std::sync::Arc<
    dyn Fn(
            DispatchPoint,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DispatchAction> + Send>>
        + Send
        + Sync,
>;

/// 전달 한 바퀴가 서버에 알리는 일(044 R14 표 6'·6'').
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchEvent {
    /// 시도가 결과를 저장하지 못하고 끝났다(drop·저장 실패) — 회수와 재전달이 필요하다.
    Orphaned,
    /// 한 바퀴가 끝났고 재시도 가능 실패가 남았다 — backoff 뒤 다시 전달한다.
    RetryableFailures,
    /// 한 바퀴가 끝났고 남은 재시도 가능 실패가 없다.
    Settled,
}

pub type DispatchEvents = Arc<dyn Fn(&str, DispatchEvent) + Send + Sync>;

/// 전달 시도 하나: 시도 id와 그 N-notify 예약(관문이 없으면 예약 없음).
pub struct NotifyAttempt {
    attempt_id: String,
    _reservation: Option<Reservation>,
}

impl NotifyAttempt {
    /// 새 시도. 관문이 `stopping`이면 `None`(새 전달을 시작하지 않는다).
    pub fn begin(gate: Option<&Arc<WorkGate>>) -> Option<Self> {
        let attempt_id = uuid::Uuid::new_v4().to_string();
        let reservation = match gate {
            Some(gate) => Some(gate.reserve_notify(&attempt_id).ok()?),
            None => None,
        };
        Some(Self {
            attempt_id,
            _reservation: reservation,
        })
    }
}

/// 시도의 끝을 지킨다: 결과 저장 commit 없이 drop되면 예약을 먼저 놓고 회수를 부탁한다.
struct AttemptGuard {
    attempt: Option<NotifyAttempt>,
    saved: bool,
    done: bool,
    bench_id: String,
    events: Option<DispatchEvents>,
}

impl AttemptGuard {
    fn attempt_id(&self) -> &str {
        &self.attempt.as_ref().expect("live attempt").attempt_id
    }
}

impl Drop for AttemptGuard {
    fn drop(&mut self) {
        drop(self.attempt.take());
        if self.saved && !self.done {
            if let Some(events) = &self.events {
                events(&self.bench_id, DispatchEvent::Orphaned);
            }
        }
    }
}

pub struct CoordinatorNotificationDispatcher<R, N> {
    repository: R,
    notifier: N,
    probe: Option<DispatchProbe>,
    gate: Option<Arc<WorkGate>>,
    events: Option<DispatchEvents>,
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
            probe: None,
            gate: None,
            events: None,
        }
    }

    /// 작업 관문과 서버 알림을 붙인다(운영 조립). 관문이 있으면 시도마다 N-notify를 쥐고 회수 한 바퀴를 돈다.
    pub fn with_server(mut self, gate: Arc<WorkGate>, events: DispatchEvents) -> Self {
        self.gate = Some(gate);
        self.events = Some(events);
        self
    }

    pub fn with_probe(mut self, probe: Option<DispatchProbe>) -> Self {
        self.probe = probe;
        self
    }

    async fn probe(&self, point: DispatchPoint) -> DispatchAction {
        match &self.probe {
            Some(probe) => probe(point).await,
            None => DispatchAction::Continue,
        }
    }

    pub async fn dispatch_pending(
        &self,
        bench_id: &str,
    ) -> Result<Vec<CoordinatorNotification>, OrchestrationError> {
        self.dispatch_pending_with(bench_id, None).await
    }

    /// 전달 한 바퀴(044 R14 표 6'·6''). 시작할 때 살아 있는 시도가 없는 `Dispatching{attemptId}`를 회수한다. 전달마다
    /// 시도 id와 N-notify 예약(`first`가 있으면 보고 호출이 넘긴 예약)을 쥐고 `Dispatching{attemptId}`를 저장한 뒤
    /// coordinator에 전달하고, **결과 저장 commit 뒤에만** 예약을 놓는다. 결과를 저장하지 못하고 끝나면(drop·저장 실패)
    /// 시도 guard가 회수 한 바퀴를 부탁한다. 재시도 가능 실패가 남으면 서버 재전달을 부탁한다.
    pub async fn dispatch_pending_with(
        &self,
        bench_id: &str,
        first: Option<NotifyAttempt>,
    ) -> Result<Vec<CoordinatorNotification>, OrchestrationError> {
        let mut first = first;
        let mut delivered = Vec::new();
        let mut reactivate_failed = true;
        let mut retryable_failures = false;
        self.probe(DispatchPoint::FirstPoll).await;
        self.reclaim_orphaned(bench_id)?;
        loop {
            let attempt = match first.take() {
                Some(attempt) => attempt,
                None => match NotifyAttempt::begin(self.gate.as_ref()) {
                    Some(attempt) => attempt,
                    // 정지 판정이 끝났다(`stopping`): 새 전달을 시작하지 않는다.
                    None => break,
                },
            };
            let mut guard = AttemptGuard {
                attempt: Some(attempt),
                saved: false,
                done: false,
                bench_id: bench_id.to_owned(),
                events: self.events.clone(),
            };
            let attempt_id = guard.attempt_id().to_owned();
            // 저장소 경계는 각 단계 블록 안에서만 쥔다 — Main 턴을 기다리는 동안 쥐지 않는다(research R2·R9).
            let (notification_id, binding, snapshot) = {
                let mut tx = self.repository.begin()?;
                let sessions = tx.sessions();
                let session = session_for_bench_mut(sessions, bench_id)?;
                supersede_stale_notifications(session);
                if reactivate_failed {
                    for notification in &mut session.coordinator_notifications {
                        if notification.status == CoordinatorNotificationStatus::Failed
                            && notification
                                .failure
                                .as_ref()
                                .is_some_and(|failure| failure.retryable)
                        {
                            notification
                                .transition(CoordinatorNotificationStatus::Pending, now())?;
                        }
                    }
                    reactivate_failed = false;
                }
                let Some((notification_id, binding)) = next_delivery(session, bench_id)? else {
                    tx.commit()?;
                    guard.done = true;
                    break;
                };
                {
                    let notification = session
                        .coordinator_notifications
                        .iter_mut()
                        .find(|notification| notification.id == notification_id)
                        .ok_or_else(|| not_found("Coordinator notification"))?;
                    notification.attempt_count += 1;
                    notification.attempt_id = Some(attempt_id.clone());
                    notification.transition(CoordinatorNotificationStatus::Dispatching, now())?;
                }
                touch(session);
                let snapshot = session
                    .coordinator_notifications
                    .iter()
                    .find(|notification| notification.id == notification_id)
                    .cloned()
                    .ok_or_else(|| not_found("Coordinator notification"))?;
                tx.commit()?;
                guard.saved = true;
                (notification_id, binding, snapshot)
            };
            self.probe(DispatchPoint::AfterDispatchingSaved).await;
            let receipt = self.notifier.notify_coordinator(&binding, &snapshot).await;
            if self.probe(DispatchPoint::BeforeResultSave).await == DispatchAction::FailResultSave {
                return Err(OrchestrationError::new(
                    OrchestrationErrorCode::WorkerUnavailable,
                    "Injected result save failure.",
                )
                .retryable());
            }

            let notification = {
                let mut tx = self.repository.begin()?;
                let sessions = tx.sessions();
                let session = session_for_bench_mut(sessions, bench_id)?;
                let notification = {
                    let notification = session
                        .coordinator_notifications
                        .iter_mut()
                        .find(|candidate| candidate.id == notification_id)
                        .ok_or_else(|| not_found("Coordinator notification"))?;
                    // 이 시도가 소유한 `Dispatching`일 때만 결과를 쓴다(회수됐거나 다른 상태로 바뀌었으면 그대로 둔다).
                    let owned = notification.status == CoordinatorNotificationStatus::Dispatching
                        && notification.attempt_id.as_deref() == Some(attempt_id.as_str());
                    if owned {
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
                                // The Main run exists but declined delivery, so this is the
                                // "Main 사용 중" case rather than a missing runtime (FR-022).
                                notification.failure = Some(CommandFailure {
                                code: OrchestrationErrorCode::CoordinatorBusy,
                                message: receipt.reason.unwrap_or_else(|| {
                                    "Main이 보고 통지를 아직 받을 수 없습니다. 잠시 뒤 다시 전달합니다.".into()
                                }),
                                retryable: true,
                            });
                                notification
                                    .transition(CoordinatorNotificationStatus::Failed, now())?;
                                retryable_failures = true;
                            }
                            Err(error) => {
                                retryable_failures |= error.retryable;
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
                tx.commit()?;
                notification
            };
            // 결과 저장 commit 뒤에만 N-notify를 놓는다.
            guard.done = true;
            drop(guard);
            delivered.push(notification);
        }
        if let Some(events) = &self.events {
            events(
                bench_id,
                if retryable_failures {
                    DispatchEvent::RetryableFailures
                } else {
                    DispatchEvent::Settled
                },
            );
        }
        Ok(delivered)
    }

    /// 회수 한 바퀴(R14 표 6''): 시도의 N-notify 예약이 관문에 없는 `Dispatching{attemptId}`를 같은 id일 때만
    /// `Failed(retryable)`로 되돌린다. 살아 있는 시도는 건드리지 않는다. 관문이 없으면(재시작 복구 전용 조립) 하지 않는다.
    /// 예약 판정은 저장소 경계 밖에서 한다 — 시도의 예약은 결과 commit 뒤에만 풀리고 시도 id는 다시 쓰지 않으므로,
    /// 한 번 "예약 없음"이면 그 시도는 끝났다.
    pub fn reclaim_orphaned(&self, bench_id: &str) -> Result<usize, OrchestrationError> {
        let Some(gate) = &self.gate else {
            return Ok(0);
        };
        let candidates: Vec<(String, String)> = {
            let mut tx = self.repository.begin()?;
            let sessions = tx.sessions();
            let session = session_for_bench_mut(sessions, bench_id)?;
            session
                .coordinator_notifications
                .iter()
                .filter(|notification| {
                    notification.status == CoordinatorNotificationStatus::Dispatching
                })
                .filter_map(|notification| {
                    notification
                        .attempt_id
                        .clone()
                        .map(|attempt| (notification.id.clone(), attempt))
                })
                .collect()
        };
        let orphaned: Vec<(String, String)> = candidates
            .into_iter()
            .filter(|(_, attempt)| !gate.notify_attempt_live(attempt))
            .collect();
        if orphaned.is_empty() {
            return Ok(0);
        }
        let mut tx = self.repository.begin()?;
        let sessions = tx.sessions();
        let session = session_for_bench_mut(sessions, bench_id)?;
        let mut reclaimed = 0;
        for notification in &mut session.coordinator_notifications {
            let same_attempt = orphaned.iter().any(|(id, attempt)| {
                *id == notification.id && notification.attempt_id.as_deref() == Some(attempt)
            });
            if same_attempt && notification.status == CoordinatorNotificationStatus::Dispatching {
                notification.failure = Some(CommandFailure {
                    code: OrchestrationErrorCode::RuntimeLost,
                    message: "Main notification delivery was interrupted.".into(),
                    retryable: true,
                });
                notification.transition(CoordinatorNotificationStatus::Failed, now())?;
                reclaimed += 1;
            }
        }
        if reclaimed > 0 {
            touch(session);
            tx.commit()?;
        }
        Ok(reclaimed)
    }

    pub fn recover_interrupted(
        &self,
        bench_id: &str,
    ) -> Result<Vec<CoordinatorNotification>, OrchestrationError> {
        let mut tx = self.repository.begin()?;
        let sessions = tx.sessions();
        let session = session_for_bench_mut(sessions, bench_id)?;
        let mut recovered = Vec::new();
        for notification in &mut session.coordinator_notifications {
            if notification.status == CoordinatorNotificationStatus::Dispatching {
                notification.status = CoordinatorNotificationStatus::Pending;
                notification.attempt_id = None;
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
            tx.commit()?;
        }
        Ok(recovered)
    }
}

fn next_delivery(
    session: &OrchestrationSession,
    bench_id: &str,
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
            bench_id: bench_id.into(),
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

fn session_for_bench_mut<'a>(
    sessions: &'a mut [OrchestrationSession],
    bench_id: &str,
) -> Result<&'a mut OrchestrationSession, OrchestrationError> {
    let session = sessions
        .iter_mut()
        .find(|session| session.bound_bench_id.as_deref() == Some(bench_id))
        .ok_or_else(|| not_found("Orchestration workspace"))?;
    session.assert_scope(bench_id)?;
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

    use crate::infrastructure::orchestration::memory_store::InMemoryOrchestrationRepository as MemoryRepository;

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
            let mut tx = self.0.begin()?;
            let collected_at = now();
            let notification = tx.sessions()[0]
                .coordinator_notifications
                .iter_mut()
                .find(|candidate| candidate.id == notification.id)
                .unwrap();
            notification.collected_at = Some(collected_at.clone());
            notification.updated_at = collected_at;
            tx.commit()?;
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
                attempt_id: None,
                created_at: now.clone(),
                updated_at: now,
            });
        MemoryRepository::from_sessions(vec![session])
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
        assert!(dispatcher
            .dispatch_pending("window-1")
            .await
            .unwrap()
            .is_empty());
        assert_eq!(calls.lock().unwrap().as_slice(), ["notification-1"]);
    }

    #[tokio::test]
    async fn keeps_notifications_pending_while_main_is_unavailable() {
        let repository = repository(false);
        let dispatcher = CoordinatorNotificationDispatcher::new(
            repository.clone(),
            FakeNotifier(Arc::new(Mutex::new(vec![]))),
        );
        assert!(dispatcher
            .dispatch_pending("window-1")
            .await
            .unwrap()
            .is_empty());
        assert_eq!(
            repository.snapshot().unwrap()[0].coordinator_notifications[0].status,
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
            repository.snapshot().unwrap()[0].coordinator_notifications[0].status,
            CoordinatorNotificationStatus::Processed
        );
    }
}
