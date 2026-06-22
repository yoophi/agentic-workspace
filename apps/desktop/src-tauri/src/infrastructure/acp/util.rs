use anyhow::Result;
use serde_json::Value;
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn expand_tilde(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home);
        }
    }
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(path)
}

pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        Ok(path.canonicalize()?)
    } else if let Some(parent) = path.parent() {
        let parent = if parent.as_os_str().is_empty() {
            env::current_dir()?
        } else if parent.exists() {
            parent.canonicalize()?
        } else {
            normalize_path(parent)?
        };
        Ok(parent.join(path.file_name().unwrap_or_default()))
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

pub fn clean_tool_title(title: Option<&str>) -> String {
    let text = title.unwrap_or("").trim();
    if text.eq_ignore_ascii_case("none") {
        String::new()
    } else {
        text.to_string()
    }
}

pub fn extract_locations(update: &Value) -> Vec<String> {
    update
        .get("locations")
        .and_then(Value::as_array)
        .map(|locations| {
            locations
                .iter()
                .filter_map(|location| location.get("path").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn string_param<'a>(params: &'a Value, key: &str) -> Result<&'a str> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing string param: {key}"))
}

pub fn select_lines(content: &str, start: usize, limit: Option<usize>) -> String {
    let mut selected = String::new();
    let iter = content.split_inclusive('\n').skip(start);
    match limit {
        Some(limit) => {
            for line in iter.take(limit) {
                selected.push_str(line);
            }
        }
        None => {
            for line in iter {
                selected.push_str(line);
            }
        }
    }
    selected
}

pub fn display_command(command: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(command.to_string());
    parts.extend(args.iter().cloned());
    shell_words::join(parts)
}

pub fn rpc_to_anyhow(err: RpcError) -> anyhow::Error {
    match err.data {
        Some(data) => anyhow::anyhow!("JSON-RPC error {}: {} ({data})", err.code, err.message),
        None => anyhow::anyhow!("JSON-RPC error {}: {}", err.code, err.message),
    }
}

#[derive(Debug)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}
