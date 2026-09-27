//! run operation 로직(040, research R5·R6). 작업대 확인 → 소유 검사 → 엔진 호출. 오류 문구는 오늘 command와 같고,
//! 새로 검사하는 동작(프롬프트·조향·취소 후 전송·권한 모드·취소)만 새 문구를 쓴다.

use std::sync::Arc;

use acp_agent_core::domain::{
    agent_tool_candidate::{
        response_for_candidates, AgentToolCandidate, AgentToolCandidateQuery,
        AgentToolCandidateResponse, AgentToolCandidateScope, AgentToolCandidateSource,
        AgentToolCandidateStatus,
    },
    run::{AgentRunRequest, PermissionMode, RalphLoopRequest},
};
use workbench_protocol::{AuthenticatedPrincipal, FaultCode, RequestId, WorkbenchFault};

use crate::{
    application::{agent_run_settings_service, bench_service::BenchServices},
    domain::agent_run_settings::{AgentCommandSource, APP_COMMAND_OVERRIDE_SETTINGS_KEY},
    infrastructure::storage_coordinator::StorageCoordinator,
    ports::run_engine::{RunEngineError, RunErrorKind},
};

pub const MESSAGE_OWNED_BY_OTHER_BENCH: &str = "run is owned by another bench.";
pub const MESSAGE_CANDIDATES_NON_OWNER: &str =
    "tool command candidates were requested from a non-owner window";
pub const MESSAGE_PERMISSION_NON_OWNER: &str =
    "permission response was sent from a non-owner window";
/// MCP 제목 도구 이름(오늘 AW `title_tool::SET_WINDOW_TITLE_TOOL`과 같음).
pub const SET_WINDOW_TITLE_TOOL: &str = "set_window_title";

pub fn engine_fault(request_id: &RequestId, error: RunEngineError) -> WorkbenchFault {
    let code = match error.kind {
        RunErrorKind::InvalidArgument => FaultCode::InvalidArgument,
        RunErrorKind::NotFound => FaultCode::NotFound,
        RunErrorKind::Conflict => FaultCode::Conflict,
        RunErrorKind::RateLimited => FaultCode::RateLimited,
        RunErrorKind::PreconditionFailed => FaultCode::PreconditionFailed,
        RunErrorKind::Unavailable => FaultCode::Unavailable,
        RunErrorKind::Internal => FaultCode::Internal,
    };
    WorkbenchFault::new(code, request_id.clone(), error.message)
}

/// 오늘 `start_agent_run`의 정규화: run id 비우기(서버가 예약 단계에서 정함), 작업 영역 식별자 제거, Ralph loop 정리.
pub fn normalize_run_request(mut request: AgentRunRequest) -> AgentRunRequest {
    if request.run_id.as_deref().is_some_and(str::is_empty) {
        request.run_id = None;
    }
    request.workspace_id = None;
    request.checkout_id = None;
    request.ralph_loop = request.ralph_loop.map(RalphLoopRequest::sanitized);
    request
}

/// 명령이 비어 있으면 앱 전역 명령 재정의를 적용한다(오늘 `start_agent_run`과 같음). aggregate lock 밖에서 부른다.
pub fn resolve_agent_command(
    coordinator: &StorageCoordinator,
    request: &mut AgentRunRequest,
) -> Result<(), String> {
    if request
        .agent_command
        .as_deref()
        .is_some_and(|command| !command.trim().is_empty())
    {
        return Ok(());
    }
    let settings = coordinator
        .with_agent_run_settings(|repository| {
            agent_run_settings_service::get_settings(
                repository,
                APP_COMMAND_OVERRIDE_SETTINGS_KEY.into(),
            )
        })
        .map_err(|error| error.to_string())?;
    if let Some(settings) = settings {
        let catalog =
            acp_agent_core::infrastructure::agent_catalog::ConfigurableAgentCatalog::from_env();
        let resolution = agent_run_settings_service::resolve_agent_command(
            &request.agent_id,
            &settings.command_overrides,
            acp_agent_core::ports::agent_catalog::AgentCatalog::command_for_agent(
                &catalog,
                &request.agent_id,
            ),
        )
        .map_err(|error| error.to_string())?;
        if resolution.source != AgentCommandSource::DefaultCommand {
            request.agent_command = Some(resolution.command);
        }
    }
    Ok(())
}

/// 살아 있는 run이면 소유 작업대가 `bench_id`여야 한다. 끝난 run(소유 기록 없음)은 엔진이 "비활성"으로 답한다.
async fn ensure_owned(
    services: &BenchServices,
    request_id: &RequestId,
    bench_id: &str,
    run_id: &str,
) -> Result<(), WorkbenchFault> {
    match services.engine.owner_of(run_id).await {
        Some(owner) if owner != bench_id => Err(WorkbenchFault::new(
            FaultCode::Forbidden,
            request_id.clone(),
            MESSAGE_OWNED_BY_OTHER_BENCH,
        )),
        _ => Ok(()),
    }
}

fn candidates_for(query: &AgentToolCandidateQuery) -> Vec<AgentToolCandidate> {
    vec![AgentToolCandidate {
        id: format!("session:{SET_WINDOW_TITLE_TOOL}"),
        name: SET_WINDOW_TITLE_TOOL.to_string(),
        description: Some(
            "Change the current Worktree Session window title for the active agent run."
                .to_string(),
        ),
        insert_text: format!("${SET_WINDOW_TITLE_TOOL}"),
        source: AgentToolCandidateSource::SessionTool,
        scope: AgentToolCandidateScope {
            run_id: query.run_id.clone(),
            agent_id: Some(query.agent_id.clone()),
            working_directory: Some(query.working_directory.clone()),
        },
    }]
}

pub async fn list_tool_candidates(
    services: &BenchServices,
    request_id: &RequestId,
    principal: &AuthenticatedPrincipal,
    bench_id: &str,
    query: AgentToolCandidateQuery,
) -> Result<AgentToolCandidateResponse, WorkbenchFault> {
    services.resolve(request_id, principal, bench_id)?;
    let run_id = query
        .run_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(str::to_owned);
    if let Some(run_id) = run_id {
        match services.engine.active_owner_of(&run_id).await {
            None => {
                return Ok(AgentToolCandidateResponse {
                    status: AgentToolCandidateStatus::Empty,
                    candidates: Vec::new(),
                })
            }
            Some(owner) if owner != bench_id => {
                return Err(WorkbenchFault::new(
                    FaultCode::Forbidden,
                    request_id.clone(),
                    MESSAGE_CANDIDATES_NON_OWNER,
                ))
            }
            Some(_) => {}
        }
    }
    Ok(response_for_candidates(candidates_for(&query)))
}

#[derive(Clone, Copy)]
pub enum PromptKind {
    Send,
    Steer,
    CancelAndSend,
}

pub async fn prompt(
    services: Arc<BenchServices>,
    request_id: &RequestId,
    principal: &AuthenticatedPrincipal,
    bench_id: &str,
    run_id: &str,
    prompt: String,
    kind: PromptKind,
) -> Result<(), WorkbenchFault> {
    services.resolve(request_id, principal, bench_id)?;
    ensure_owned(&services, request_id, bench_id, run_id).await?;
    let sink = services.run_sink(bench_id);
    let result = match kind {
        PromptKind::Send => services.engine.send_prompt(run_id, prompt, sink).await,
        PromptKind::Steer => services.engine.steer_prompt(run_id, prompt, sink).await,
        PromptKind::CancelAndSend => {
            services
                .engine
                .cancel_current_prompt_and_send(run_id, prompt, sink)
                .await
        }
    };
    result.map_err(|error| engine_fault(request_id, error))
}

pub async fn set_permission_mode(
    services: Arc<BenchServices>,
    request_id: &RequestId,
    principal: &AuthenticatedPrincipal,
    bench_id: &str,
    run_id: &str,
    mode: PermissionMode,
) -> Result<(), WorkbenchFault> {
    services.resolve(request_id, principal, bench_id)?;
    ensure_owned(&services, request_id, bench_id, run_id).await?;
    services
        .engine
        .set_permission_mode(run_id, mode, services.run_sink(bench_id))
        .await
        .map_err(|error| engine_fault(request_id, error))
}

pub async fn cancel(
    services: Arc<BenchServices>,
    request_id: &RequestId,
    principal: &AuthenticatedPrincipal,
    bench_id: &str,
    run_id: &str,
) -> Result<(), WorkbenchFault> {
    services.resolve(request_id, principal, bench_id)?;
    ensure_owned(&services, request_id, bench_id, run_id).await?;
    services
        .engine
        .cancel(run_id, services.run_sink(bench_id))
        .await;
    Ok(())
}

pub async fn respond_permission(
    services: Arc<BenchServices>,
    request_id: &RequestId,
    principal: &AuthenticatedPrincipal,
    bench_id: &str,
    run_id: &str,
    permission_id: &str,
    option_id: &str,
) -> Result<(), WorkbenchFault> {
    services.resolve(request_id, principal, bench_id)?;
    let owner = services.engine.owner_of(run_id).await.ok_or_else(|| {
        WorkbenchFault::new(
            FaultCode::NotFound,
            request_id.clone(),
            format!("unknown or finished run: {run_id}"),
        )
    })?;
    if owner != bench_id {
        return Err(WorkbenchFault::new(
            FaultCode::Forbidden,
            request_id.clone(),
            MESSAGE_PERMISSION_NON_OWNER,
        ));
    }
    services
        .engine
        .respond_permission(run_id, permission_id, option_id)
        .await
        .map_err(|error| engine_fault(request_id, error))
}

#[cfg(test)]
mod tests {
    use acp_agent_core::domain::run::{
        AgentRunRequest, RalphLoopRequest, ResumePolicy, MAX_RALPH_DELAY_MS, MAX_RALPH_ITERATIONS,
    };

    use super::normalize_run_request;

    fn sample_request() -> AgentRunRequest {
        serde_json::from_value(serde_json::json!({
            "goal": "do it",
            "agentId": "codex",
            "workspaceId": "ws",
            "checkoutId": "co",
            "resumeSessionId": "sess-1",
            "resumePolicy": "resumeIfAvailable"
        }))
        .unwrap()
    }

    // AW에서 옮긴 회귀 방지: 과거 start_agent_run이 resume 필드를 None으로 덮어써 재사용이 동작하지 않던 버그.
    #[test]
    fn normalize_preserves_resume_fields_and_clears_unsupported() {
        let out = normalize_run_request(sample_request());
        assert_eq!(out.resume_session_id.as_deref(), Some("sess-1"));
        assert_eq!(out.resume_policy, Some(ResumePolicy::ResumeIfAvailable));
        assert!(out.workspace_id.is_none());
        assert!(out.checkout_id.is_none());
        // run id는 변경 기록의 예약 단계에서 서버가 정한다(재시도 지문이 같게).
        assert!(out.run_id.is_none());
    }

    #[test]
    fn normalize_keeps_existing_run_id_and_treats_empty_as_missing() {
        let mut request = sample_request();
        request.run_id = Some("fixed-id".into());
        assert_eq!(
            normalize_run_request(request).run_id.as_deref(),
            Some("fixed-id")
        );
        let mut request = sample_request();
        request.run_id = Some(String::new());
        assert!(normalize_run_request(request).run_id.is_none());
    }

    #[test]
    fn normalize_sanitizes_ralph_loop_into_safe_range() {
        let mut request = sample_request();
        request.ralph_loop = Some(RalphLoopRequest {
            enabled: true,
            max_iterations: 10_000,
            prompt_template: "  continue  ".into(),
            stop_on_error: true,
            stop_on_permission: false,
            delay_ms: u64::MAX,
        });
        let settings = normalize_run_request(request).ralph_loop.unwrap();
        assert_eq!(settings.max_iterations, MAX_RALPH_ITERATIONS);
        assert_eq!(settings.delay_ms, MAX_RALPH_DELAY_MS);
        assert_eq!(settings.prompt_template, "continue");
    }
}
