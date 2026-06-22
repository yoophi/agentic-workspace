use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ClientCapabilities, ContentBlock, FileSystemCapabilities, Implementation,
        InitializeRequest, NewSessionRequest, PromptRequest, StopReason, TextContent,
    },
};
use anyhow::{Context, Result, anyhow, bail};
use std::{fs, future::Future, path::PathBuf, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
    task::JoinHandle,
};

use crate::{
    domain::{
        acp_session::AcpSessionRecord,
        events::{LifecycleStatus, RunEvent},
        run::{AgentRunRequest, RalphLoopRequest, ResumePolicy},
    },
    infrastructure::acp::{
        client::{AcpClient, lifecycle},
        transport::{RpcPeer, read_loop},
        util::{RpcError, display_command, expand_tilde, normalize_path, rpc_to_anyhow},
    },
    ports::{
        acp_session_store::AcpSessionStore,
        agent_catalog::AgentCatalog,
        event_sink::RunEventSink,
        permission::PermissionDecisionPort,
        session_handle::SessionHandle,
        session_launcher::{
            AbortFuture, DriverFuture, LaunchedSession, RunCommander, SessionLauncher,
        },
    },
};

const DEFAULT_WORKDIR: &str = "~/tmp/acp-tauri-agent-workspace";
const DEFAULT_STDIO_BUFFER_LIMIT_MB: usize = 50;

pub struct AcpAgentRunner<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    catalog: C,
    permissions: P,
    session_store: Arc<dyn AcpSessionStore>,
}

impl<C, P> AcpAgentRunner<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    pub fn new(catalog: C, permissions: P, session_store: Arc<dyn AcpSessionStore>) -> Self {
        Self {
            catalog,
            permissions,
            session_store,
        }
    }

    pub async fn start_session<S>(
        &self,
        request: &AgentRunRequest,
        run_id: String,
        sink: S,
    ) -> Result<AcpSessionSetup>
    where
        S: RunEventSink,
    {
        let workspace = normalize_workspace(request.cwd.as_deref().unwrap_or(DEFAULT_WORKDIR))?;
        fs::create_dir_all(&workspace)?;

        let agent_command = request
            .agent_command
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .or_else(|| self.catalog.command_for_agent(&request.agent_id))
            .ok_or_else(|| anyhow!("unknown agent: {}", request.agent_id))?;
        let agent_argv =
            shell_words::split(&agent_command).context("agent command cannot be parsed")?;
        if agent_argv.is_empty() {
            bail!("agent command cannot be empty");
        }

        sink.emit(
            &run_id,
            lifecycle(
                LifecycleStatus::Started,
                format!(
                    "{} in {}",
                    display_command(&agent_argv[0], &agent_argv[1..]),
                    workspace.display()
                ),
            ),
        );

        let mut child = Command::new(&agent_argv[0])
            .args(&agent_argv[1..])
            .current_dir(&workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("spawning ACP agent {}", agent_argv[0]))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("agent stdin is unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("agent stdout is unavailable"))?;
        let stderr = child.stderr.take();

        let peer = RpcPeer::new(stdin);
        let client = Arc::new(AcpClient::new(
            run_id.clone(),
            workspace.clone(),
            request.auto_allow.unwrap_or(true),
            self.permissions.clone(),
            sink.clone(),
        ));
        let read_task = tokio::spawn(read_loop(
            BufReader::new(stdout),
            peer.clone(),
            Arc::clone(&client),
            request
                .stdio_buffer_limit_mb
                .unwrap_or(DEFAULT_STDIO_BUFFER_LIMIT_MB)
                * 1024
                * 1024,
        ));

        let stderr_task = stderr.map(|stderr| {
            let sink = sink.clone();
            let run_id = run_id.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    sink.emit(&run_id, RunEvent::Diagnostic { message: line });
                }
            })
        });

        let init = peer
            .request_typed(initialize_request())
            .await
            .map_err(rpc_to_anyhow)?;
        let agent_name = init
            .agent_info
            .as_ref()
            .map_or("unknown", |info| &info.name);
        let agent_version = init.agent_info.as_ref().map_or("", |info| &info.version);
        sink.emit(
            &run_id,
            lifecycle(
                LifecycleStatus::Initialized,
                format!("{agent_name} {agent_version}").trim().to_string(),
            ),
        );

        let resume_policy = request.resume_policy.unwrap_or_default();
        let session_id = if let Some(session_id) =
            resume_session_id(request.resume_session_id.as_deref(), resume_policy)
        {
            sink.emit(
                &run_id,
                RunEvent::Diagnostic {
                    message: format!("resuming ACP session {session_id}"),
                },
            );
            session_id
        } else {
            create_agent_session(&peer, &workspace).await?
        };
        sink.emit(
            &run_id,
            lifecycle(LifecycleStatus::SessionCreated, session_id.clone()),
        );
        let session_record = AcpSessionRecord::from_request_with_agent_command(
            &run_id,
            &session_id,
            request,
            Some(&agent_command),
        );
        self.session_store
            .record_session(session_record.clone())
            .await?;

        let session = Arc::new(AcpSession {
            run_id,
            session_id: Mutex::new(session_id),
            workspace,
            peer,
            resume_policy,
            session_record,
            session_store: self.session_store.clone(),
            in_flight: Mutex::new(()),
        });

        Ok(AcpSessionSetup {
            session,
            child,
            read_task,
            stderr_task,
        })
    }
}

pub struct AcpSessionSetup {
    pub session: Arc<AcpSession>,
    pub child: Child,
    pub read_task: JoinHandle<Result<()>>,
    pub stderr_task: Option<JoinHandle<()>>,
}

impl<C, P> SessionLauncher for AcpAgentRunner<C, P>
where
    C: AgentCatalog,
    P: PermissionDecisionPort,
{
    type Session = AcpSession;

    async fn launch<S>(
        self,
        request: AgentRunRequest,
        run_id: String,
        sink: S,
    ) -> Result<LaunchedSession<AcpSession>>
    where
        S: RunEventSink,
    {
        let setup = self
            .start_session(&request, run_id.clone(), sink.clone())
            .await?;
        let AcpSessionSetup {
            session,
            child,
            read_task,
            stderr_task,
        } = setup;

        let commander = AcpRunCommander {
            child,
            read_task,
            stderr_task,
            session: session.clone(),
            sink,
            run_id,
            initial_goal: request.goal,
            ralph_loop: request.ralph_loop,
        };

        Ok(LaunchedSession {
            session,
            commander: Box::new(commander),
        })
    }
}

struct AcpRunCommander<S>
where
    S: RunEventSink,
{
    child: Child,
    read_task: JoinHandle<Result<()>>,
    stderr_task: Option<JoinHandle<()>>,
    session: Arc<AcpSession>,
    sink: S,
    run_id: String,
    initial_goal: String,
    ralph_loop: Option<RalphLoopRequest>,
}

impl<S> RunCommander for AcpRunCommander<S>
where
    S: RunEventSink,
{
    fn run_to_completion(self: Box<Self>) -> DriverFuture {
        Box::pin(async move {
            let Self {
                mut child,
                read_task,
                stderr_task,
                session,
                sink,
                run_id,
                initial_goal,
                ralph_loop,
            } = *self;

            run_prompt_sequence(sink.clone(), &run_id, initial_goal, ralph_loop, |prompt| {
                let session = session.clone();
                let sink = sink.clone();
                async move { session.send_prompt(sink, prompt).await.map(|_| ()) }
            })
            .await;

            match child.wait().await {
                Ok(status) => {
                    if let Some(code) = status.code() {
                        if code != 0 {
                            sink.emit(
                                &run_id,
                                RunEvent::Diagnostic {
                                    message: format!("agent process exited with code {code}"),
                                },
                            );
                        }
                    }
                }
                Err(err) => {
                    sink.emit(
                        &run_id,
                        RunEvent::Diagnostic {
                            message: format!("failed to wait for agent process: {err}"),
                        },
                    );
                }
            }

            read_task.abort();
            let _ = read_task.await;
            if let Some(task) = stderr_task {
                task.abort();
            }

            sink.emit(
                &run_id,
                lifecycle(LifecycleStatus::Completed, "agent exited"),
            );
        })
    }

    fn abort(self: Box<Self>) -> AbortFuture {
        Box::pin(async move {
            let Self {
                mut child,
                read_task,
                stderr_task,
                ..
            } = *self;
            let _ = child.start_kill();
            let _ = child.wait().await;
            read_task.abort();
            if let Some(task) = stderr_task {
                task.abort();
            }
        })
    }
}

async fn run_prompt_sequence<S, Fut>(
    sink: S,
    run_id: &str,
    initial_goal: String,
    ralph_loop: Option<RalphLoopRequest>,
    mut send_prompt: impl FnMut(String) -> Fut,
) where
    S: RunEventSink,
    Fut: Future<Output = Result<()>>,
{
    if send_loop_prompt(sink.clone(), run_id, initial_goal, &mut send_prompt)
        .await
        .is_err()
    {
        return;
    }

    let Some(settings) = ralph_loop.filter(|settings| settings.enabled) else {
        return;
    };
    let prompt = settings.prompt_template.trim().to_string();
    if prompt.is_empty() {
        sink.emit(
            run_id,
            RunEvent::Diagnostic {
                message: "Ralph loop stopped: loop prompt is empty".into(),
            },
        );
        return;
    }

    for iteration in 1..=settings.max_iterations {
        if settings.delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(settings.delay_ms)).await;
        }
        sink.emit(
            run_id,
            RunEvent::Diagnostic {
                message: format!(
                    "Ralph loop iteration {iteration}/{} started",
                    settings.max_iterations
                ),
            },
        );
        if send_loop_prompt(sink.clone(), run_id, prompt.clone(), &mut send_prompt)
            .await
            .is_err()
        {
            if settings.stop_on_error {
                sink.emit(
                    run_id,
                    RunEvent::Diagnostic {
                        message: format!(
                            "Ralph loop stopped after iteration {iteration}: prompt dispatch failed"
                        ),
                    },
                );
                return;
            }
        }
    }

    sink.emit(
        run_id,
        RunEvent::Diagnostic {
            message: format!(
                "Ralph loop stopped: reached max iterations ({})",
                settings.max_iterations
            ),
        },
    );
}

async fn send_loop_prompt<S, Fut>(
    sink: S,
    run_id: &str,
    prompt: String,
    send_prompt: &mut impl FnMut(String) -> Fut,
) -> Result<()>
where
    S: RunEventSink,
    Fut: Future<Output = Result<()>>,
{
    send_prompt(prompt).await.map_err(|err| {
        sink.emit(
            run_id,
            RunEvent::Error {
                message: err.to_string(),
            },
        );
        err
    })
}

pub struct AcpSession {
    pub run_id: String,
    pub peer: RpcPeer,
    session_id: Mutex<String>,
    workspace: PathBuf,
    resume_policy: ResumePolicy,
    session_record: AcpSessionRecord,
    session_store: Arc<dyn AcpSessionStore>,
    in_flight: Mutex<()>,
}

impl AcpSession {
    pub async fn session_id(&self) -> String {
        self.session_id.lock().await.clone()
    }
}

impl SessionHandle for AcpSession {
    async fn send_prompt<S>(&self, sink: S, text: String) -> Result<String>
    where
        S: RunEventSink,
    {
        let _guard = self
            .in_flight
            .try_lock()
            .map_err(|_| anyhow!("agent is still responding to the previous prompt"))?;

        sink.emit(
            &self.run_id,
            lifecycle(LifecycleStatus::PromptSent, "prompt submitted"),
        );

        let mut reissued = false;
        let stop_reason = loop {
            let current_id = self.session_id().await;
            let outcome = self
                .peer
                .request_typed(PromptRequest::new(
                    current_id.clone(),
                    vec![ContentBlock::Text(TextContent::new(text.clone()))],
                ))
                .await;
            match outcome {
                Ok(response) => break stop_reason_label(response.stop_reason),
                Err(err) if !reissued && is_session_not_found(&err) => {
                    if !should_reissue_missing_session(self.resume_policy) {
                        return Err(anyhow!("resume session not found: {current_id}"));
                    }
                    reissued = true;
                    sink.emit(
                        &self.run_id,
                        RunEvent::Diagnostic {
                            message: "agent dropped the previous session; creating a new one"
                                .into(),
                        },
                    );
                    let new_id = create_agent_session(&self.peer, &self.workspace).await?;
                    sink.emit(
                        &self.run_id,
                        lifecycle(LifecycleStatus::SessionCreated, new_id.clone()),
                    );
                    let mut record = self.session_record.clone();
                    record.session_id.clone_from(&new_id);
                    self.session_store.record_session(record).await?;
                    *self.session_id.lock().await = new_id;
                    continue;
                }
                Err(err) => return Err(rpc_to_anyhow(err)),
            }
        };

        sink.emit(
            &self.run_id,
            lifecycle(
                LifecycleStatus::PromptCompleted,
                format!("stopReason={stop_reason}"),
            ),
        );
        Ok(stop_reason)
    }
}

fn normalize_workspace(path: &str) -> Result<PathBuf> {
    normalize_path(&expand_tilde(path))
}

fn resume_session_id(session_id: Option<&str>, resume_policy: ResumePolicy) -> Option<String> {
    match resume_policy {
        ResumePolicy::Fresh => None,
        ResumePolicy::ResumeIfAvailable | ResumePolicy::ResumeRequired => session_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    }
}

fn should_reissue_missing_session(resume_policy: ResumePolicy) -> bool {
    !matches!(resume_policy, ResumePolicy::ResumeRequired)
}

async fn create_agent_session(peer: &RpcPeer, workspace: &PathBuf) -> Result<String> {
    let response = peer
        .request_typed(NewSessionRequest::new(workspace.clone()))
        .await
        .map_err(rpc_to_anyhow)?;
    Ok(response.session_id.to_string())
}

fn initialize_request() -> InitializeRequest {
    InitializeRequest::new(ProtocolVersion::V1)
        .client_capabilities(
            ClientCapabilities::new()
                .fs(FileSystemCapabilities::new()
                    .read_text_file(true)
                    .write_text_file(true))
                .terminal(true),
        )
        .client_info(
            Implementation::new("tauri-acp-agent-workbench", env!("CARGO_PKG_VERSION"))
                .title("Tauri ACP Agent Workbench".to_string()),
        )
}

fn stop_reason_label(stop_reason: StopReason) -> String {
    serde_json::to_value(stop_reason)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn is_session_not_found(err: &RpcError) -> bool {
    const MARKER: &str = "Session not found";
    if err.message.contains(MARKER) {
        return true;
    }
    let Some(data) = &err.data else {
        return false;
    };
    if let Some(text) = data.as_str() {
        if text.contains(MARKER) {
            return true;
        }
    }
    if let Some(details) = data.get("details").and_then(serde_json::Value::as_str) {
        if details.contains(MARKER) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{resume_session_id, run_prompt_sequence, should_reissue_missing_session};
    use crate::{
        domain::{
            events::RunEvent,
            run::{RalphLoopRequest, ResumePolicy},
        },
        ports::event_sink::RunEventSink,
    };
    use anyhow::anyhow;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone, Default)]
    struct CollectingSink {
        events: Arc<Mutex<Vec<(String, RunEvent)>>>,
    }

    impl RunEventSink for CollectingSink {
        fn emit(&self, run_id: &str, event: RunEvent) {
            self.events
                .lock()
                .unwrap()
                .push((run_id.to_string(), event));
        }
    }

    fn ralph_loop(max_iterations: usize) -> RalphLoopRequest {
        RalphLoopRequest {
            enabled: true,
            max_iterations,
            prompt_template: "continue".into(),
            stop_on_error: true,
            stop_on_permission: true,
            delay_ms: 0,
        }
    }

    #[test]
    fn fresh_policy_ignores_resume_session_id() {
        assert_eq!(
            resume_session_id(Some("session-1"), ResumePolicy::Fresh),
            None
        );
    }

    #[test]
    fn resume_policies_use_non_empty_session_id() {
        assert_eq!(
            resume_session_id(Some(" session-1 "), ResumePolicy::ResumeIfAvailable),
            Some("session-1".to_string())
        );
        assert_eq!(
            resume_session_id(Some("session-2"), ResumePolicy::ResumeRequired),
            Some("session-2".to_string())
        );
        assert_eq!(
            resume_session_id(Some("  "), ResumePolicy::ResumeRequired),
            None
        );
    }

    #[test]
    fn resume_required_does_not_reissue_missing_session() {
        assert!(should_reissue_missing_session(ResumePolicy::Fresh));
        assert!(should_reissue_missing_session(
            ResumePolicy::ResumeIfAvailable
        ));
        assert!(!should_reissue_missing_session(
            ResumePolicy::ResumeRequired
        ));
    }

    #[tokio::test]
    async fn ralph_loop_sends_follow_up_prompts_until_max_iterations() {
        let sink = CollectingSink::default();
        let prompts = Arc::new(Mutex::new(Vec::new()));

        run_prompt_sequence(
            sink.clone(),
            "run-1",
            "initial".into(),
            Some(ralph_loop(2)),
            {
                let prompts = prompts.clone();
                move |prompt| {
                    let prompts = prompts.clone();
                    async move {
                        prompts.lock().unwrap().push(prompt);
                        Ok(())
                    }
                }
            },
        )
        .await;

        assert_eq!(
            prompts.lock().unwrap().as_slice(),
            ["initial", "continue", "continue"]
        );
        let events = sink.events.lock().unwrap();
        assert!(events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Diagnostic { message }
                if message == "Ralph loop iteration 1/2 started"
        )));
        assert!(events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Diagnostic { message }
                if message == "Ralph loop stopped: reached max iterations (2)"
        )));
    }

    #[tokio::test]
    async fn ralph_loop_stops_on_prompt_dispatch_failure() {
        let sink = CollectingSink::default();
        let prompts = Arc::new(Mutex::new(Vec::new()));
        let attempts = Arc::new(AtomicUsize::new(0));

        run_prompt_sequence(
            sink.clone(),
            "run-1",
            "initial".into(),
            Some(ralph_loop(3)),
            {
                let prompts = prompts.clone();
                let attempts = attempts.clone();
                move |prompt| {
                    let prompts = prompts.clone();
                    let attempts = attempts.clone();
                    async move {
                        prompts.lock().unwrap().push(prompt);
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                        if attempt == 2 {
                            Err(anyhow!("dispatch failed"))
                        } else {
                            Ok(())
                        }
                    }
                }
            },
        )
        .await;

        assert_eq!(prompts.lock().unwrap().as_slice(), ["initial", "continue"]);
        let events = sink.events.lock().unwrap();
        assert!(events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Error { message } if message == "dispatch failed"
        )));
        assert!(events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Diagnostic { message }
                if message == "Ralph loop stopped after iteration 1: prompt dispatch failed"
        )));
        assert!(!events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Diagnostic { message }
                if message == "Ralph loop iteration 2/3 started"
        )));
    }
}
