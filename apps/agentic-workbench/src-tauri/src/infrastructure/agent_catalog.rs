use std::{env, fs, path::PathBuf, process::Command};

use crate::{
    domain::agent::{AgentDescriptor, AgentOptionDescriptor},
    ports::agent_catalog::AgentCatalog,
};

const AGENT_CATALOG_PATH_ENV: &str = "ACP_AGENT_CATALOG_PATH";
const OPENCODE_MODELS_CACHE_PATH_ENV: &str = "ACP_OPENCODE_MODELS_CACHE_PATH";
const OPENCODE_MODELS_REFRESH_ENV: &str = "ACP_OPENCODE_MODELS_REFRESH";
const MODELS_DEV_API_URL: &str = "https://models.dev/api.json";
const MODELS_DEV_FETCH_TIMEOUT_SECONDS: &str = "2";
pub const CODEX_AGENT_ACP_VERSION: &str = "1.1.5";
pub const CLAUDE_AGENT_ACP_VERSION: &str = "0.60.0";

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
                command: format!("npx -y @agentclientprotocol/codex-acp@{CODEX_AGENT_ACP_VERSION}"),
                runtime_version: Some(CODEX_AGENT_ACP_VERSION.into()),
                models: options(&[
                    ("gpt-5.6", "GPT-5.6"),
                    ("gpt-5.6-sol", "GPT-5.6 Sol"),
                    ("gpt-5.6-terra", "GPT-5.6 Terra"),
                    ("gpt-5.6-luna", "GPT-5.6 Luna"),
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
                command: format!(
                    "npx -y @agentclientprotocol/claude-agent-acp@{CLAUDE_AGENT_ACP_VERSION}"
                ),
                runtime_version: Some(CLAUDE_AGENT_ACP_VERSION.into()),
                models: options(&[
                    ("best", "Best available"),
                    ("fable", "Fable alias"),
                    ("opus", "Opus alias"),
                    ("sonnet", "Sonnet alias"),
                    ("haiku", "Haiku alias"),
                    ("opusplan", "Opus plan / Sonnet execution"),
                    ("opus[1m]", "Opus 1M context"),
                    ("sonnet[1m]", "Sonnet 1M context"),
                    ("claude-fable-5", "Claude Fable 5"),
                    ("claude-opus-4-8", "Claude Opus 4.8"),
                    ("claude-sonnet-5", "Claude Sonnet 5"),
                    ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
                    ("claude-haiku-4-5", "Claude Haiku 4.5"),
                ]),
                context_sizes: Vec::new(),
            },
            AgentDescriptor {
                id: "pi-coding-agent".into(),
                label: "Pi Coding Agent".into(),
                command: "npx -y pi-acp".into(),
                runtime_version: None,
                models: Vec::new(),
                context_sizes: Vec::new(),
            },
            AgentDescriptor {
                id: "opencode".into(),
                label: "OpenCode".into(),
                command: "npx -y opencode-ai acp".into(),
                runtime_version: None,
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
    let cache_path = opencode_models_cache_path();
    let refresh = env::var_os(OPENCODE_MODELS_REFRESH_ENV).is_some();
    if !refresh
        && let Some(cache_path) = &cache_path
        && let Some(models) = read_cached_opencode_models(cache_path)
    {
        return Some(models);
    }

    let models = fetch_opencode_models_from_models_dev().or_else(|| {
        cache_path
            .as_ref()
            .and_then(|path| read_cached_opencode_models(path))
    })?;

    if let Some(cache_path) = &cache_path {
        write_cached_opencode_models(cache_path, &models);
    }

    Some(models)
}

fn fetch_opencode_models_from_models_dev() -> Option<Vec<AgentOptionDescriptor>> {
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "--max-time",
            MODELS_DEV_FETCH_TIMEOUT_SECONDS,
            MODELS_DEV_API_URL,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let content = String::from_utf8(output.stdout).ok()?;
    parse_models_dev_opencode_models(&content)
}

fn parse_models_dev_opencode_models(content: &str) -> Option<Vec<AgentOptionDescriptor>> {
    let root: serde_json::Value = serde_json::from_str(content).ok()?;
    let models = root
        .get("opencode")?
        .get("models")?
        .as_object()?
        .keys()
        .map(|id| {
            let opencode_id = format!("opencode/{id}");
            AgentOptionDescriptor {
                label: model_label(&opencode_id),
                id: opencode_id,
            }
        })
        .collect::<Vec<_>>();

    (!models.is_empty()).then_some(models)
}

fn read_cached_opencode_models(cache_path: &PathBuf) -> Option<Vec<AgentOptionDescriptor>> {
    let content = fs::read_to_string(cache_path).ok()?;
    serde_json::from_str::<Vec<AgentOptionDescriptor>>(&content)
        .ok()
        .filter(|models| !models.is_empty())
}

fn write_cached_opencode_models(cache_path: &PathBuf, models: &[AgentOptionDescriptor]) {
    let Some(parent) = cache_path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    if let Ok(content) = serde_json::to_string_pretty(models) {
        let _ = fs::write(cache_path, content);
    }
}

fn opencode_models_cache_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os(OPENCODE_MODELS_CACHE_PATH_ENV) {
        return Some(PathBuf::from(path));
    }

    let base = if cfg!(target_os = "macos") {
        env::var_os("HOME")
            .map(PathBuf::from)?
            .join("Library")
            .join("Caches")
    } else if let Some(path) = env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(path)
    } else {
        env::var_os("HOME").map(PathBuf::from)?.join(".cache")
    };

    Some(base.join("agentic-workbench").join("opencode-models.json"))
}

fn opencode_fallback_models() -> Vec<AgentOptionDescriptor> {
    options(&[
        ("opencode/claude-opus-4-8", "OpenCode Claude Opus 4.8"),
        ("opencode/claude-sonnet-4-6", "OpenCode Claude Sonnet 4.6"),
        ("opencode/claude-haiku-4-5", "OpenCode Claude Haiku 4.5"),
        ("opencode/gpt-5.6", "OpenCode GPT-5.6"),
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
        assert!(models.iter().any(|model| model.id == "opencode/gpt-5.6"));
    }

    #[test]
    fn codex_catalog_contains_gpt_5_6_family() {
        let agents = StaticAgentCatalog.list_agents();
        let codex = agents
            .iter()
            .find(|agent| agent.id == "codex")
            .expect("Codex agent");

        assert_eq!(
            codex.command,
            format!("npx -y @agentclientprotocol/codex-acp@{CODEX_AGENT_ACP_VERSION}")
        );
        assert_eq!(
            codex.runtime_version.as_deref(),
            Some(CODEX_AGENT_ACP_VERSION)
        );

        for model_id in ["gpt-5.6", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"] {
            assert!(
                codex.models.iter().any(|model| model.id == model_id),
                "missing Codex model {model_id}"
            );
        }
    }

    #[test]
    fn claude_catalog_pins_acp_and_lists_current_models() {
        let agents = StaticAgentCatalog.list_agents();
        let claude = agents
            .iter()
            .find(|agent| agent.id == "claude-code")
            .expect("Claude Code agent");

        assert_eq!(
            claude.command,
            format!("npx -y @agentclientprotocol/claude-agent-acp@{CLAUDE_AGENT_ACP_VERSION}")
        );
        assert_eq!(
            claude.runtime_version.as_deref(),
            Some(CLAUDE_AGENT_ACP_VERSION)
        );
        for model_id in [
            "best",
            "fable",
            "opus",
            "sonnet",
            "haiku",
            "opusplan",
            "opus[1m]",
            "sonnet[1m]",
            "claude-fable-5",
            "claude-opus-4-8",
            "claude-sonnet-5",
            "claude-sonnet-4-6",
            "claude-haiku-4-5",
        ] {
            assert!(
                claude.models.iter().any(|model| model.id == model_id),
                "missing Claude model {model_id}"
            );
        }
    }

    #[test]
    fn parses_opencode_models_from_models_dev_catalog() {
        let catalog = r#"{
            "opencode": {
                "models": {
                    "gpt-5.3-codex-spark": { "name": "GPT-5.3 Codex Spark" },
                    "claude-sonnet-4-6": { "name": "Claude Sonnet 4.6" }
                }
            }
        }"#;

        let models = parse_models_dev_opencode_models(catalog).expect("models parse");

        assert!(
            models
                .iter()
                .any(|model| model.id == "opencode/gpt-5.3-codex-spark")
        );
        assert!(
            models
                .iter()
                .any(|model| model.id == "opencode/claude-sonnet-4-6")
        );
    }
}
