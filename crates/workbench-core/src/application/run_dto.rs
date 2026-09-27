//! run DTO ↔ acp-agent-core 도메인 변환(040). protocol DTO는 원본 serde 속성의 미러라 JSON으로 옮긴다. wire가
//! 같다는 것은 아래 parity 테스트가 모든 variant로 고정한다.

use serde::{de::DeserializeOwned, Serialize};

/// 같은 wire를 가진 두 타입 사이의 변환. 실패는 계약 위반(미러가 어긋남)이다.
pub fn convert<From: Serialize, To: DeserializeOwned>(value: &From) -> To {
    serde_json::from_value(serde_json::to_value(value).expect("serializes"))
        .expect("protocol DTO and domain type share the same wire")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use acp_agent_core::domain::{
        agent_tool_candidate::{
            AgentToolCandidate, AgentToolCandidateQuery, AgentToolCandidateResponse,
            AgentToolCandidateScope, AgentToolCandidateSource, AgentToolCandidateStatus,
        },
        run::{
            AgentMcpHttpHeader, AgentMcpServerConfig, AgentRun, AgentRunRequest, ContextSizePreset,
            PermissionMode, RalphLoopRequest, ResumePolicy,
        },
    };
    use workbench_protocol::operations::run::{
        AgentRunDto, AgentRunRequestDto, AgentToolCandidateQueryDto, AgentToolCandidateResponseDto,
    };

    fn parity<D: serde::Serialize, P: serde::de::DeserializeOwned + serde::Serialize>(
        label: &str,
        domain: &D,
    ) {
        let json = serde_json::to_value(domain).unwrap();
        let dto: P = serde_json::from_value(json.clone()).unwrap_or_else(|error| {
            panic!("{label}: dto cannot read domain JSON: {error}\n{json}")
        });
        assert_eq!(serde_json::to_value(&dto).unwrap(), json, "{label}");
    }

    #[test]
    fn run_request_and_result_wire_parity() {
        for (policy, mode, size) in [
            (
                ResumePolicy::Fresh,
                PermissionMode::Default,
                ContextSizePreset::Default,
            ),
            (
                ResumePolicy::ResumeIfAvailable,
                PermissionMode::Auto,
                ContextSizePreset::Medium,
            ),
            (
                ResumePolicy::ResumeRequired,
                PermissionMode::ReadOnly,
                ContextSizePreset::Large,
            ),
            (
                ResumePolicy::Fresh,
                PermissionMode::Plan,
                ContextSizePreset::XLarge,
            ),
            (
                ResumePolicy::Fresh,
                PermissionMode::AcceptEdits,
                ContextSizePreset::Default,
            ),
            (
                ResumePolicy::Fresh,
                PermissionMode::DangerouslySkipAllPermissions,
                ContextSizePreset::Default,
            ),
        ] {
            let request = AgentRunRequest {
                goal: "g".into(),
                agent_id: "codex".into(),
                workspace_id: Some("w".into()),
                checkout_id: None,
                cwd: Some("/w".into()),
                agent_command: Some("codex acp".into()),
                agent_env: Some(BTreeMap::from([("K".into(), "V".into())])),
                mcp_servers: vec![AgentMcpServerConfig::Http {
                    name: "aw".into(),
                    url: "http://x".into(),
                    headers: vec![AgentMcpHttpHeader {
                        name: "Authorization".into(),
                        value: "Bearer t".into(),
                    }],
                }],
                stdio_buffer_limit_mb: Some(8),
                auto_allow: Some(false),
                permission_mode: Some(mode),
                model_id: Some("m".into()),
                effort_id: Some("high".into()),
                context_size: Some(size),
                run_id: Some("r1".into()),
                resume_session_id: None,
                resume_policy: Some(policy),
                ralph_loop: Some(RalphLoopRequest {
                    enabled: true,
                    max_iterations: 3,
                    prompt_template: "again".into(),
                    stop_on_error: true,
                    stop_on_permission: false,
                    delay_ms: 10,
                }),
            };
            parity::<_, AgentRunRequestDto>("request", &request);
        }
        parity::<_, AgentRunDto>(
            "run",
            &AgentRun::with_id("r1".into(), "g".into(), "codex".into()),
        );
    }

    #[test]
    fn tool_candidate_wire_parity() {
        parity::<_, AgentToolCandidateQueryDto>(
            "query",
            &AgentToolCandidateQuery {
                run_id: None,
                agent_id: "codex".into(),
                working_directory: "/w".into(),
                session_mode: "new".into(),
            },
        );
        for (status, source) in [
            (
                AgentToolCandidateStatus::Loading,
                AgentToolCandidateSource::SessionTool,
            ),
            (
                AgentToolCandidateStatus::Ready,
                AgentToolCandidateSource::AppCommand,
            ),
            (
                AgentToolCandidateStatus::Empty,
                AgentToolCandidateSource::Extension,
            ),
            (
                AgentToolCandidateStatus::Error,
                AgentToolCandidateSource::SessionTool,
            ),
        ] {
            parity::<_, AgentToolCandidateResponseDto>(
                "response",
                &AgentToolCandidateResponse {
                    status,
                    candidates: vec![AgentToolCandidate {
                        id: "c".into(),
                        name: "n".into(),
                        description: None,
                        insert_text: "$n".into(),
                        source,
                        scope: AgentToolCandidateScope {
                            run_id: None,
                            agent_id: Some("codex".into()),
                            working_directory: None,
                        },
                    }],
                },
            );
        }
    }
}
