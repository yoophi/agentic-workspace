//! Codex App Server의 계정별 모델 목록. 대화나 유료 추론 요청을 만들지 않는다.
use crate::{
    domain::agent::AgentOptionDescriptor,
    infrastructure::acp::util::{enriched_path, resolve_program},
};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::{
    process::Stdio,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

type Options = (Vec<AgentOptionDescriptor>, Vec<AgentOptionDescriptor>);
type CachedOptions = Option<(Instant, Option<Options>)>;
static CACHE: OnceLock<Mutex<CachedOptions>> = OnceLock::new();

pub(super) fn discover_models() -> Option<Options> {
    let mut cache = CACHE.get_or_init(|| Mutex::new(None)).lock().ok()?;
    if let Some((at, result)) = &*cache {
        let ttl = if result.is_some() { 300 } else { 15 };
        if at.elapsed() < Duration::from_secs(ttl) {
            return result.clone();
        }
    }
    // AgentCatalog는 동기 포트이므로 호출자의 Tokio runtime에 중첩하지 않는다.
    let result = std::thread::spawn(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()?;
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(30), query_models())
                .await
                .ok()?
                .ok()
        })
    })
    .join()
    .ok()
    .flatten();
    *cache = Some((Instant::now(), result.clone()));
    result
}

async fn query_models() -> Result<Options> {
    let mut child = Command::new(resolve_program("npx"))
        .args([
            "-y",
            "--prefer-online",
            "--package",
            "@agentclientprotocol/codex-acp@latest",
            "codex",
            "app-server",
        ])
        .env("PATH", enriched_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let mut input = child.stdin.take().ok_or_else(|| anyhow!("missing stdin"))?;
    let mut lines = BufReader::new(
        child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("missing stdout"))?,
    )
    .lines();
    let result = async {
        send(&mut input, json!({"id":1,"method":"initialize","params":{"clientInfo":{"name":"aw-model-discovery","version":env!("CARGO_PKG_VERSION")}}})).await?;
        receive(&mut lines, 1).await?;
        send(&mut input, json!({"method":"initialized"})).await?;
        let mut cursor = Value::Null;
        let mut entries = Vec::new();
        // 모든 페이지를 읽되 잘못된 cursor로 무한 반복하지 않는다.
        for id in 2..102 {
            send(&mut input, json!({"id":id,"method":"model/list","params":{"limit":100,"cursor":cursor}})).await?;
            let result = receive(&mut lines, id).await?;
            entries.extend(result.get("data").and_then(Value::as_array).ok_or_else(|| anyhow!("invalid model/list response"))?.iter().cloned());
            let next = result.get("nextCursor").cloned().unwrap_or(Value::Null);
            if next.is_null() { return parse_models(&entries); }
            if next == cursor { bail!("model/list repeated cursor"); }
            cursor = next;
        }
        bail!("model/list exceeded page limit")
    }.await;
    let _ = child.kill().await;
    let _ = child.wait().await;
    result
}

async fn send(input: &mut tokio::process::ChildStdin, value: Value) -> Result<()> {
    input.write_all(format!("{value}\n").as_bytes()).await?;
    input.flush().await?;
    Ok(())
}
async fn receive(
    lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    id: u64,
) -> Result<Value> {
    while let Some(line) = lines.next_line().await? {
        let value: Value = serde_json::from_str(&line)?;
        if value.get("id").and_then(Value::as_u64) == Some(id) {
            if value.get("error").is_some() {
                bail!("Codex model discovery failed");
            }
            return value
                .get("result")
                .cloned()
                .ok_or_else(|| anyhow!("missing result"));
        }
    }
    bail!("Codex model discovery process exited")
}
fn parse_models(entries: &[Value]) -> Result<Options> {
    let mut models = Vec::<AgentOptionDescriptor>::new();
    let mut efforts = Vec::<AgentOptionDescriptor>::new();
    for entry in entries {
        if entry.get("hidden").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        // 표시용 id와 실행용 model이 다르면 실행용 식별자를 저장한다.
        let Some(id) = entry
            .get("model")
            .or_else(|| entry.get("id"))
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        else {
            continue;
        };
        if !models.iter().any(|model| model.id == id) {
            models.push(AgentOptionDescriptor {
                id: id.into(),
                label: entry
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or(id)
                    .into(),
            });
        }
        if let Some(values) = entry
            .get("supportedReasoningEfforts")
            .and_then(Value::as_array)
        {
            for value in values {
                if let Some(id) = value.get("reasoningEffort").and_then(Value::as_str) {
                    if !efforts.iter().any(|effort| effort.id == id) {
                        efforts.push(AgentOptionDescriptor {
                            id: id.into(),
                            label: id.into(),
                        });
                    }
                }
            }
        }
    }
    if models.is_empty() {
        bail!("Codex returned no visible models");
    }
    Ok((models, efforts))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_list_uses_runtime_ids_and_filters_hidden_models() {
        let (models, efforts) = parse_models(&[
            json!({"id":"display-id", "model":"gpt-6.1-sol", "displayName":"GPT-6.1 Sol", "supportedReasoningEfforts":[{"reasoningEffort":"high"}]}),
            json!({"id":"hidden", "hidden":true}),
            json!({"model":"gpt-6.1-sol"}),
            json!({"model":"another", "supportedReasoningEfforts":[{"reasoningEffort":"high"},{"reasoningEffort":"ultra"}]})
        ]).unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "gpt-6.1-sol");
        assert_eq!(models[0].label, "GPT-6.1 Sol");
        assert_eq!(
            efforts.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(),
            ["high", "ultra"]
        );
        assert!(parse_models(&[]).is_err());
    }
    #[test]
    #[ignore = "실제 npm/로그인된 Codex 계정이 필요한 smoke test"]
    fn discovers_real_models() {
        let (models, _) = discover_models().expect("live model/list");
        assert!(!models.is_empty());
    }
}
