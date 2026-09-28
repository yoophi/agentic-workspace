//! 데스크톱 전달 포트(040, research R4). ADR 0003: 데스크톱은 구독자가 아니라 발행 결과를 받아 작업대의 창에
//! 넣는다. 구현은 AW(`TauriDesktopBridge`), 테스트는 기록형 가짜. 셋 다 `RuntimeAdapters`에서 선택이다(없으면 no-op).

use acp_agent_core::domain::run::AgentRunRequest;
use serde_json::Value;
use std::sync::Arc;

/// 작업대로 보낼 발행 결과. payload는 창이 오늘 받는 모양(공유 봉투의 상위 집합)이다.
#[derive(Debug, Clone, PartialEq)]
pub enum DesktopDelivery {
    Run {
        bench_id: String,
        payload: Value,
    },
    ExchangeRequested {
        bench_id: String,
        payload: Value,
    },
    ExchangeStatus {
        bench_id: String,
        payload: Value,
    },
    TitleRequested {
        bench_id: String,
        title: String,
    },
    /// 041: orchestration 작업 영역 갱신(`OrchestrationEvent` JSON + 순번 필드는 US3).
    Orchestration {
        bench_id: String,
        payload: Value,
    },
}

pub trait DesktopBridge: Send + Sync {
    /// 스트림 lock 안에서 불린다: 막히지 않아야 하고 hub·런타임을 다시 호출하면 안 된다.
    fn deliver(&self, delivery: DesktopDelivery);
}

/// run 종료 이벤트 뒤 후처리(041 전 과도기: worktree 가드 검사·orchestration 실패 처리). 발행 lock 밖에서 불린다.
pub trait RunTerminalHook: Send + Sync {
    fn on_terminal(&self, run_id: &str);
}

/// `run.start` 요청 보강(MCP 토큰·env, Main Coordinator principal). 데스크톱이 소유한 MCP 서버를 알기 때문에 AW가 구현한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchContext {
    pub bench_id: String,
    pub panel_id: Option<String>,
    pub run_id: String,
    /// 041: orchestration이 정한 이 run의 역할. decorator는 이것으로 MCP 권한을 만든다(창 label 역조회 없음).
    pub orchestration: Option<OrchestrationLaunchRole>,
}

/// 041: orchestration run의 역할(서버 상태에서 도출).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestrationLaunchRole {
    Coordinator {
        workspace_id: String,
        generation_id: String,
    },
    Child {
        workspace_id: String,
        node_id: String,
        task_id: String,
    },
}

pub trait RunLaunchDecorator: Send + Sync {
    fn decorate(
        &self,
        request: &mut AgentRunRequest,
        context: &LaunchContext,
    ) -> Result<(), String>;

    /// 041: 재시도·재배정으로 교체된 자식 run, 교대로 물러난 coordinator run의 MCP 토큰을 폐기한다(토큰 수명
    /// 관리일 뿐 권한 근거는 아니다 — 역할은 서버 상태로 판정한다).
    fn revoke_run(&self, _run_id: &str) {}
}

/// `decorate`가 발급한 run 자격을 엔진 기동 확정 전의 오류·future 취소에서 회수한다. 성공한 run은 terminal 경로가 회수한다.
pub struct PendingLaunchRevocation {
    decorator: Arc<dyn RunLaunchDecorator>,
    run_id: String,
    armed: bool,
}

impl PendingLaunchRevocation {
    pub fn armed(decorator: Arc<dyn RunLaunchDecorator>, run_id: impl Into<String>) -> Self {
        Self {
            decorator,
            run_id: run_id.into(),
            armed: true,
        }
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PendingLaunchRevocation {
    fn drop(&mut self) {
        if self.armed {
            self.decorator.revoke_run(&self.run_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use super::*;

    struct IssuingDecorator {
        issued: AtomicUsize,
        revoked: AtomicUsize,
    }

    impl RunLaunchDecorator for IssuingDecorator {
        fn decorate(
            &self,
            _request: &mut AgentRunRequest,
            _context: &LaunchContext,
        ) -> Result<(), String> {
            self.issued.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn revoke_run(&self, _run_id: &str) {
            self.revoked.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn cancellation_after_credential_issuance_rolls_the_pending_launch_back() {
        let recorder = Arc::new(IssuingDecorator {
            issued: AtomicUsize::new(0),
            revoked: AtomicUsize::new(0),
        });
        let decorator: Arc<dyn RunLaunchDecorator> = recorder.clone();
        let task = tokio::spawn(async move {
            let mut request: AgentRunRequest = serde_json::from_value(serde_json::json!({
                "goal": "g",
                "agentId": "codex",
                "runId": "run-cancelled"
            }))
            .unwrap();
            let context = LaunchContext {
                bench_id: "bench".into(),
                panel_id: None,
                run_id: "run-cancelled".into(),
                orchestration: None,
            };
            decorator.decorate(&mut request, &context).unwrap();
            let _rollback = PendingLaunchRevocation::armed(decorator, context.run_id);
            std::future::pending::<()>().await;
        });
        tokio::task::yield_now().await;
        assert_eq!(recorder.issued.load(Ordering::SeqCst), 1);
        task.abort();
        let _ = task.await;
        assert_eq!(recorder.revoked.load(Ordering::SeqCst), 1);
    }
}
