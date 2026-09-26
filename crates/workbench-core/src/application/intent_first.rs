//! intent-first 변경 runner(research R5·R16). 모든 변경 operation이 공유하는 절차:
//!
//! 1. 멱등성 키 필수 검사 → 정규화 입력 지문 → ledger `find` → 재요청 판정(`idempotency::decide`)
//! 2. `pending` commit — 자원 예약 정책(`Reservation`)에 따라 `DuplicateReservation`을 처리
//! 3. aggregate lock 안에서: expectedRevision 검사 → `apply`(부작용) → ledger `complete`(+revision)
//! 4. 응답. 저장 뒤 확정 실패는 `pending`을 남기고 `outcome: unknown`으로 알린다(037 Codex 리뷰).
//!
//! `project_create.rs`의 절차를 추출한 것이며, 037 crash point·동시성·멱등성 테스트가 수정 없이 통과한다.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use chrono::Utc;
use serde::Serialize;
use workbench_protocol::{
    CallReply, OperationId, Outcome, RequestId, WorkbenchFault, CONTRACT_REVISION,
};

use crate::{
    application::{
        handlers::ledger_fault,
        idempotency::{decide, fingerprint, ReplayDecision},
        registry::CallContext,
        workbench_runtime::{CrashPoint, FailPoint, TestHooks},
    },
    infrastructure::{
        sqlite_ledger::SqliteOperationLedger, storage_coordinator::StorageCoordinator,
    },
    ports::operation_ledger::{LedgerError, LedgerKey, NewLedgerEntry, OperationLedger},
};

/// mutation N회마다 만료 결과를 정리한다.
const GC_EVERY_N_MUTATIONS: u64 = 50;
/// 서버가 만든 예약 id가 충돌하면 새 id로 다시 시도하는 횟수.
const RESERVATION_ATTEMPTS: usize = 3;

pub const MESSAGE_RESERVATION_IN_PROGRESS: &str =
    "Another change to this worktree path is still in progress.";
pub const MESSAGE_EXPECTED_REVISION_UNSUPPORTED: &str =
    "expectedRevision is not supported for git operations.";

/// 037의 `MESSAGE_KEY_REQUIRED`("idempotencyKey is required for project.create.")와 같은 형식.
pub fn key_required_message(operation: OperationId) -> String {
    format!("idempotencyKey is required for {operation}.")
}

/// 서버 생성 id 공급자(`Reservation::ServerGenerated`).
pub type GenerateIdFn = Box<dyn FnMut() -> Result<String, WorkbenchFault> + Send>;
/// lock 안 부작용. 복구 뒤 한 번 다시 불릴 수 있다.
pub type ApplyFn<Out, E> = Box<dyn FnMut(&ApplyContext<'_>) -> Result<Applied<Out>, E> + Send>;
/// 저장 파일 손상 시 lock 안에서 부르는 복구.
pub type RecoverFn<E> = Box<dyn Fn() -> Result<(), E> + Send>;
/// 도메인 오류 → Fault.
pub type FaultFn<E> = Box<dyn Fn(&RequestId, E) -> WorkbenchFault + Send>;

/// 자원 예약 정책(research R16).
pub enum Reservation {
    /// 예약 없음(upsert·수정). 재시작 판정은 `unknown`.
    None,
    /// 서버가 새 id를 만든다(`project-*`·`prompt-*`). `DuplicateReservation`이면 새 id로 최대 3회 재시도.
    ServerGenerated(GenerateIdFn),
    /// 호출자가 준 자원(worktree 경로, 삭제 대상 id). 충돌은 재시도 없이 `conflict` outcome `unknown`.
    CallerProvided(String),
}

/// lock 안 `apply`의 결과.
pub enum Applied<Out> {
    Ok(Out),
    /// 부작용 전 거절(검증·사전 조건). ledger는 `failed`로 닫힌다.
    Rejected(WorkbenchFault),
}

pub struct ApplyContext<'a> {
    /// 예약된 자원 id(`ServerGenerated`·`CallerProvided`일 때).
    pub reserved_id: Option<&'a str>,
    pub request_id: &'a RequestId,
}

pub struct MutationSpec<Out, E> {
    pub operation: OperationId,
    pub aggregate: String,
    /// 지문 계산용 정규화 입력(trim 등을 끝낸 값).
    pub normalized_input: serde_json::Value,
    pub reservation: Reservation,
    /// 저장 단위 aggregate면 true — `expectedRevision`을 지원하고 응답에 revision을 싣는다.
    /// Git 변경은 false — `expectedRevision`이 오면 `invalidArgument`, 응답 revision은 `null`.
    pub tracks_revision: bool,
    /// 부작용. 복구 뒤 한 번 다시 불릴 수 있으므로 `FnMut`이다.
    pub apply: ApplyFn<Out, E>,
    /// `apply`의 오류가 저장 파일 손상이면 true → lock 안에서 `recover` 뒤 재시도.
    pub is_store_corrupt: fn(&E) -> bool,
    pub recover: RecoverFn<E>,
    pub fault: FaultFn<E>,
}

pub struct IntentFirst {
    ledger: Arc<SqliteOperationLedger>,
    coordinator: Arc<StorageCoordinator>,
    hooks: Arc<TestHooks>,
    mutations: AtomicU64,
}

impl IntentFirst {
    pub fn new(
        ledger: Arc<SqliteOperationLedger>,
        coordinator: Arc<StorageCoordinator>,
        hooks: Arc<TestHooks>,
    ) -> Self {
        Self {
            ledger,
            coordinator,
            hooks,
            mutations: AtomicU64::new(0),
        }
    }

    pub fn ledger(&self) -> &Arc<SqliteOperationLedger> {
        &self.ledger
    }

    pub fn coordinator(&self) -> &Arc<StorageCoordinator> {
        &self.coordinator
    }

    pub fn hooks(&self) -> &Arc<TestHooks> {
        &self.hooks
    }

    fn note_mutation(&self) {
        let count = self.mutations.fetch_add(1, Ordering::SeqCst) + 1;
        if count.is_multiple_of(GC_EVERY_N_MUTATIONS) {
            let _ = self.ledger.gc_expired(Utc::now());
        }
    }

    /// blocking pool에서 `execute`를 돌린다. 성공하면 GC 카운터를 올린다.
    pub async fn run<Out, E>(
        &self,
        ctx: &CallContext,
        spec: MutationSpec<Out, E>,
    ) -> Result<CallReply, WorkbenchFault>
    where
        Out: Serialize + Send + 'static,
        E: Send + 'static,
    {
        let ledger = Arc::clone(&self.ledger);
        let coordinator = Arc::clone(&self.coordinator);
        let hooks = Arc::clone(&self.hooks);
        let ctx = ctx.clone();
        let request_id = ctx.request_id.clone();
        let result =
            tokio::task::spawn_blocking(move || execute(&ledger, &coordinator, &hooks, &ctx, spec))
                .await
                .map_err(|error| WorkbenchFault::internal(request_id, error.to_string()))?;

        if result.is_ok() {
            self.note_mutation();
        }
        result
    }
}

/// lock 안에서 실행되는 단계의 결과.
enum Locked {
    Applied {
        output: serde_json::Value,
        revision: u64,
    },
    /// 저장 전 거절. ledger는 `failed`로 닫는다.
    Rejected(WorkbenchFault),
    /// 부작용은 끝났지만 ledger `complete`가 실패했다. row를 `pending`으로 남겨 재시작 시 reconciler가
    /// 판정하게 하고, 호출자에게는 `outcome: unknown`을 알린다. `failed`로 닫으면 적용된 변경이
    /// "적용 안 됨"으로 영구 기록되어 재시도가 거짓 실패를 재생하고 새 키로 중복이 생긴다.
    SavedButUnconfirmed(String),
    Crashed,
}

fn without_revision(reply: CallReply) -> CallReply {
    match reply {
        CallReply::Complete { output, .. } => CallReply::complete(output, None),
        other => other,
    }
}

fn crash_fault(request_id: &RequestId) -> WorkbenchFault {
    WorkbenchFault::internal(request_id.clone(), "crash injected")
}

pub fn execute<Out, E>(
    ledger: &SqliteOperationLedger,
    coordinator: &StorageCoordinator,
    hooks: &TestHooks,
    ctx: &CallContext,
    mut spec: MutationSpec<Out, E>,
) -> Result<CallReply, WorkbenchFault>
where
    Out: Serialize,
{
    let request_id = &ctx.request_id;
    let key = ctx.idempotency_key.clone().ok_or_else(|| {
        WorkbenchFault::invalid_argument(
            request_id.clone(),
            key_required_message(spec.operation),
            Some("/idempotencyKey"),
        )
    })?;
    if !spec.tracks_revision && ctx.expected_revision.is_some() {
        return Err(WorkbenchFault::invalid_argument(
            request_id.clone(),
            MESSAGE_EXPECTED_REVISION_UNSUPPORTED,
            Some("/expectedRevision"),
        ));
    }

    let fp = fingerprint(spec.operation, &spec.normalized_input);
    let ledger_key = LedgerKey {
        principal_kind: ctx.principal.kind,
        operation: spec.operation,
        contract_revision: CONTRACT_REVISION,
        idempotency_key: key,
    };

    let tracks_revision = spec.tracks_revision;
    let replay = |ledger: &SqliteOperationLedger| -> Result<Option<Result<CallReply, WorkbenchFault>>, WorkbenchFault> {
        let existing = ledger
            .find(&ledger_key)
            .map_err(|error| ledger_fault(request_id, error))?;
        Ok(match decide(existing.as_ref(), &fp, request_id) {
            ReplayDecision::Proceed => None,
            // ledger는 모든 aggregate에 revision을 매기지만, 저장 단위가 아닌 Git 변경은 첫 응답처럼 revision을 싣지 않는다.
            ReplayDecision::ReturnStored(result) if !tracks_revision => Some(result.map(without_revision)),
            ReplayDecision::ReturnStored(result) => Some(result),
            ReplayDecision::Conflict(fault) => Some(Err(fault)),
        })
    };

    if let Some(result) = replay(ledger)? {
        return result;
    }

    // 2. pending — 예약 정책(R16)
    let mut attempt = 0;
    let (reserved_id, execution_id) = loop {
        attempt += 1;
        let reserved = match &mut spec.reservation {
            Reservation::None => None,
            Reservation::ServerGenerated(generate) => Some(generate()?),
            Reservation::CallerProvided(id) => Some(id.clone()),
        };
        match ledger.begin(NewLedgerEntry {
            key: ledger_key.clone(),
            input_fingerprint: fp.clone(),
            aggregate: spec.aggregate.clone(),
            reserved_resource_id: reserved.clone(),
            request_id: request_id.clone(),
        }) {
            Ok(execution_id) => break (reserved, execution_id),
            // 같은 키가 방금 다른 호출자에 의해 시작됐다. 그 기록을 기준으로 다시 판정한다.
            Err(LedgerError::DuplicateKey) => {
                return replay(ledger)?.unwrap_or_else(|| {
                    Err(WorkbenchFault::internal(
                        request_id.clone(),
                        "멱등성 기록 경합을 해소할 수 없습니다.",
                    ))
                });
            }
            Err(LedgerError::DuplicateReservation) => match &spec.reservation {
                // 서버 id(`project-{nanos}`)가 같은 나노초에 떨어진 경우: id만 다시 뽑는다.
                Reservation::ServerGenerated(_) if attempt < RESERVATION_ATTEMPTS => {
                    std::thread::yield_now();
                    continue;
                }
                // 호출자가 준 자원은 다른 실행이 잡고 있다. 그 실행이 끝나면(종료 상태) 예약이 풀린다.
                Reservation::CallerProvided(_) => {
                    return Err(WorkbenchFault::conflict(
                        request_id.clone(),
                        MESSAGE_RESERVATION_IN_PROGRESS,
                        Outcome::Unknown,
                    )
                    .with_retryable(true));
                }
                _ => return Err(ledger_fault(request_id, LedgerError::DuplicateReservation)),
            },
            Err(error) => return Err(ledger_fault(request_id, error)),
        }
    };

    if hooks.crash_if(CrashPoint::AfterPending).is_err() {
        return Err(crash_fault(request_id));
    }

    // 3. lock 안: expectedRevision → apply → complete
    let expected_revision = ctx.expected_revision;
    let aggregate = spec.aggregate.clone();
    let apply_ctx = ApplyContext {
        reserved_id: reserved_id.as_deref(),
        request_id,
    };
    let apply = &mut spec.apply;
    let locked: Result<Locked, E> = coordinator.with_aggregate(
        &aggregate,
        || {
            if tracks_revision {
                if let Some(expected) = expected_revision {
                    let current = coordinator.revision_of(&aggregate);
                    if expected != current {
                        return Ok(Locked::Rejected(WorkbenchFault::precondition_failed(
                            request_id.clone(),
                            expected,
                            current,
                        )));
                    }
                }
            }
            let output = match apply(&apply_ctx)? {
                Applied::Rejected(fault) => return Ok(Locked::Rejected(fault)),
                Applied::Ok(output) => output,
            };
            // 여기부터는 부작용이 끝났다. 어떤 실패도 `notApplied`로 보고하면 안 된다.
            if hooks.crash_if(CrashPoint::AfterJsonSave).is_err()
                || hooks.crash_if(CrashPoint::BeforeApplied).is_err()
            {
                return Ok(Locked::Crashed);
            }
            if hooks.should_fail(FailPoint::LedgerComplete) {
                return Ok(Locked::SavedButUnconfirmed(
                    "injected ledger completion failure".to_owned(),
                ));
            }
            let output = serde_json::to_value(&output).expect("output serializes");
            match ledger.complete(&execution_id, &output) {
                Ok(revision) => {
                    if tracks_revision {
                        coordinator.set_revision_of(&aggregate, revision);
                    }
                    Ok(Locked::Applied { output, revision })
                }
                Err(error) => Ok(Locked::SavedButUnconfirmed(error.to_string())),
            }
        },
        spec.is_store_corrupt,
        &*spec.recover,
    );

    // 4. 응답
    match locked {
        Ok(Locked::Applied { output, revision }) => Ok(CallReply::complete(
            output,
            tracks_revision.then_some(revision),
        )),
        Ok(Locked::Crashed) => Err(crash_fault(request_id)),
        // ledger는 `pending` 그대로. 같은 키 재요청은 진행 중 충돌(unknown)로 응답되고, 다음 기동의
        // reconciler가 판정한다.
        Ok(Locked::SavedButUnconfirmed(cause)) => Err(WorkbenchFault::unavailable(
            request_id.clone(),
            format!(
                "변경은 저장되었지만 변경 기록 확정에 실패했습니다: {cause}. 다음 실행 시 자동으로 판정됩니다."
            ),
        )
        .with_outcome(Outcome::Unknown)),
        Ok(Locked::Rejected(fault)) => {
            // fail()이 실패해도 호출자에게는 원래 Fault를 돌려준다. row는 pending으로 남고, 부작용이 없으므로
            // 다음 기동의 reconciler가 unknown으로 닫는다. 진단 로깅은 2단계에서 붙인다.
            let _ = ledger.fail(
                &execution_id,
                &serde_json::to_value(&fault).expect("fault serializes"),
            );
            Err(fault)
        }
        Err(error) => {
            let fault = (spec.fault)(request_id, error);
            // 부작용 전 실패(load/save 오류). fail() 실패 시의 동작은 위 Rejected 분기와 같다.
            let _ = ledger.fail(
                &execution_id,
                &serde_json::to_value(&fault).expect("fault serializes"),
            );
            Err(fault)
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::{
        AuthenticatedPrincipal, FaultCode, IdempotencyKey, OperationId, RequestId,
    };

    use super::*;
    use crate::infrastructure::{
        data_paths::DataPaths,
        storage_coordinator::{Repositories, GOALS_AGGREGATE},
    };

    fn runtime_parts() -> (
        tempfile::TempDir,
        Arc<SqliteOperationLedger>,
        Arc<StorageCoordinator>,
        Arc<TestHooks>,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path());
        paths.ensure_dirs().unwrap();
        let ledger = Arc::new(SqliteOperationLedger::open(&paths).unwrap());
        ledger.migrate().unwrap();
        let coordinator = Arc::new(StorageCoordinator::new(Repositories::json(&paths)));
        (dir, ledger, coordinator, Arc::new(TestHooks::default()))
    }

    fn ctx(key: Option<&str>, expected_revision: Option<u64>) -> CallContext {
        CallContext {
            principal: AuthenticatedPrincipal::desktop(),
            request_id: RequestId::new("r1").unwrap(),
            idempotency_key: key.map(|key| IdempotencyKey::new(key).unwrap()),
            expected_revision,
        }
    }

    fn spec(
        aggregate: &str,
        reservation: Reservation,
        tracks_revision: bool,
    ) -> MutationSpec<serde_json::Value, String> {
        MutationSpec {
            operation: OperationId::ProjectCreate,
            aggregate: aggregate.to_owned(),
            normalized_input: json!({"a": 1}),
            reservation,
            tracks_revision,
            apply: Box::new(|ctx| Ok(Applied::Ok(json!({ "reserved": ctx.reserved_id })))),
            is_store_corrupt: |_| false,
            recover: Box::new(|| Ok(())),
            fault: Box::new(|rid, error| WorkbenchFault::internal(rid.clone(), error)),
        }
    }

    #[test]
    fn key_is_required_with_operation_specific_message() {
        let (_dir, ledger, coordinator, hooks) = runtime_parts();
        let fault = execute(
            &ledger,
            &coordinator,
            &hooks,
            &ctx(None, None),
            spec(GOALS_AGGREGATE, Reservation::None, true),
        )
        .unwrap_err();
        assert_eq!(fault.code, FaultCode::InvalidArgument);
        assert_eq!(
            fault.message,
            "idempotencyKey is required for project.create."
        );
    }

    #[test]
    fn caller_provided_reservation_conflicts_immediately_while_pending() {
        let (_dir, ledger, coordinator, hooks) = runtime_parts();
        // 다른 실행이 같은 자원을 pending으로 잡고 있다.
        let mut other = ctx(Some("other"), None);
        other.request_id = RequestId::new("r0").unwrap();
        ledger
            .begin(NewLedgerEntry {
                key: LedgerKey {
                    principal_kind: other.principal.kind,
                    operation: OperationId::ProjectCreate,
                    contract_revision: CONTRACT_REVISION,
                    idempotency_key: other.idempotency_key.clone().unwrap(),
                },
                input_fingerprint: "x".into(),
                aggregate: "git-worktrees:/repo".into(),
                reserved_resource_id: Some("/repo-worktrees/a".into()),
                request_id: other.request_id.clone(),
            })
            .unwrap();

        let fault = execute(
            &ledger,
            &coordinator,
            &hooks,
            &ctx(Some("mine"), None),
            spec(
                "git-worktrees:/repo",
                Reservation::CallerProvided("/repo-worktrees/a".into()),
                false,
            ),
        )
        .unwrap_err();
        assert_eq!(fault.code, FaultCode::Conflict);
        assert_eq!(fault.outcome, Outcome::Unknown);
        assert!(fault.retryable);
        assert_eq!(fault.message, MESSAGE_RESERVATION_IN_PROGRESS);
    }

    #[test]
    fn server_generated_reservation_retries_then_conflicts() {
        let (_dir, ledger, coordinator, hooks) = runtime_parts();
        let mut other = ctx(Some("other"), None);
        other.request_id = RequestId::new("r0").unwrap();
        ledger
            .begin(NewLedgerEntry {
                key: LedgerKey {
                    principal_kind: other.principal.kind,
                    operation: OperationId::ProjectCreate,
                    contract_revision: CONTRACT_REVISION,
                    idempotency_key: other.idempotency_key.clone().unwrap(),
                },
                input_fingerprint: "x".into(),
                aggregate: GOALS_AGGREGATE.into(),
                reserved_resource_id: Some("fixed".into()),
                request_id: other.request_id.clone(),
            })
            .unwrap();
        let calls = Arc::new(AtomicU64::new(0));
        let counter = Arc::clone(&calls);
        let fault = execute(
            &ledger,
            &coordinator,
            &hooks,
            &ctx(Some("mine"), None),
            spec(
                GOALS_AGGREGATE,
                Reservation::ServerGenerated(Box::new(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok("fixed".to_owned()) // 항상 같은 id를 만들어 3회 모두 충돌
                })),
                true,
            ),
        )
        .unwrap_err();
        assert_eq!(calls.load(Ordering::SeqCst), RESERVATION_ATTEMPTS as u64);
        assert_eq!(fault.code, FaultCode::Internal);
    }

    #[test]
    fn expected_revision_policy_depends_on_tracks_revision() {
        let (_dir, ledger, coordinator, hooks) = runtime_parts();
        // Git 변경: expectedRevision 자체가 거절되고 ledger에 아무 것도 남지 않는다.
        let fault = execute(
            &ledger,
            &coordinator,
            &hooks,
            &ctx(Some("k1"), Some(0)),
            spec(
                "git-worktrees:/repo",
                Reservation::CallerProvided("/p".into()),
                false,
            ),
        )
        .unwrap_err();
        assert_eq!(fault.code, FaultCode::InvalidArgument);
        assert_eq!(fault.message, MESSAGE_EXPECTED_REVISION_UNSUPPORTED);
        assert_eq!(
            ledger
                .count_by_state(crate::ports::operation_ledger::LedgerState::Pending)
                .unwrap(),
            0
        );

        // 저장 단위: 맞는 revision은 통과하고 응답에 revision이 실린다.
        let reply = execute(
            &ledger,
            &coordinator,
            &hooks,
            &ctx(Some("k2"), Some(0)),
            spec(GOALS_AGGREGATE, Reservation::None, true),
        )
        .unwrap();
        assert_eq!(reply.revision(), Some(1));
        assert_eq!(coordinator.revision_of(GOALS_AGGREGATE), 1);

        // stale
        let fault = execute(
            &ledger,
            &coordinator,
            &hooks,
            &ctx(Some("k3"), Some(0)),
            spec(GOALS_AGGREGATE, Reservation::None, true),
        )
        .unwrap_err();
        assert_eq!(fault.code, FaultCode::PreconditionFailed);

        // Git 변경 성공 응답은 revision null
        let reply = execute(
            &ledger,
            &coordinator,
            &hooks,
            &ctx(Some("k4"), None),
            spec(
                "git-worktrees:/repo",
                Reservation::CallerProvided("/p".into()),
                false,
            ),
        )
        .unwrap();
        assert_eq!(reply.revision(), None);
        assert_eq!(reply.output(), Some(&json!({"reserved": "/p"})));
    }
}
