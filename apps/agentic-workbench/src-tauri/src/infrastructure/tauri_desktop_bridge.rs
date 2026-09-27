//! core 데스크톱 포트의 Tauri 구현(040, research R4·R10). 발행 결과를 작업대의 창에 **창 삽입 경로 하나로만** 넣는다
//! (ADR 0003·0004; 네이티브 `emit`은 모든 창에 방송되어 쓰지 않는다). `run.start` 보강은 run에 묶인 MCP 토큰만
//! 만든다 — orchestration 역할은 서버 상태로 판정한다(041 research R7). run 종료 후처리는 core가 한다.

use std::sync::{Arc, OnceLock};

use serde_json::Value;
use tauri::{AppHandle, Manager};
use workbench_core::ports::desktop_bridge::{
    DesktopBridge, DesktopDelivery, LaunchContext, RunLaunchDecorator,
};

use crate::{
    domain::run::AgentRunRequest,
    infrastructure::{
        acp_agent_launch_factory::inject_mcp_launch_env, desktop_benches, mcp::McpServerState,
    },
};

pub const AGENT_RUN_EVENT_FALLBACK: &str = "agent-run-event-fallback";
pub const AGENT_EXCHANGE_REQUESTED_FALLBACK: &str = "agent-exchange-requested-fallback";
pub const AGENT_EXCHANGE_STATUS_FALLBACK: &str = "agent-exchange-status-fallback";
pub const MCP_WINDOW_TITLE_FALLBACK: &str = "mcp-window-title-fallback";
/// 041: orchestration 갱신(오늘 `TauriOrchestrationEventSink`의 삽입 경로 이름). 네이티브 emit(전체 창 방송)은 없다.
pub const ORCHESTRATION_WORKSPACE_UPDATED_FALLBACK: &str =
    "orchestration-workspace-updated-fallback";
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

/// 네트워크 경로로 이벤트를 받는 창(043, research R4): `(label, incarnation)`. 이 창에는 앱 내부 삽입 전달을 하지 않는다
/// (구독과 중복 금지, FR-007). incarnation까지 맞춰야 하므로 같은 label로 다시 만든 창(선언 전)은 전달을 받는다.
fn network_windows() -> std::sync::MutexGuard<'static, std::collections::HashMap<String, String>> {
    static TABLE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, String>>> =
        OnceLock::new();
    TABLE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 창이 네트워크 경로로 이벤트를 받겠다고 선언한다. 창의 **현재** incarnation일 때만 받아들인다.
pub fn declare_network_delivery(label: &str, incarnation: &str) -> Result<(), String> {
    if crate::infrastructure::window_principals::incarnation(label).as_deref() != Some(incarnation)
    {
        return Err(
            crate::infrastructure::window_principals::MESSAGE_WINDOW_NOT_REGISTERED.to_owned(),
        );
    }
    network_windows().insert(label.to_owned(), incarnation.to_owned());
    Ok(())
}

/// 창 `Destroyed`: 그 incarnation의 선언만 지운다.
pub fn forget_network_delivery(label: &str, incarnation: &str) {
    let mut table = network_windows();
    if table
        .get(label)
        .is_some_and(|current| current == incarnation)
    {
        table.remove(label);
    }
}

/// 창의 페이지가 호환 경로로 부팅했다(043): 이전 페이지(같은 incarnation, 새로고침 전)가 남긴 선언을 거둬 삽입 전달을
/// 되살린다. 부르는 창은 살아 있으므로 label의 선언을 incarnation과 상관없이 지운다.
pub fn withdraw_network_delivery(label: &str) {
    network_windows().remove(label);
}

/// 이 창에 삽입 전달을 건너뛸지(현재 incarnation이 네트워크 경로를 선언함).
pub fn is_network_delivery(label: &str) -> bool {
    let Some(current) = crate::infrastructure::window_principals::incarnation(label) else {
        return false;
    };
    network_windows()
        .get(label)
        .is_some_and(|declared| *declared == current)
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
        if is_network_delivery(&label) {
            return;
        }
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
                self.eval_in_bench(
                    &bench_id,
                    ORCHESTRATION_WORKSPACE_UPDATED_FALLBACK,
                    &payload,
                );
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
                    // 창 제목 적용(표현 상태)은 경로와 상관없이 앱이 한다. 화면 알림만 네트워크 창에서는 구독이 대신한다.
                    if is_network_delivery(&label) {
                        return;
                    }
                    let _ = window.eval(dispatch_script(
                        MCP_WINDOW_TITLE_FALLBACK,
                        &serde_json::json!({ "title": title }),
                    ));
                });
            }
        }
    }
}

impl RunLaunchDecorator for TauriDesktopBridge {
    /// 오늘 `start_agent_run`의 보강: run에 묶인 MCP 토큰을 만들어 env·MCP 서버·안내문을 넣는다. 작업대에 창이
    /// 없으면(닫힘) 띄우지 않는다(오늘 문구).
    fn decorate(
        &self,
        request: &mut AgentRunRequest,
        context: &LaunchContext,
    ) -> Result<(), String> {
        let mcp = self
            .mcp
            .get()
            .ok_or_else(|| "MCP server is not ready.".to_owned())?;
        desktop_benches::label_for(&context.bench_id)
            .ok_or_else(|| desktop_benches::MESSAGE_WINDOW_UNAVAILABLE.to_owned())?;
        inject_mcp_launch_env(request, mcp.launch_env(&context.run_id));
        Ok(())
    }

    fn revoke_run(&self, run_id: &str) {
        if let Some(mcp) = self.mcp.get() {
            mcp.revoke_run_capability(run_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::window_principals;

    fn label() -> String {
        format!("session-test-{}", uuid::Uuid::new_v4())
    }

    #[test]
    fn only_the_current_incarnation_can_declare_and_skip_delivery() {
        let label = label();
        assert!(
            !is_network_delivery(&label),
            "unregistered windows receive injections"
        );
        let old = window_principals::register(&label);
        assert!(
            !is_network_delivery(&label),
            "compat windows receive injections"
        );
        declare_network_delivery(&label, &old).unwrap();
        assert!(is_network_delivery(&label));

        // 같은 label로 다시 만든 창: 선언 전에는 삽입 전달을 받는다. 옛 incarnation으로는 선언하지 못한다.
        let new = window_principals::register(&label);
        assert!(
            !is_network_delivery(&label),
            "a reopened window starts on injections"
        );
        assert!(declare_network_delivery(&label, &old).is_err());
        // 옛 창의 늦은 정리는 새 창의 선언을 지우지 못한다.
        declare_network_delivery(&label, &new).unwrap();
        forget_network_delivery(&label, &old);
        assert!(is_network_delivery(&label));
        forget_network_delivery(&label, &new);
        assert!(!is_network_delivery(&label));
        window_principals::retire(&label, &new);
    }

    /// 선언한 창을 새로고침했는데 부팅이 호환 경로로 떨어지면(같은 incarnation) 선언을 거둬 삽입 전달을 되살린다.
    #[test]
    fn a_window_that_falls_back_to_compat_withdraws_its_declaration() {
        let label = label();
        let inc = window_principals::register(&label);
        declare_network_delivery(&label, &inc).unwrap();
        assert!(is_network_delivery(&label));
        withdraw_network_delivery(&label);
        assert!(
            !is_network_delivery(&label),
            "the compat page receives injections again"
        );
        window_principals::retire(&label, &inc);
    }
}
