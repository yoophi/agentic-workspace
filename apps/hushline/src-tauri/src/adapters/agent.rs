// hushline의 ACP agent run 어댑터.
//
// 공유 crate `acp-agent-core`의 use case를 얇게 감싸 Tauri command로 노출하고,
// run 이벤트를 프론트로 흘려보내는 sink를 제공한다. 저장/영속은 하지 않으며(세션 재개
// 미사용, NoopAcpSessionStore), run 이벤트 스트림은 `agent-run-event` 채널로 emit한다.
use std::{
    env,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use tauri::{AppHandle, Emitter, State, Window};

use acp_agent_core::{
    application::{
        cancel_agent_run::CancelAgentRunUseCase, send_prompt::SendPromptUseCase,
        set_permission_mode::SetPermissionModeUseCase, start_agent_run::StartAgentRunUseCase,
    },
    domain::{
        events::{RunEvent, RunEventEnvelope},
        run::{AgentRun, AgentRunRequest, PermissionMode},
    },
    infrastructure::{
        acp::runner::AcpAgentRunner, agent_catalog::ConfigurableAgentCatalog,
        agent_session_registry::AppState, noop_acp_session_store::NoopAcpSessionStore,
    },
    ports::{event_sink::RunEventSink, permission::PermissionDecision},
};

/// run 이벤트가 전달되는 Tauri 이벤트 채널. `@yoophi/agent-client`의 `listenRunEvents`가 구독한다.
pub const AGENT_RUN_EVENT: &str = "agent-run-event";

/// run 이벤트를 프론트(단일 창)로 그대로 emit하는 sink.
#[derive(Clone)]
pub struct HushlineAgentSink {
    app: AppHandle,
}

impl HushlineAgentSink {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl RunEventSink for HushlineAgentSink {
    fn emit(&self, run_id: &str, event: RunEvent) {
        let envelope = RunEventEnvelope {
            run_id: run_id.to_string(),
            event,
        };
        let _ = self.app.emit(AGENT_RUN_EVENT, envelope);
    }
}

/// agent가 접근·기록할 수 있는 안전 경계(사용자 홈). 자막·정리 문서는 이 하위에서만 다룬다.
fn managed_root() -> Result<PathBuf, String> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| "홈 디렉터리를 확인할 수 없습니다".to_string())
}

/// cwd가 관리 경계(`base`) 하위의 절대 경로인지 검증한다(`..` 금지). 순수 함수라 테스트 가능.
fn ensure_cwd_within(base: &Path, cwd: &str) -> Result<PathBuf, String> {
    let cwd = cwd.trim();
    if cwd.is_empty() {
        return Err("cwd가 비어 있습니다".to_string());
    }
    let path = PathBuf::from(cwd);
    if !path.is_absolute() {
        return Err("cwd는 절대 경로여야 합니다".to_string());
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("cwd에 상위 경로(..)를 포함할 수 없습니다".to_string());
    }
    if !path.starts_with(base) {
        return Err(format!(
            "cwd는 관리 디렉터리({}) 하위여야 합니다",
            base.display()
        ));
    }
    Ok(path)
}

/// 새 agent run을 시작한다. goal/cwd를 검증하고 공유 use case에 위임한다.
#[tauri::command]
pub async fn start_agent_run(
    app: AppHandle,
    window: Window,
    state: State<'_, AppState>,
    request: AgentRunRequest,
) -> Result<AgentRun, String> {
    if request.goal.trim().is_empty() {
        return Err("goal은 비어 있을 수 없습니다".to_string());
    }
    let root = managed_root()?;
    let cwd = request.cwd.clone().unwrap_or_default();
    ensure_cwd_within(&root, &cwd)?;

    let owner = window.label().to_string();
    let sink = HushlineAgentSink::new(app);
    let registry = state.inner().clone();
    let permissions = state.permissions();
    let catalog = ConfigurableAgentCatalog::from_env();
    let runner = AcpAgentRunner::new(catalog, permissions, Arc::new(NoopAcpSessionStore));

    StartAgentRunUseCase::new(registry)
        .execute(runner, sink, request, Some(owner))
        .await
        .map_err(String::from)
}

/// 진행 중 run에 후속 프롬프트를 보낸다(지식 대화용).
#[tauri::command]
pub async fn send_prompt_to_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
    prompt: String,
) -> Result<(), String> {
    let sink = HushlineAgentSink::new(app);
    SendPromptUseCase::new(state.inner().clone())
        .execute(sink, run_id, prompt)
        .await
        .map_err(String::from)
}

/// 진행 중 run의 권한 모드를 런타임에 변경한다.
#[tauri::command]
pub async fn set_run_permission_mode(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
    permission_mode: PermissionMode,
) -> Result<(), String> {
    let sink = HushlineAgentSink::new(app);
    SetPermissionModeUseCase::new(state.inner().clone())
        .execute(sink, run_id, permission_mode)
        .await
        .map_err(String::from)
}

/// run을 취소한다.
#[tauri::command]
pub async fn cancel_agent_run(
    app: AppHandle,
    state: State<'_, AppState>,
    run_id: String,
) -> Result<(), String> {
    let sink = HushlineAgentSink::new(app);
    CancelAgentRunUseCase::new(state.inner().clone())
        .execute(sink, run_id)
        .await;
    Ok(())
}

/// 권한 요청에 대한 사용자 선택을 전달한다. 소유 창만 응답할 수 있다.
#[tauri::command]
pub async fn respond_agent_permission(
    window: Window,
    state: State<'_, AppState>,
    run_id: String,
    permission_id: String,
    option_id: String,
) -> Result<(), String> {
    let owner = state
        .owner_of(&run_id)
        .await
        .ok_or_else(|| format!("알 수 없거나 종료된 run: {run_id}"))?;
    if owner != window.label() {
        return Err("소유하지 않은 창에서 권한 응답을 보냈습니다".to_string());
    }
    state
        .permissions()
        .respond_for_run(&run_id, &permission_id, PermissionDecision { option_id })
        .await
        .map_err(|err| err.to_string())
}

/// 자막을 사용자가 지정한 방식으로 정리한 새 문서. 원본 자막과 연결된다.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizedDocument {
    pub source_url: Option<String>,
    pub source_transcript_path: Option<String>,
    pub style: String,
    pub title: String,
    pub content: String,
    /// 프론트에서 생성한 ISO-8601 생성 시각(백엔드에 시계 의존성을 두지 않는다).
    pub created_at: String,
}

/// 정리 문서를 관리 디렉터리 하위에 JSON으로 저장한다.
#[tauri::command]
pub fn save_organized_document(
    dir: String,
    base_name: String,
    document: OrganizedDocument,
) -> Result<String, String> {
    let root = managed_root()?;
    let target_dir = ensure_cwd_within(&root, &dir)?;
    if document.content.trim().is_empty() {
        return Err("정리 내용이 비어 있어 저장할 수 없습니다".to_string());
    }
    let safe_base: String = base_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let safe_base = if safe_base.trim_matches('_').is_empty() {
        "organized".to_string()
    } else {
        safe_base
    };
    let path = target_dir.join(format!("{safe_base}.organized.json"));
    let json = serde_json::to_string_pretty(&document)
        .map_err(|e| format!("정리 문서 JSON 생성 실패: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("정리 문서 저장 실패: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 계약 CT-2: cwd가 관리 경계 밖이면 거부한다.
    #[test]
    fn rejects_cwd_outside_managed_root() {
        let base = Path::new("/home/user");
        assert!(ensure_cwd_within(base, "/etc/passwd").is_err());
        assert!(ensure_cwd_within(base, "").is_err());
        assert!(ensure_cwd_within(base, "relative/path").is_err());
        assert!(ensure_cwd_within(base, "/home/user/../root").is_err());
    }

    #[test]
    fn accepts_cwd_within_managed_root() {
        let base = Path::new("/home/user");
        assert_eq!(
            ensure_cwd_within(base, "/home/user/Downloads/Hushline/vid").unwrap(),
            PathBuf::from("/home/user/Downloads/Hushline/vid")
        );
    }
}
