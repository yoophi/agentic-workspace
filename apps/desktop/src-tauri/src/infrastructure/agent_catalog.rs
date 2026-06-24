use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use crate::{
    domain::agent::{AgentDescriptor, AgentOptionDescriptor},
    ports::agent_catalog::AgentCatalog,
};

const AGENT_CATALOG_PATH_ENV: &str = "ACP_AGENT_CATALOG_PATH";

#[derive(Clone, Default)]
pub struct ConfigurableAgentCatalog {
    file_path: Option<PathBuf>,
    fallback: StaticAgentCatalog,
}

impl ConfigurableAgentCatalog {
    pub fn from_env() -> Self {
        Self {
            file_path: env::var_os(AGENT_CATALOG_PATH_ENV).map(PathBuf::from),
            fallback: StaticAgentCatalog,
        }
    }
}

impl AgentCatalog for ConfigurableAgentCatalog {
    fn list_agents(&self) -> Vec<AgentDescriptor> {
        self.file_path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .and_then(|content| serde_json::from_str::<Vec<AgentDescriptor>>(&content).ok())
            .filter(|agents| !agents.is_empty())
            .unwrap_or_else(|| self.fallback.list_agents())
    }
}

#[derive(Clone, Default)]
pub struct StaticAgentCatalog;

impl AgentCatalog for StaticAgentCatalog {
    fn list_agents(&self) -> Vec<AgentDescriptor> {
        vec![
            AgentDescriptor {
                id: "codex".into(),
                label: "Codex".into(),
                command: "npx -y @agentclientprotocol/codex-acp".into(),
                models: options(&[
                    ("gpt-5.5", "GPT-5.5"),
                    ("gpt-5.4", "GPT-5.4"),
                    ("gpt-5.4-mini", "GPT-5.4 mini"),
                    ("gpt-5.4-nano", "GPT-5.4 nano"),
                    ("gpt-5.3-codex", "GPT-5.3 Codex"),
                    ("gpt-5.3-codex-spark", "GPT-5.3 Codex Spark"),
                    ("gpt-5.2-codex", "GPT-5.2 Codex"),
                    ("gpt-5.1-codex", "GPT-5.1 Codex"),
                    ("gpt-5-codex", "GPT-5 Codex"),
                ]),
                context_sizes: options(&[
                    ("medium", "Medium"),
                    ("large", "Large"),
                    ("xLarge", "XL"),
                ]),
            },
            AgentDescriptor {
                id: "claude-code".into(),
                label: "Claude Code".into(),
                command: "npx -y @agentclientprotocol/claude-agent-acp".into(),
                models: options(&[
                    ("opus", "Opus alias"),
                    ("sonnet", "Sonnet alias"),
                    ("fable", "Fable alias"),
                    ("claude-opus-4-8", "Claude Opus 4.8"),
                    ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
                    ("claude-haiku-4-5", "Claude Haiku 4.5"),
                ]),
                context_sizes: Vec::new(),
            },
            AgentDescriptor {
                id: "pi-coding-agent".into(),
                label: "Pi Coding Agent".into(),
                command: "npx -y pi-acp".into(),
                models: Vec::new(),
                context_sizes: Vec::new(),
            },
            AgentDescriptor {
                id: "opencode".into(),
                label: "OpenCode".into(),
                command: "npx -y opencode-ai acp".into(),
                models: opencode_models().unwrap_or_else(opencode_fallback_models),
                context_sizes: Vec::new(),
            },
        ]
    }
}

fn options(values: &[(&str, &str)]) -> Vec<AgentOptionDescriptor> {
    values
        .iter()
        .map(|(id, label)| AgentOptionDescriptor {
            id: (*id).into(),
            label: (*label).into(),
        })
        .collect()
}

fn opencode_models() -> Option<Vec<AgentOptionDescriptor>> {
    let mut child = Command::new("opencode")
        .arg("models")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if child.try_wait().ok()?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(25));
    }

    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }

    let models = String::from_utf8_lossy(&output.stdout);
    let parsed = models
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|id| AgentOptionDescriptor {
            id: id.to_string(),
            label: model_label(id),
        })
        .collect::<Vec<_>>();

    (!parsed.is_empty()).then_some(parsed)
}

fn opencode_fallback_models() -> Vec<AgentOptionDescriptor> {
    options(&[
        ("opencode/claude-opus-4-8", "OpenCode Claude Opus 4.8"),
        ("opencode/claude-sonnet-4-6", "OpenCode Claude Sonnet 4.6"),
        ("opencode/claude-haiku-4-5", "OpenCode Claude Haiku 4.5"),
        ("opencode/gpt-5.5", "OpenCode GPT-5.5"),
        ("opencode/gpt-5.4", "OpenCode GPT-5.4"),
        ("opencode/gpt-5.4-mini", "OpenCode GPT-5.4 mini"),
        ("opencode/gpt-5.3-codex", "OpenCode GPT-5.3 Codex"),
        ("opencode/gemini-3.1-pro", "OpenCode Gemini 3.1 Pro"),
    ])
}

fn model_label(id: &str) -> String {
    id.split('/')
        .map(format_model_part)
        .collect::<Vec<_>>()
        .join(" / ")
}

fn format_model_part(part: &str) -> String {
    let segments = part
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut formatted = Vec::new();
    let mut index = 0;

    while index < segments.len() {
        let segment = segments[index];
        if index + 1 < segments.len()
            && segment.chars().all(|character| character.is_ascii_digit())
            && segments[index + 1]
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            formatted.push(format!("{segment}.{}", segments[index + 1]));
            index += 2;
            continue;
        }

        formatted.push(capitalize_model_segment(segment));
        index += 1;
    }

    formatted.join(" ")
}

fn capitalize_model_segment(segment: &str) -> String {
    match segment {
        "ai" => "AI".into(),
        "api" => "API".into(),
        "claude" => "Claude".into(),
        "codex" => "Codex".into(),
        "flash" => "Flash".into(),
        "gemini" => "Gemini".into(),
        "gpt" => "GPT".into(),
        "haiku" => "Haiku".into(),
        "max" => "Max".into(),
        "mini" => "mini".into(),
        "nano" => "nano".into(),
        "openai" => "OpenAI".into(),
        "opencode" => "OpenCode".into(),
        "opus" => "Opus".into(),
        "pro" => "Pro".into(),
        "sonnet" => "Sonnet".into(),
        "spark" => "Spark".into(),
        "thinking" => "Thinking".into(),
        value => value.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_provider_model_ids_for_display() {
        assert_eq!(
            model_label("openai/gpt-5.3-codex-spark"),
            "OpenAI / GPT 5.3 Codex Spark"
        );
        assert_eq!(
            model_label("opencode/claude-sonnet-4-6"),
            "OpenCode / Claude Sonnet 4.6",
        );
    }

    #[test]
    fn opencode_fallback_contains_current_coding_models() {
        let models = opencode_fallback_models();

        assert!(
            models
                .iter()
                .any(|model| model.id == "opencode/claude-opus-4-8")
        );
        assert!(models.iter().any(|model| model.id == "opencode/gpt-5.5"));
    }
}
