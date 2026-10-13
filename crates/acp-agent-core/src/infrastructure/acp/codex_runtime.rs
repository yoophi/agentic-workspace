//! npx 업데이트 대기와 ACP 초기화 제한 시간을 분리한다.
use super::util::{enriched_path, resolve_program};
use crate::{domain::events::RunEvent, ports::event_sink::RunEventSink};
use anyhow::{anyhow, bail, Context, Result};
use std::{collections::BTreeMap, path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    process::Command,
    time::{interval, timeout},
};

pub(super) fn uses_latest(command: &str) -> bool {
    let Ok(args) = shell_words::split(command) else {
        return false;
    };
    args.first().is_some_and(|p| {
        std::path::Path::new(p)
            .file_stem()
            .is_some_and(|s| s == "npx")
    }) && args
        .iter()
        .any(|arg| arg == "@agentclientprotocol/codex-acp@latest")
}

pub(super) async fn prepare<S: RunEventSink>(
    command: &str,
    env: Option<&BTreeMap<String, String>>,
    run_id: &str,
    sink: &S,
) -> Result<()> {
    prepare_with(
        command,
        env,
        run_id,
        sink,
        Preparation {
            npm: resolve_program("npm"),
            npx: resolve_program(&shell_words::split(command)?[0]),
            marker: marker_path(),
            install_timeout: Duration::from_secs(180),
        },
    )
    .await
}
struct Preparation {
    npm: String,
    npx: String,
    marker: Option<PathBuf>,
    install_timeout: Duration,
}
async fn prepare_with<S: RunEventSink>(
    command: &str,
    env: Option<&BTreeMap<String, String>>,
    run_id: &str,
    sink: &S,
    preparation: Preparation,
) -> Result<()> {
    let args = shell_words::split(command)?;
    sink.emit(
        run_id,
        RunEvent::Diagnostic {
            message: "Codex ACP 최신 버전을 확인하고 있습니다. 필요한 업데이트는 자동 설치합니다."
                .into(),
        },
    );
    let mut check = Command::new(&preparation.npm);
    check.args([
        "view",
        "@agentclientprotocol/codex-acp@latest",
        "version",
        "--json",
    ]);
    apply_env(&mut check, env);
    let output = timeout(Duration::from_secs(20), check.output()).await
        .map_err(|_| anyhow!("Codex ACP 최신 버전 확인 시간이 초과되었습니다. 네트워크/npm 레지스트리를 확인한 후 다시 시도하세요."))?
        .context("Codex ACP 최신 버전 확인 실패: npm 실행을 확인하세요.")?;
    if !output.status.success() {
        bail!("Codex ACP 최신 버전 확인에 실패했습니다. 네트워크/npm 레지스트리를 확인한 후 다시 시도하세요.");
    }
    let version: String = serde_json::from_slice(&output.stdout)
        .context("Codex ACP 버전 응답이 올바르지 않습니다.")?;
    let marker = preparation.marker;
    let previous = marker
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok());
    let message = match previous.as_deref() {
        Some(old) if old.trim() == version => {
            format!("Codex ACP {version} 실행 준비를 확인하고 있습니다.")
        }
        Some(old) => format!(
            "Codex ACP 업데이트 중: {} → {version}. 설치가 끝나면 세션을 시작합니다.",
            old.trim()
        ),
        None => format!(
            "Codex ACP {version} 설치/업데이트 준비 중입니다. 설치가 끝나면 세션을 시작합니다."
        ),
    };
    sink.emit(run_id, RunEvent::Diagnostic { message });
    let mut install = Command::new(&preparation.npx);
    install
        .arg("--prefer-online")
        .args(&args[1..])
        .arg("--version");
    apply_env(&mut install, env);
    let future = timeout(preparation.install_timeout, install.output());
    tokio::pin!(future);
    let mut progress = interval(Duration::from_secs(5));
    progress.tick().await;
    let output = loop {
        tokio::select! {
            result = &mut future => break result
                .map_err(|_| anyhow!("Codex ACP 설치/업데이트 시간이 초과되었습니다. 네트워크와 npm 캐시를 확인한 후 다시 시도하세요."))?
                .context("Codex ACP 설치/업데이트 명령 실행에 실패했습니다.")?,
            _ = progress.tick() => sink.emit(run_id, RunEvent::Diagnostic { message: "Codex ACP 설치/업데이트 확인을 기다리고 있습니다…".into() }),
        }
    };
    if !output.status.success() {
        bail!("Codex ACP 설치/업데이트에 실패했습니다. 네트워크, npm 캐시 및 디스크 권한을 확인한 후 다시 시도하세요.");
    }
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if actual.split_whitespace().last() != Some(version.as_str()) {
        bail!("Codex ACP 설치/업데이트 후 최신 버전({version})을 확인할 수 없습니다. npm 캐시를 확인한 후 다시 시도하세요.");
    }
    if let Some(path) = marker {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, &version);
    }
    sink.emit(
        run_id,
        RunEvent::Diagnostic {
            message: format!("Codex ACP 실행 준비 완료: {actual}"),
        },
    );
    Ok(())
}
fn apply_env(command: &mut Command, env: Option<&BTreeMap<String, String>>) {
    if let Some(env) = env {
        command.envs(env);
    }
    let path = env
        .and_then(|env| env.get("PATH"))
        .filter(|path| !path.trim().is_empty())
        .map(|path| format!("{path}:{}", enriched_path()))
        .unwrap_or_else(|| enriched_path().to_owned());
    command
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
}
fn marker_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".cache/agentic-workbench/codex-acp-version"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    mod process_tests {
        use super::*;
        use std::{
            os::unix::fs::PermissionsExt,
            sync::{Arc, Mutex},
        };
        #[derive(Clone, Default)]
        struct Sink(Arc<Mutex<Vec<String>>>);
        impl RunEventSink for Sink {
            fn emit(&self, _: &str, event: RunEvent) {
                if let RunEvent::Diagnostic { message } = event {
                    self.0.lock().unwrap().push(message);
                }
            }
        }
        fn script(dir: &std::path::Path, name: &str, body: &str) -> String {
            let path = dir.join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            path.to_string_lossy().into_owned()
        }
        async fn fixture(body: &str, duration: Duration) -> (Result<()>, Vec<String>, String) {
            let dir = tempfile::tempdir().unwrap();
            let marker = dir.path().join("version");
            std::fs::write(&marker, "1.1.5").unwrap();
            let settings = Preparation {
                npm: script(dir.path(), "npm", "printf '\"2.2.2\"'"),
                npx: script(dir.path(), "npx", body),
                marker: Some(marker.clone()),
                install_timeout: duration,
            };
            let sink = Sink::default();
            let result = prepare_with(
                "npx -y @agentclientprotocol/codex-acp@latest",
                None,
                "test",
                &sink,
                settings,
            )
            .await;
            let messages = sink.0.lock().unwrap().clone();
            (result, messages, std::fs::read_to_string(marker).unwrap())
        }
        #[tokio::test]
        async fn announces_upgrade_then_records_only_successful_preparation() {
            let (result, messages, version) =
                fixture("printf 'codex-acp 2.2.2'", Duration::from_secs(2)).await;
            result.unwrap();
            assert!(messages
                .iter()
                .any(|m| m.contains("업데이트 중: 1.1.5 → 2.2.2")));
            assert!(messages.last().unwrap().contains("실행 준비 완료"));
            assert_eq!(version, "2.2.2");
        }
        #[tokio::test]
        async fn stale_npx_cache_cannot_be_reported_as_updated() {
            let (result, messages, version) =
                fixture("printf 'codex-acp 1.1.5'", Duration::from_secs(2)).await;
            assert!(result.unwrap_err().to_string().contains("최신 버전(2.2.2)"));
            assert!(!messages.iter().any(|m| m.contains("준비 완료")));
            assert_eq!(version, "1.1.5");
        }
        #[tokio::test]
        async fn failed_update_is_actionable_and_does_not_record_success() {
            let (result, messages, version) = fixture("exit 1", Duration::from_secs(2)).await;
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("설치/업데이트에 실패"));
            assert!(!messages.iter().any(|m| m.contains("준비 완료")));
            assert_eq!(version, "1.1.5");
        }
        #[tokio::test]
        async fn update_timeout_is_distinct_from_acp_initialization() {
            let (result, messages, version) =
                fixture("exec sleep 10", Duration::from_millis(30)).await;
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("설치/업데이트 시간이 초과"));
            assert!(!messages.iter().any(|m| m.contains("준비 완료")));
            assert_eq!(version, "1.1.5");
        }
    }
    #[test]
    fn only_latest_npx_commands_need_preparation() {
        assert!(uses_latest("npx -y @agentclientprotocol/codex-acp@latest"));
        assert!(uses_latest(
            "/usr/local/bin/npx -y @agentclientprotocol/codex-acp@latest"
        ));
        assert!(!uses_latest("npx -y @agentclientprotocol/codex-acp@1.1.5"));
        assert!(!uses_latest("custom-codex-acp"));
    }
}
