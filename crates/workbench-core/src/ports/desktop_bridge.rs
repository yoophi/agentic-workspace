//! 데스크톱 전달 포트(040, research R4). ADR 0003: 데스크톱은 구독자가 아니라 발행 결과를 받아 작업대의 창에
//! 넣는다. 구현은 AW(`TauriDesktopBridge`), 테스트는 기록형 가짜. 셋 다 `RuntimeAdapters`에서 선택이다(없으면 no-op).

use acp_agent_core::domain::run::AgentRunRequest;
use serde_json::Value;

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

    /// 041: 재시도·재배정으로 교체된 자식 run의 MCP 토큰을 폐기한다(토큰 수명 관리, 권한 근거는 아님).
    fn revoke_run(&self, _run_id: &str) {}

    /// 041: coordinator 교대로 끝난 세대의 MCP 토큰을 폐기한다.
    fn revoke_generation(&self, _workspace_id: &str, _generation_id: &str) {}
}
