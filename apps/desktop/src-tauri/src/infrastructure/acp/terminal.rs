use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    domain::events::RunEvent,
    infrastructure::acp::util::{display_command, string_param},
};

struct TerminalState {
    child: Arc<Mutex<Child>>,
    output: Arc<Mutex<Vec<u8>>>,
    output_limit: usize,
    reader_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

pub struct TerminalHandler {
    terminals: Mutex<HashMap<String, TerminalState>>,
}

impl TerminalHandler {
    pub fn new() -> Self {
        Self {
            terminals: Mutex::new(HashMap::new()),
        }
    }

    pub async fn create<F>(
        &self,
        params: Value,
        workspace: PathBuf,
        resolve_inside_workspace: impl Fn(&str) -> Result<PathBuf>,
        emit: F,
    ) -> Result<Value>
    where
        F: Fn(RunEvent),
    {
        let command = string_param(&params, "command")?;
        let args = params
            .get("args")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let working_dir = if let Some(cwd) = params.get("cwd").and_then(Value::as_str) {
            resolve_inside_workspace(cwd)?
        } else {
            workspace
        };
        let mut cmd = Command::new(command);
        cmd.args(&args)
            .current_dir(&working_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(env_items) = params.get("env").and_then(Value::as_array) {
            for item in env_items {
                if let (Some(name), Some(value)) = (
                    item.get("name").and_then(Value::as_str),
                    item.get("value").and_then(Value::as_str),
                ) {
                    cmd.env(name, value);
                }
            }
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("spawning terminal command {command}"))?;
        let terminal_id = Uuid::new_v4().to_string();
        let output_limit = params
            .get("outputByteLimit")
            .and_then(Value::as_u64)
            .unwrap_or(256_000) as usize;

        emit(RunEvent::Terminal {
            operation: "create".into(),
            terminal_id: Some(terminal_id.clone()),
            message: format!(
                "{} (cwd={})",
                display_command(command, &args),
                working_dir.display()
            ),
        });

        let output = Arc::new(Mutex::new(Vec::new()));
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        let reader_output = Arc::clone(&output);
        let reader_task = tokio::spawn(async move {
            let out_task = async {
                if let Some(stdout) = stdout.as_mut() {
                    capture_output(stdout, Arc::clone(&reader_output), output_limit).await;
                }
            };
            let err_task = async {
                if let Some(stderr) = stderr.as_mut() {
                    capture_output(stderr, Arc::clone(&reader_output), output_limit).await;
                }
            };
            tokio::join!(out_task, err_task);
        });

        self.terminals.lock().await.insert(
            terminal_id.clone(),
            TerminalState {
                child: Arc::new(Mutex::new(child)),
                output,
                output_limit,
                reader_task: Arc::new(Mutex::new(Some(reader_task))),
            },
        );
        Ok(json!({"terminalId": terminal_id}))
    }

    pub async fn output(&self, params: Value) -> Result<Value> {
        let terminal_id = string_param(&params, "terminalId")?;
        let terminals = self.terminals.lock().await;
        let state = terminals
            .get(terminal_id)
            .ok_or_else(|| anyhow!("unknown terminal id: {terminal_id}"))?;
        let output = terminal_text(state).await;
        let truncated = state.output.lock().await.len() >= state.output_limit;
        let exit_status = terminal_exit_status(state).await?;
        Ok(json!({"output": output, "truncated": truncated, "exitStatus": exit_status}))
    }

    pub async fn wait_for_exit<F>(&self, params: Value, emit: F) -> Result<Value>
    where
        F: Fn(RunEvent),
    {
        let terminal_id = string_param(&params, "terminalId")?;
        let (child, reader_task) = {
            let terminals = self.terminals.lock().await;
            let state = terminals
                .get(terminal_id)
                .ok_or_else(|| anyhow!("unknown terminal id: {terminal_id}"))?;
            (Arc::clone(&state.child), Arc::clone(&state.reader_task))
        };
        let status = child.lock().await.wait().await?;
        if let Some(reader_task) = reader_task.lock().await.take() {
            let _ = reader_task.await;
        }
        emit(RunEvent::Terminal {
            operation: "exit".into(),
            terminal_id: Some(terminal_id.to_string()),
            message: format!("{:?}", status.code()),
        });
        Ok(exit_status_json(
            status.code(),
            unix_signal_from_status(&status),
        ))
    }

    pub async fn kill<F>(&self, params: Value, emit: F) -> Result<Value>
    where
        F: Fn(RunEvent),
    {
        let terminal_id = string_param(&params, "terminalId")?;
        let terminals = self.terminals.lock().await;
        let state = terminals
            .get(terminal_id)
            .ok_or_else(|| anyhow!("unknown terminal id: {terminal_id}"))?;
        let child = state.child.lock().await;
        if let Some(pid) = child.id() {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        emit(RunEvent::Terminal {
            operation: "kill".into(),
            terminal_id: Some(terminal_id.to_string()),
            message: "SIGTERM sent".into(),
        });
        Ok(json!({}))
    }

    pub async fn release(&self, params: Value) -> Result<Value> {
        let terminal_id = string_param(&params, "terminalId")?;
        if let Some(state) = self.terminals.lock().await.remove(terminal_id) {
            if let Some(reader_task) = state.reader_task.lock().await.take() {
                reader_task.abort();
            }
        }
        Ok(json!({}))
    }
}

async fn capture_output<R>(reader: &mut R, output: Arc<Mutex<Vec<u8>>>, limit: usize)
where
    R: AsyncReadExt + Unpin,
{
    let mut buf = [0_u8; 4096];
    loop {
        let Ok(read) = reader.read(&mut buf).await else {
            return;
        };
        if read == 0 {
            return;
        }
        let mut output = output.lock().await;
        output.extend_from_slice(&buf[..read]);
        if output.len() > limit {
            let excess = output.len() - limit;
            output.drain(..excess);
        }
    }
}

async fn terminal_text(state: &TerminalState) -> String {
    String::from_utf8_lossy(&state.output.lock().await).into_owned()
}

async fn terminal_exit_status(state: &TerminalState) -> Result<Value> {
    let mut child = state.child.lock().await;
    if let Some(status) = child.try_wait()? {
        Ok(exit_status_json(
            status.code(),
            unix_signal_from_status(&status),
        ))
    } else {
        Ok(Value::Null)
    }
}

fn exit_status_json(code: Option<i32>, signal: Option<i32>) -> Value {
    if let Some(code) = code {
        json!({"exitCode": code})
    } else if let Some(signal) = signal {
        json!({"signal": signal.to_string()})
    } else {
        Value::Null
    }
}

#[cfg(unix)]
fn unix_signal_from_status(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn unix_signal_from_status(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}
