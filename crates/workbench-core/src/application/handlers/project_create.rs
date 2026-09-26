//! `project.create`: intent-first 경로(research R6).
//!
//! 1. 정규화 → 지문 → ledger `find` → 재요청 판정
//! 2. `pending` commit(예약 id 포함)
//! 3. aggregate lock 안에서: expectedRevision 검사 → JSON load·push·atomic save → ledger `applied`(+revision)
//! 4. 응답
//!
//! crash point는 `test-hooks` feature에서만 동작한다. crash가 주입되면 ledger를 더 건드리지 않고 즉시 돌아간다
//! (실제 프로세스 종료를 흉내 낸다).

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use async_trait::async_trait;
use chrono::Utc;
use workbench_protocol::{
    operations::project::ProjectCreateInput, CallReply, OperationId, Outcome, RequestId,
    WorkbenchFault, CONTRACT_REVISION,
};

use crate::{
    application::{
        handlers::{ledger_fault, project_fault, to_dto},
        idempotency::{decide, fingerprint, ReplayDecision},
        project_service,
        registry::{decode_input, CallContext, OperationHandler},
        workbench_runtime::{CrashPoint, FailPoint, TestHooks},
    },
    domain::project::ProjectDraft,
    infrastructure::{
        sqlite_ledger::SqliteOperationLedger,
        storage_coordinator::{StorageCoordinator, PROJECTS_AGGREGATE},
    },
    ports::operation_ledger::{LedgerError, LedgerKey, NewLedgerEntry, OperationLedger},
};

/// mutation N회마다 만료 결과를 정리한다.
const GC_EVERY_N_MUTATIONS: u64 = 50;

pub const MESSAGE_KEY_REQUIRED: &str = "idempotencyKey is required for project.create.";

pub struct ProjectCreateHandler {
    ledger: Arc<SqliteOperationLedger>,
    coordinator: Arc<StorageCoordinator>,
    hooks: Arc<TestHooks>,
    mutations: AtomicU64,
}

impl ProjectCreateHandler {
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

    fn note_mutation(&self) {
        let count = self.mutations.fetch_add(1, Ordering::SeqCst) + 1;
        if count.is_multiple_of(GC_EVERY_N_MUTATIONS) {
            let _ = self.ledger.gc_expired(Utc::now());
        }
    }
}

/// lock 안에서 실행되는 단계의 결과. 바깥 `Result`의 `ProjectError`는 coordinator가 복구 재시도 판단에 쓴다.
enum Locked {
    Applied {
        project: crate::domain::project::Project,
        revision: u64,
    },
    /// 저장 전 거절(expectedRevision 불일치). ledger는 `failed`로 닫는다.
    Rejected(WorkbenchFault),
    /// JSON 저장은 끝났지만 ledger `complete`가 실패했다. row를 `pending`으로 남겨 재시작 시 reconciler가
    /// 판정하게 하고, 호출자에게는 `outcome: unknown`을 알린다. `failed`로 닫으면 저장된 프로젝트가
    /// "적용 안 됨"으로 영구 기록되어 재시도가 거짓 실패를 재생하고 새 키로 중복 생성이 일어난다.
    SavedButUnconfirmed(String),
    Crashed,
}

fn execute(
    ledger: &SqliteOperationLedger,
    coordinator: &StorageCoordinator,
    hooks: &TestHooks,
    ctx: &CallContext,
    draft: ProjectDraft,
) -> Result<CallReply, WorkbenchFault> {
    let request_id = &ctx.request_id;
    let key = ctx.idempotency_key.clone().ok_or_else(|| {
        WorkbenchFault::invalid_argument(
            request_id.clone(),
            MESSAGE_KEY_REQUIRED,
            Some("/idempotencyKey"),
        )
    })?;

    let normalized = serde_json::json!({
        "name": draft.name,
        "workingDirectory": draft.working_directory,
        "description": draft.description,
    });
    let fp = fingerprint(OperationId::ProjectCreate, &normalized);
    let ledger_key = LedgerKey {
        principal_kind: ctx.principal.kind,
        operation: OperationId::ProjectCreate,
        contract_revision: CONTRACT_REVISION,
        idempotency_key: key,
    };

    let replay = |ledger: &SqliteOperationLedger| -> Result<Option<Result<CallReply, WorkbenchFault>>, WorkbenchFault> {
        let existing = ledger
            .find(&ledger_key)
            .map_err(|error| ledger_fault(request_id, error))?;
        Ok(match decide(existing.as_ref(), &fp, request_id) {
            ReplayDecision::Proceed => None,
            ReplayDecision::ReturnStored(result) => Some(result),
            ReplayDecision::Conflict(fault) => Some(Err(fault)),
        })
    };

    if let Some(result) = replay(ledger)? {
        return result;
    }

    // 예약 id는 `project-{unix_nanos}`라 동시 호출이 같은 나노초에 떨어지면 `DuplicateReservation`이 난다.
    // 그 경우 id만 다시 뽑아 몇 번 재시도한다(기존 id 형식은 저장 파일 호환 때문에 유지).
    const RESERVATION_ATTEMPTS: usize = 3;
    let mut attempt = 0;
    let (id, execution_id) = loop {
        attempt += 1;
        let id =
            project_service::new_project_id().map_err(|error| project_fault(request_id, error))?;
        match ledger.begin(NewLedgerEntry {
            key: ledger_key.clone(),
            input_fingerprint: fp.clone(),
            aggregate: PROJECTS_AGGREGATE.to_owned(),
            reserved_resource_id: Some(id.clone()),
            request_id: request_id.clone(),
        }) {
            Ok(execution_id) => break (id, execution_id),
            // 같은 키가 방금 다른 호출자에 의해 시작됐다. 그 기록을 기준으로 다시 판정한다.
            Err(LedgerError::DuplicateKey) => {
                return replay(ledger)?.unwrap_or_else(|| {
                    Err(WorkbenchFault::internal(
                        request_id.clone(),
                        "멱등성 기록 경합을 해소할 수 없습니다.",
                    ))
                });
            }
            Err(LedgerError::DuplicateReservation) if attempt < RESERVATION_ATTEMPTS => {
                std::thread::yield_now();
                continue;
            }
            Err(error) => return Err(ledger_fault(request_id, error)),
        }
    };

    if hooks.crash_if(CrashPoint::AfterPending).is_err() {
        return Err(crash_fault(request_id));
    }

    let expected_revision = ctx.expected_revision;
    let locked = coordinator.with_projects(|repo| {
        if let Some(expected) = expected_revision {
            let current = coordinator.revision();
            if expected != current {
                return Ok(Locked::Rejected(WorkbenchFault::precondition_failed(
                    request_id.clone(),
                    expected,
                    current,
                )));
            }
        }
        let project = project_service::create_project_with_id(repo, id.clone(), draft.clone())?;
        // 여기부터는 부작용(JSON 저장)이 끝났다. 어떤 실패도 `notApplied`로 보고하면 안 된다.
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
        match ledger.complete(
            &execution_id,
            &serde_json::to_value(to_dto(&project)).expect("dto serializes"),
        ) {
            Ok(revision) => {
                coordinator.set_revision(revision);
                Ok(Locked::Applied { project, revision })
            }
            Err(error) => Ok(Locked::SavedButUnconfirmed(error.to_string())),
        }
    });

    match locked {
        Ok(Locked::Applied { project, revision }) => Ok(CallReply::complete(
            serde_json::to_value(to_dto(&project)).expect("dto serializes"),
            Some(revision),
        )),
        Ok(Locked::Crashed) => Err(crash_fault(request_id)),
        // ledger는 `pending` 그대로. 같은 키 재요청은 진행 중 충돌(unknown)로 응답되고, 다음 기동의
        // reconciler가 JSON에서 예약 id를 찾아 `applied`로 확정한다.
        Ok(Locked::SavedButUnconfirmed(cause)) => Err(WorkbenchFault::unavailable(
            request_id.clone(),
            format!(
                "프로젝트는 저장되었지만 변경 기록 확정에 실패했습니다: {cause}. 다음 실행 시 자동으로 판정됩니다."
            ),
        )
        .with_outcome(Outcome::Unknown)),
        Ok(Locked::Rejected(fault)) => {
            // fail()이 실패해도 호출자에게는 원래 Fault를 돌려준다. row는 pending으로 남고, 부작용이 없으므로
            // 다음 기동의 reconciler가 JSON에서 id를 찾지 못해 unknown으로 닫는다. 진단 로깅은 2단계에서 붙인다.
            let _ = ledger.fail(
                &execution_id,
                &serde_json::to_value(&fault).expect("fault serializes"),
            );
            Err(fault)
        }
        Err(error) => {
            let fault = project_fault(request_id, error);
            // 저장 전 실패(load/save 오류). fail() 실패 시의 동작은 위 Rejected 분기와 같다.
            let _ = ledger.fail(
                &execution_id,
                &serde_json::to_value(&fault).expect("fault serializes"),
            );
            Err(fault)
        }
    }
}

fn crash_fault(request_id: &RequestId) -> WorkbenchFault {
    WorkbenchFault::internal(request_id.clone(), "crash injected")
}

#[async_trait]
impl OperationHandler for ProjectCreateHandler {
    async fn handle(
        &self,
        ctx: &CallContext,
        input: serde_json::Value,
    ) -> Result<CallReply, WorkbenchFault> {
        let input: ProjectCreateInput = decode_input(&ctx.request_id, &input)?;
        let draft = project_service::normalize_draft(ProjectDraft {
            name: input.name,
            working_directory: input.working_directory,
            description: input.description,
        })
        .map_err(|error| project_fault(&ctx.request_id, error))?;

        let ledger = Arc::clone(&self.ledger);
        let coordinator = Arc::clone(&self.coordinator);
        let hooks = Arc::clone(&self.hooks);
        let ctx = ctx.clone();
        let request_id = ctx.request_id.clone();
        let result = tokio::task::spawn_blocking(move || {
            execute(&ledger, &coordinator, &hooks, &ctx, draft)
        })
        .await
        .map_err(|error| WorkbenchFault::internal(request_id, error.to_string()))?;

        if result.is_ok() {
            self.note_mutation();
        }
        result
    }
}
