//! core 데스크톱 포트의 Tauri 구현(040, research R4·R10). 발행 결과를 작업대의 창에 **창 삽입 경로 하나로만** 넣는다
//! (ADR 0003·0004; 네이티브 `emit`은 모든 창에 방송되어 쓰지 않는다). run 종료 후처리와 `run.start` 보강은 041 전
//! 과도기로 AW에 남은 orchestration·MCP 서버에 기댄다.

use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tauri::{AppHandle, Manager};
use workbench_core::ports::desktop_bridge::{
    DesktopBridge, DesktopDelivery, LaunchContext, RunLaunchDecorator, RunTerminalHook,
};

use crate::{
    application::orchestration_service::OrchestrationService,
    domain::run::AgentRunRequest,
    inbound::tauri_commands::{orchestration_error, resolve_agent_run_launch_principal},
    infrastructure::{
        acp_agent_launch_factory::inject_mcp_launch_env,
        acp_agent_worker_adapter::{take_worktree_guard, verify_worktree_unchanged},
        desktop_benches,
        mcp::McpServerState,
        tauri_orchestration_event_sink::TauriOrchestrationEventSink,
    },
};

pub const AGENT_RUN_EVENT_FALLBACK: &str = "agent-run-event-fallback";
pub const AGENT_EXCHANGE_REQUESTED_FALLBACK: &str = "agent-exchange-requested-fallback";
pub const AGENT_EXCHANGE_STATUS_FALLBACK: &str = "agent-exchange-status-fallback";
pub const MCP_WINDOW_TITLE_FALLBACK: &str = "mcp-window-title-fallback";
/// 041: orchestration 갱신(오늘 `TauriOrchestrationEventSink`의 삽입 경로 이름). 네이티브 emit(전체 창 방송)은 없다.
pub const ORCHESTRATION_WORKSPACE_UPDATED_FALLBACK: &str = "orchestration-workspace-updated-fallback";
pub const ORCHESTRATION_COMMAND_UPDATED_FALLBACK: &str = "orchestration-command-updated-fallback";
pub const ORCHESTRATION_NOTIFICATION_UPDATED_FALLBACK: &str =
    "orchestration-coordinator-notification-updated-fallback";

/// 오늘과 같은 규칙: 사유에 command/notification이 들어 있으면 상세 이벤트도 한 번 보낸다.
pub fn orchestration_detail_event(reason: &str) -> Option<&'static str> {
    if reason.contains("command") || reason.contains("Command") {
        Some(ORCHESTRATION_COMMAND_UPDATED_FALLBACK)
    } else if reason.contains("notification") || reason.contains("Notification") {
        Some(ORCHESTRATION_NOTIFICATION_UPDATED_FALLBACK)
    } else {
        None
    }
}

pub fn dispatch_script(event_name: &str, payload: &Value) -> String {
    format!("window.dispatchEvent(new CustomEvent('{event_name}', {{ detail: {payload} }}));")
}

pub struct TauriDesktopBridge {
    app: AppHandle,
    /// 런타임은 MCP 서버보다 먼저 만들어진다(MCP가 런타임을 쓴다). 시작 뒤 `bind_mcp`로 묶는다.
    mcp: OnceLock<McpServerState>,
}

impl TauriDesktopBridge {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            mcp: OnceLock::new(),
        })
    }

    pub fn bind_mcp(&self, mcp: McpServerState) {
        let _ = self.mcp.set(mcp);
    }

    fn eval_in_bench(&self, bench_id: &str, event_name: &str, payload: &Value) {
        let Some(label) = desktop_benches::label_for(bench_id) else {
            return;
        };
        if let Some(window) = self.app.get_webview_window(&label) {
            let _ = window.eval(dispatch_script(event_name, payload));
        }
    }
}

impl DesktopBridge for TauriDesktopBridge {
    fn deliver(&self, delivery: DesktopDelivery) {
        match delivery {
            DesktopDelivery::Run { bench_id, payload } => {
                self.eval_in_bench(&bench_id, AGENT_RUN_EVENT_FALLBACK, &payload)
            }
            DesktopDelivery::ExchangeRequested { bench_id, payload } => {
                self.eval_in_bench(&bench_id, AGENT_EXCHANGE_REQUESTED_FALLBACK, &payload)
            }
            DesktopDelivery::ExchangeStatus { bench_id, payload } => {
                self.eval_in_bench(&bench_id, AGENT_EXCHANGE_STATUS_FALLBACK, &payload)
            }
            DesktopDelivery::Orchestration { bench_id, payload } => {
                self.eval_in_bench(&bench_id, ORCHESTRATION_WORKSPACE_UPDATED_FALLBACK, &payload);
                let reason = payload
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(event_name) = orchestration_detail_event(reason) {
                    self.eval_in_bench(&bench_id, event_name, &payload);
                }
            }
            DesktopDelivery::TitleRequested { bench_id, title } => {
                // `set_title`은 메인 스레드로 넘어가므로 스트림 lock 안에서 부르지 않는다.
                let Some(label) = desktop_benches::label_for(&bench_id) else {
                    return;
                };
                let app = self.app.clone();
                tauri::async_runtime::spawn(async move {
                    let Some(window) = app.get_webview_window(&label) else {
                        eprintln!("[workbench] title request for a closed window {label}");
                        return;
                    };
                    if let Err(error) = window.set_title(&title) {
                        eprintln!("[workbench] failed to apply title to {label}: {error}");
                        return;
                    }
                    let _ = crate::infrastructure::native_window_menu::sync_window_menu(&app);
                    let _ = window.eval(dispatch_script(
                        MCP_WINDOW_TITLE_FALLBACK,
                        &serde_json::json!({ "title": title }),
                    ));
                });
            }
        }
    }
}

impl RunTerminalHook for TauriDesktopBridge {
    /// orchestration 자식 run의 Worktree 변경 검사(오늘 `TauriRunEventSink`의 종료 처리와 같음, 041 전 과도기).
    fn on_terminal(&self, run_id: &str) {
        let Some(guard) = take_worktree_guard(run_id) else {
            return;
        };
        let Err(violation) = verify_worktree_unchanged(&guard.worktree_path, &guard.baseline)
        else {
            return;
        };
        let Ok(repository) = crate::inbound::tauri_commands::orchestration_repository(&self.app)
        else {
            return;
        };
        let service = OrchestrationService::new(
            repository,
            TauriOrchestrationEventSink::new(self.app.clone()),
        );
        let _ = service.fail_task_for_runtime(
            &guard.window_label,
            &guard.task_id,
            &guard.node_id,
            violation.code,
            &violation.message,
        );
    }
}

impl RunLaunchDecorator for TauriDesktopBridge {
    /// 오늘 `start_agent_run`의 보강: Main Coordinator 패널이면 orchestration principal, 아니면 run principal로
    /// MCP 토큰을 만들어 env·MCP 서버·안내문을 넣는다.
    fn decorate(
        &self,
        request: &mut AgentRunRequest,
        context: &LaunchContext,
    ) -> Result<(), String> {
        let mcp = self
            .mcp
            .get()
            .ok_or_else(|| "MCP server is not ready.".to_owned())?;
        let label = desktop_benches::label_for(&context.bench_id)
            .ok_or_else(|| desktop_benches::MESSAGE_WINDOW_UNAVAILABLE.to_owned())?;
        let principal = resolve_agent_run_launch_principal(
            &self.app,
            &label,
            context.panel_id.as_deref(),
            &context.run_id,
        )?;
        let env = match principal {
            Some(principal) => mcp
                .launch_env_for_principal(principal)
                .map_err(orchestration_error)?,
            None => mcp.launch_env(&context.run_id),
        };
        inject_mcp_launch_env(request, env);
        Ok(())
    }
}
