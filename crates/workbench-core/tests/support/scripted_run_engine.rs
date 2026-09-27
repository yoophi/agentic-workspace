//! 스크립트형 가짜 run 엔진(040, research R3·R13). 프로세스 없이 run 슬롯·소유·권한 대기를 메모리에서 흉내 내고,
//! 오류 문구는 acp-agent-core 유스케이스와 같다. fixture의 `runScript`로 동작을 바꾼다.

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

/// `send_and_wait` 턴 안에서 실행할 일(041 liveness ①): 가짜 agent가 턴 중에 도구를 부르는 것을 흉내 낸다.
pub type TurnHook = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

use acp_agent_core::{
    domain::{
        events::{LifecycleStatus, PermissionOption, RunEvent},
        run::{AgentRun, AgentRunRequest, PermissionMode},
    },
    ports::event_sink::RunEventSink,
};
use async_trait::async_trait;
use serde::Deserialize;
use workbench_core::{
    infrastructure::run::workbench_run_sink::WorkbenchRunSink,
    ports::run_engine::{RunEngine, RunEngineError, RunErrorKind},
};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunScript {
    /// 시작 직후 이 id로 권한 요청을 낸다.
    #[serde(default)]
    pub permission_id: Option<String>,
    /// `start`가 소유를 기록하기 전 지연(입장 구간을 늘려 닫기 경합을 재현).
    #[serde(default)]
    pub start_delay_ms: u64,
    /// prompt를 받아들이기 전 지연(042 연결 단절·종료 drain 시험: 호출이 진행 중인 구간을 늘린다).
    #[serde(default)]
    pub prompt_delay_ms: u64,
    /// prompt 효과(수 증가·이벤트) **뒤** 돌아가기 전 지연. 효과 뒤·멱등 기록 전에 연결이 끊기는 구간(research R17,
    /// 설계 리뷰 C1)을 재현한다.
    #[serde(default)]
    pub prompt_settle_ms: u64,
    /// run 슬롯을 만들고 Started를 낸 **뒤** 돌아가기 전 지연(`run.start`의 효과 뒤·기록 전 구간).
    #[serde(default)]
    pub start_settle_ms: u64,
    /// 동시 실행 상한.
    #[serde(default)]
    pub max_runs: Option<usize>,
}

struct Slot {
    owner: String,
    permissions: HashSet<String>,
}

#[derive(Default)]
pub struct ScriptedRunEngine {
    script: RunScript,
    runs: Mutex<HashMap<String, Slot>>,
    pub starts: AtomicUsize,
    pub prompts: AtomicUsize,
    pub turn_hook: Mutex<Option<TurnHook>>,
    /// `start`가 슬롯을 만든 뒤·돌아가기 전에 실행한다(자식 첫 턴이 바인딩 전에 도구를 부르는 경우).
    pub start_hook: Mutex<Option<TurnHook>>,
    /// 효과 표지(`start:<run>`, `prompt:<run>:<text>`). 효과가 난 직후·settle 지연 전에 기록된다(042 R17 시험 동기화).
    applied: Mutex<Vec<String>>,
    applied_notify: tokio::sync::Notify,
}

fn not_active() -> RunEngineError {
    RunEngineError::new(RunErrorKind::NotFound, "agent run is not active")
}

impl ScriptedRunEngine {
    pub fn new(script: RunScript) -> Self {
        Self {
            script,
            ..Self::default()
        }
    }

    fn active(&self, run_id: &str) -> bool {
        self.runs.lock().unwrap().contains_key(run_id)
    }

    /// run이 스스로 끝난 것처럼 슬롯을 지운다.
    pub fn finish(&self, run_id: &str, sink: &WorkbenchRunSink) {
        self.runs.lock().unwrap().remove(run_id);
        sink.emit(
            run_id,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Completed,
                message: "done".into(),
            },
        );
    }

    fn mark_applied(&self, label: String) {
        self.applied.lock().unwrap().push(label);
        self.applied_notify.notify_waiters();
    }

    /// `pred`에 맞는 효과 표지가 기록될 때까지 기다린다(settle 지연 구간 진입 확인).
    pub async fn wait_applied(&self, pred: impl Fn(&str) -> bool, wait: Duration) -> String {
        tokio::time::timeout(wait, async {
            loop {
                let notified = self.applied_notify.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                if let Some(found) = self.applied.lock().unwrap().iter().find(|l| pred(l)) {
                    return found.clone();
                }
                notified.await;
            }
        })
        .await
        .expect("effect marker within the wait")
    }

    pub fn run_count(&self) -> usize {
        self.runs.lock().unwrap().len()
    }

    pub fn runs_owned_by(&self, owner: &str) -> usize {
        self.runs
            .lock()
            .unwrap()
            .values()
            .filter(|slot| slot.owner == owner)
            .count()
    }
}

#[async_trait]
impl RunEngine for ScriptedRunEngine {
    async fn start(
        &self,
        request: AgentRunRequest,
        owner: &str,
        sink: WorkbenchRunSink,
    ) -> Result<AgentRun, RunEngineError> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        if self.script.start_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.script.start_delay_ms)).await;
        }
        let run_id = request.run_id.clone().expect("normalized run id");
        {
            let mut runs = self.runs.lock().unwrap();
            if runs.contains_key(&run_id) {
                return Err(RunEngineError::new(
                    RunErrorKind::Conflict,
                    format!("duplicate run id: {run_id}"),
                ));
            }
            if let Some(limit) = self.script.max_runs {
                if runs.len() >= limit {
                    return Err(RunEngineError::new(
                        RunErrorKind::RateLimited,
                        format!("concurrent run limit ({limit}) reached; cancel an existing run before starting a new one"),
                    ));
                }
            }
            let mut permissions = HashSet::new();
            if let Some(permission) = &self.script.permission_id {
                permissions.insert(permission.clone());
            }
            runs.insert(
                run_id.clone(),
                Slot {
                    owner: owner.to_owned(),
                    permissions,
                },
            );
        }
        sink.emit(
            &run_id,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Started,
                message: "started".into(),
            },
        );
        self.mark_applied(format!("start:{run_id}"));
        if self.script.start_settle_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.script.start_settle_ms)).await;
        }
        if let Some(permission) = &self.script.permission_id {
            sink.emit(
                &run_id,
                RunEvent::Permission {
                    permission_id: Some(permission.clone()),
                    title: "allow?".into(),
                    input: None,
                    options: vec![PermissionOption {
                        name: "Allow".into(),
                        kind: "allow_once".into(),
                        option_id: "allow".into(),
                    }],
                    selected: None,
                    requires_response: true,
                },
            );
        }
        let hook = self.start_hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook(run_id.clone()).await;
        }
        Ok(AgentRun {
            id: run_id,
            goal: request.goal,
            agent_id: request.agent_id,
        })
    }

    async fn send_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        if prompt.trim().is_empty() {
            return Err(RunEngineError::new(
                RunErrorKind::InvalidArgument,
                "prompt is empty",
            ));
        }
        if self.script.prompt_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.script.prompt_delay_ms)).await;
        }
        if !self.active(run_id) {
            return Err(not_active());
        }
        self.prompts.fetch_add(1, Ordering::SeqCst);
        self.mark_applied(format!("prompt:{run_id}:{prompt}"));
        sink.emit(run_id, RunEvent::AgentMessage { text: prompt });
        if self.script.prompt_settle_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.script.prompt_settle_ms)).await;
        }
        Ok(())
    }

    async fn queue_prompt(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        if !self.active(run_id) {
            return Err(RunEngineError::new(
                RunErrorKind::NotFound,
                format!("unknown or finished run: {run_id}"),
            ));
        }
        if self.script.prompt_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.script.prompt_delay_ms)).await;
        }
        self.prompts.fetch_add(1, Ordering::SeqCst);
        self.mark_applied(format!("prompt:{run_id}:{prompt}"));
        sink.emit(run_id, RunEvent::AgentMessage { text: prompt });
        if self.script.prompt_settle_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.script.prompt_settle_ms)).await;
        }
        Ok(())
    }

    async fn send_and_wait(
        &self,
        run_id: &str,
        prompt: String,
        _queue: bool,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        self.queue_prompt(run_id, prompt, sink).await?;
        let hook = self.turn_hook.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook(run_id.to_owned()).await;
        }
        Ok(())
    }

    async fn steer_prompt(
        &self,
        run_id: &str,
        prompt: String,
        _sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        if prompt.trim().is_empty() {
            return Err(RunEngineError::new(
                RunErrorKind::InvalidArgument,
                "steer prompt is empty",
            ));
        }
        if !self.active(run_id) {
            return Err(not_active());
        }
        Err(RunEngineError::new(
            RunErrorKind::PreconditionFailed,
            "steer unsupported: active-turn steer is not supported by this ACP agent; choose Cancel & send or Queue",
        ))
    }

    async fn cancel_current_prompt_and_send(
        &self,
        run_id: &str,
        prompt: String,
        sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        self.send_prompt(run_id, prompt, sink).await
    }

    async fn set_permission_mode(
        &self,
        run_id: &str,
        _mode: PermissionMode,
        _sink: WorkbenchRunSink,
    ) -> Result<(), RunEngineError> {
        if !self.active(run_id) {
            return Err(not_active());
        }
        Ok(())
    }

    async fn cancel(&self, run_id: &str, sink: WorkbenchRunSink) {
        let cancelled = self.runs.lock().unwrap().remove(run_id).is_some();
        sink.emit(
            run_id,
            RunEvent::Lifecycle {
                status: LifecycleStatus::Cancelled,
                message: if cancelled {
                    "run cancelled".into()
                } else {
                    "run was already terminated".into()
                },
            },
        );
    }

    async fn respond_permission(
        &self,
        run_id: &str,
        permission_id: &str,
        _option_id: &str,
    ) -> Result<(), RunEngineError> {
        let mut runs = self.runs.lock().unwrap();
        let removed = runs
            .get_mut(run_id)
            .map(|slot| slot.permissions.remove(permission_id))
            .unwrap_or(false);
        if removed {
            Ok(())
        } else {
            Err(RunEngineError::new(
                RunErrorKind::NotFound,
                format!("unknown or already answered permission: {permission_id}"),
            ))
        }
    }

    async fn owner_of(&self, run_id: &str) -> Option<String> {
        self.runs
            .lock()
            .unwrap()
            .get(run_id)
            .map(|slot| slot.owner.clone())
    }

    async fn active_owner_of(&self, run_id: &str) -> Option<String> {
        self.owner_of(run_id).await
    }

    async fn cancel_runs_owned_by(&self, owner: &str) -> Vec<String> {
        let mut runs = self.runs.lock().unwrap();
        let ids: Vec<String> = runs
            .iter()
            .filter(|(_, slot)| slot.owner == owner)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &ids {
            runs.remove(id);
        }
        let mut ids = ids;
        ids.sort();
        ids
    }
}
