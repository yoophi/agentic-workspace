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
                // Pinned Codex 0.144.6의 known ReasoningEffort ID 집합이다.
                // 모델별 실제 지원 여부는 session configOptions로 다시 검증한다.
                efforts: options(&[
                    ("none", "None"),
                    ("minimal", "Minimal"),
                    ("low", "Low"),
                    ("medium", "Medium"),
                    ("high", "High"),
                    ("xhigh", "XHigh"),
                    ("max", "Max"),
                    ("ultra", "Ultra"),
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
                    ("claude-opus-5", "Claude Opus 5"),
                    ("claude-opus-4-8", "Claude Opus 4.8"),
                    ("claude-sonnet-5", "Claude Sonnet 5"),
                    ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
                    ("claude-haiku-4-5", "Claude Haiku 4.5"),
                ]),
                efforts: Vec::new(),
                context_sizes: Vec::new(),
            },
            AgentDescriptor {
                id: "pi-coding-agent".into(),
                label: "Pi Coding Agent".into(),
                command: "npx -y pi-acp".into(),
                runtime_version: None,
                models: Vec::new(),
                efforts: Vec::new(),
                context_sizes: Vec::new(),
            },
            AgentDescriptor {
                id: "opencode".into(),
                label: "OpenCode".into(),
                command: "npx -y opencode-ai acp".into(),
                runtime_version: None,
                models: opencode_models().unwrap_or_else(opencode_fallback_models),
                efforts: Vec::new(),
                context_sizes: Vec::new(),
            },
            AgentDescriptor {
                id: "kiro-cli".into(),
                label: "Kiro CLI".into(),
                command: "kiro-cli acp".into(),
                runtime_version: None,
                // Kiro는 session/new 응답의 ACP 표준 `models.availableModels`로 모델을 광고하고
                // `session/set_model`로 전환한다. 아래 목록은 그 광고값의 기본 스냅샷이며,
                // 실제 가용 목록은 세션 생성 시 응답으로 다시 검증한다.
                models: options(&[
                    ("auto", "Auto (task-optimized)"),
                    ("claude-opus-5", "Claude Opus 5"),
                    ("claude-sonnet-5", "Claude Sonnet 5"),
                    ("gpt-5.6-sol", "GPT-5.6 Sol"),
                    ("gpt-5.6-terra", "GPT-5.6 Terra"),
                    ("gpt-5.6-luna", "GPT-5.6 Luna"),
                    ("deepseek-3.2", "DeepSeek 3.2"),
                    ("minimax-m2.5", "MiniMax M2.5"),
                    ("glm-5", "GLM-5"),
                ]),
                // `kiro-cli acp --effort`가 받는 thinking effort 단계다.
                efforts: options(&[
                    ("low", "Low"),
                    ("medium", "Medium"),
                    ("high", "High"),
                    ("xhigh", "XHigh"),
                    ("max", "Max"),
                ]),
                context_sizes: Vec::new(),
            },
        ]
    }
}

/// ACP `configOptions`를 광고하지 않고 CLI 인자로만 모델/effort를 받는 에이전트의 플래그.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliRunOptionFlags {
    pub model: Option<&'static str>,
    pub effort: Option<&'static str>,
}

/// 실행 설정을 기동 인자로 넘겨야 하는 에이전트인지 판별한다.
///
/// Kiro CLI는 세션 응답에 ACP 표준 `models.availableModels`만 싣고
/// `session/set_config_option`은 지원하지 않는다. 따라서 모델·effort 선택값은
/// `kiro-cli acp --model <id> --effort <id>` 형태로 프로세스 기동 시 넘겨야 반영된다.
pub fn cli_run_option_flags(agent_id: &str) -> Option<CliRunOptionFlags> {
    match agent_id {
        "kiro-cli" => Some(CliRunOptionFlags {
            model: Some("--model"),
            effort: Some("--effort"),
        }),
        _ => None,
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
    if !refresh {
        if let Some(cache_path) = &cache_path {
            if let Some(models) = read_cached_opencode_models(cache_path) {
                return Some(models);
            }
        }
    }

    let models = fetch_opencode_models_from_models_dev()
        .or_else(|| cache_path.as_ref().and_then(read_cached_opencode_models))?;

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
    fn kiro_catalog_exposes_acp_command_with_models_and_efforts() {
        let agents = StaticAgentCatalog.list_agents();
        let kiro = agents
            .iter()
            .find(|agent| agent.id == "kiro-cli")
            .expect("Kiro CLI agent");

        assert_eq!(kiro.command, "kiro-cli acp");
        for model_id in [
            "auto",
            "claude-opus-5",
            "claude-sonnet-5",
            "gpt-5.6-sol",
            "glm-5",
        ] {
            assert!(
                kiro.models.iter().any(|model| model.id == model_id),
                "missing Kiro model {model_id}"
            );
        }
        assert_eq!(
            kiro.efforts
                .iter()
                .map(|effort| effort.id.as_str())
                .collect::<Vec<_>>(),
            vec!["low", "medium", "high", "xhigh", "max"]
        );
    }

    #[test]
    fn only_kiro_receives_run_options_as_cli_flags() {
        let flags = cli_run_option_flags("kiro-cli").expect("Kiro CLI flags");
        assert_eq!(flags.model, Some("--model"));
        assert_eq!(flags.effort, Some("--effort"));

        // 나머지 에이전트는 세션 configOptions로 설정을 받는다.
        assert!(cli_run_option_flags("codex").is_none());
        assert!(cli_run_option_flags("claude-code").is_none());
    }

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

        assert!(models
            .iter()
            .any(|model| model.id == "opencode/claude-opus-4-8"));
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

        assert_eq!(
            codex
                .efforts
                .iter()
                .map(|effort| (effort.id.as_str(), effort.label.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("none", "None"),
                ("minimal", "Minimal"),
                ("low", "Low"),
                ("medium", "Medium"),
                ("high", "High"),
                ("xhigh", "XHigh"),
                ("max", "Max"),
                ("ultra", "Ultra"),
            ]
        );
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
            "claude-opus-5",
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

        assert!(models
            .iter()
            .any(|model| model.id == "opencode/gpt-5.3-codex-spark"));
        assert!(models
            .iter()
            .any(|model| model.id == "opencode/claude-sonnet-4-6"));
    }
}
