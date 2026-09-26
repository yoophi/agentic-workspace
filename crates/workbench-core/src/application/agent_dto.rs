//! agent catalog·provider 세션 ↔ protocol DTO 변환(038 US3, research R1).

use acp_agent_core::domain::agent::{AgentDescriptor, AgentOptionDescriptor};
use workbench_protocol::operations::agent::{
    AgentDescriptorDto, AgentOptionDescriptorDto, ProviderSessionDto,
};

use crate::domain::provider_session::ProviderSession;

fn option_dto(option: &AgentOptionDescriptor) -> AgentOptionDescriptorDto {
    AgentOptionDescriptorDto {
        id: option.id.clone(),
        label: option.label.clone(),
    }
}

pub fn agent_descriptor_dto(agent: &AgentDescriptor) -> AgentDescriptorDto {
    AgentDescriptorDto {
        id: agent.id.clone(),
        label: agent.label.clone(),
        command: agent.command.clone(),
        runtime_version: agent.runtime_version.clone(),
        models: agent.models.iter().map(option_dto).collect(),
        efforts: agent.efforts.iter().map(option_dto).collect(),
        context_sizes: agent.context_sizes.iter().map(option_dto).collect(),
    }
}

pub fn provider_session_dto(session: &ProviderSession) -> ProviderSessionDto {
    ProviderSessionDto {
        agent_id: session.agent_id.clone(),
        id: session.id.clone(),
        cwd: session.cwd.clone(),
        title: session.title.clone(),
        file: session.file.clone(),
        message_count: session.message_count as u64,
        created_at: session.created_at.clone(),
        updated_at: session.updated_at.clone(),
        model: session.model.clone(),
        branch: session.branch.clone(),
        source: session.source.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::dto::assert_wire_parity;

    fn option(id: &str) -> AgentOptionDescriptor {
        AgentOptionDescriptor {
            id: id.into(),
            label: id.to_uppercase(),
        }
    }

    #[test]
    fn agent_descriptor_wire_parity() {
        let empty = AgentDescriptor {
            id: "codex".into(),
            label: "Codex".into(),
            command: "codex-acp".into(),
            runtime_version: None,
            models: Vec::new(),
            efforts: Vec::new(),
            context_sizes: Vec::new(),
        };
        assert_wire_parity("agent (empty)", &empty, &agent_descriptor_dto(&empty));
        let full = AgentDescriptor {
            runtime_version: Some("1.2.3".into()),
            models: vec![option("gpt"), option("mini")],
            efforts: vec![option("high")],
            context_sizes: vec![option("1m")],
            ..empty
        };
        assert_wire_parity("agent (full)", &full, &agent_descriptor_dto(&full));
    }

    #[test]
    fn provider_session_wire_parity() {
        let bare = ProviderSession {
            agent_id: "codex".into(),
            id: "s1".into(),
            cwd: None,
            title: None,
            file: "/tmp/s1.jsonl".into(),
            message_count: 0,
            created_at: None,
            updated_at: None,
            model: None,
            branch: None,
            source: None,
        };
        assert_wire_parity("session (bare)", &bare, &provider_session_dto(&bare));
        let full = ProviderSession {
            cwd: Some("/repo".into()),
            title: Some("t".into()),
            message_count: 12,
            created_at: Some("2026-01-01T00:00:00Z".into()),
            updated_at: Some("2026-01-02T00:00:00Z".into()),
            model: Some("m".into()),
            branch: Some("main".into()),
            source: Some("cli".into()),
            ..bare
        };
        assert_wire_parity("session (full)", &full, &provider_session_dto(&full));
    }
}
