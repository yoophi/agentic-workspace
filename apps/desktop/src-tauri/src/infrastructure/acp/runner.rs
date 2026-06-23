use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ClientCapabilities, ContentBlock, FileSystemCapabilities, Implementation,
        InitializeRequest, NewSessionRequest, PromptRequest, StopReason, TextContent,
    },
};
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
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
        run::{AgentRunRequest, PermissionMode, RalphLoopRequest, ResumePolicy},
    },
    infrastructure::acp::{
        client::{AcpClient, lifecycle},
        transport::{RpcPeer, read_loop},
        util::{
            RpcError, display_command, enriched_path, expand_tilde, normalize_path,
            resolve_program, rpc_to_anyhow,
        },
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

        // GUI 앱의 제한된 PATH로는 npx/node 등을 찾지 못하므로, 로그인 셸에서
        // 보강한 PATH로 프로그램을 절대경로로 resolve하고 자식 환경에도 주입한다.
        let program = resolve_program(&agent_argv[0]);
        let mut child = Command::new(&program)
            .args(&agent_argv[1..])
            .current_dir(&workspace)
            .env("PATH", enriched_path())
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
            request.auto_allow.unwrap_or(false),
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
        let session_setup = if let Some(session_id) =
            resume_session_id(request.resume_session_id.as_deref(), resume_policy)
        {
            sink.emit(
                &run_id,
                RunEvent::Diagnostic {
                    message: format!("resuming ACP session {session_id}"),
                },
            );
            AcpCreatedSession {
                session_id,
                response: Value::Null,
            }
        } else {
            create_agent_session(&peer, &workspace).await?
        };
        let session_id = session_setup.session_id;
        sink.emit(
            &run_id,
            lifecycle(LifecycleStatus::SessionCreated, session_id.clone()),
        );
        apply_permission_mode(
            &peer,
            &run_id,
            &session_id,
            &request.agent_id,
            request.permission_mode.unwrap_or_default(),
            &session_setup.response,
            &sink,
        )
        .await?;
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
            permission_mode: request.permission_mode.unwrap_or_default(),
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
    permission_mode: PermissionMode,
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
                    let new_session = create_agent_session(&self.peer, &self.workspace).await?;
                    let new_id = new_session.session_id;
                    sink.emit(
                        &self.run_id,
                        lifecycle(LifecycleStatus::SessionCreated, new_id.clone()),
                    );
                    apply_permission_mode(
                        &self.peer,
                        &self.run_id,
                        &new_id,
                        &self.session_record.agent_id,
                        self.permission_mode,
                        &new_session.response,
                        &sink,
                    )
                    .await?;
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

struct AcpCreatedSession {
    session_id: String,
    response: Value,
}

async fn create_agent_session(peer: &RpcPeer, workspace: &PathBuf) -> Result<AcpCreatedSession> {
    let params = serde_json::to_value(NewSessionRequest::new(workspace.clone()))?;
    let response = peer
        .request("session/new", params)
        .await
        .map_err(rpc_to_anyhow)?;
    let session_id = response
        .get("sessionId")
        .or_else(|| response.get("session_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("session/new response missing sessionId"))?
        .to_string();

    Ok(AcpCreatedSession {
        session_id,
        response,
    })
}

async fn apply_permission_mode<S>(
    peer: &RpcPeer,
    run_id: &str,
    session_id: &str,
    agent_id: &str,
    permission_mode: PermissionMode,
    session_response: &Value,
    sink: &S,
) -> Result<()>
where
    S: RunEventSink,
{
    let candidates = permission_mode_candidates(agent_id, permission_mode);
    let selected_config_mode = candidates
        .iter()
        .find(|candidate| session_config_option_supports(session_response, "mode", candidate));
    let selected_session_mode = candidates
        .iter()
        .find(|candidate| session_modes_support(session_response, candidate));
    let Some(selected_mode) = selected_config_mode.or(selected_session_mode) else {
        sink.emit(
            run_id,
            RunEvent::Diagnostic {
                message: format!(
                    "permission mode {:?} is not advertised by agent {agent_id}; continuing with the agent default",
                    permission_mode
                ),
            },
        );
        return Ok(());
    };

    if selected_config_mode.is_some() {
        peer.request(
            "session/set_config_option",
            json!({
                "sessionId": session_id,
                "configId": "mode",
                "value": selected_mode,
            }),
        )
        .await
        .map_err(rpc_to_anyhow)?;
    } else {
        peer.request(
            "session/set_mode",
            json!({
                "sessionId": session_id,
                "modeId": selected_mode,
            }),
        )
        .await
        .map_err(rpc_to_anyhow)?;
    }

    sink.emit(
        run_id,
        RunEvent::Diagnostic {
            message: format!(
                "permission mode {:?} applied as agent mode {selected_mode}",
                permission_mode
            ),
        },
    );
    Ok(())
}

fn permission_mode_candidates(
    agent_id: &str,
    permission_mode: PermissionMode,
) -> Vec<&'static str> {
    match agent_id {
        "claude-code" => match permission_mode {
            PermissionMode::Default => vec!["default"],
            PermissionMode::Auto => vec!["auto"],
            PermissionMode::ReadOnly => vec!["default"],
            PermissionMode::Plan => vec!["plan"],
            PermissionMode::AcceptEdits => vec!["acceptEdits"],
            PermissionMode::DangerouslySkipAllPermissions => vec!["bypassPermissions"],
        },
        "codex" => match permission_mode {
            PermissionMode::Default | PermissionMode::Auto | PermissionMode::AcceptEdits => {
                vec!["agent"]
            }
            PermissionMode::ReadOnly | PermissionMode::Plan => vec!["read-only"],
            PermissionMode::DangerouslySkipAllPermissions => vec!["agent-full-access"],
        },
        _ => match permission_mode {
            PermissionMode::Default => vec!["default", "agent"],
            PermissionMode::Auto => vec!["auto", "agent"],
            PermissionMode::ReadOnly => vec!["read-only", "default"],
            PermissionMode::Plan => vec!["plan", "read-only"],
            PermissionMode::AcceptEdits => vec!["acceptEdits", "agent"],
            PermissionMode::DangerouslySkipAllPermissions => {
                vec!["bypassPermissions", "agent-full-access"]
            }
        },
    }
}

fn session_config_option_supports(response: &Value, config_id: &str, value: &str) -> bool {
    response
        .get("configOptions")
        .or_else(|| response.get("config_options"))
        .and_then(Value::as_array)
        .is_some_and(|options| {
            options.iter().any(|option| {
                option.get("id").and_then(Value::as_str) == Some(config_id)
                    && option
                        .get("options")
                        .and_then(Value::as_array)
                        .is_some_and(|values| {
                            values.iter().any(|option_value| {
                                option_value.get("value").and_then(Value::as_str) == Some(value)
                            })
                        })
            })
        })
}

fn session_modes_support(response: &Value, mode_id: &str) -> bool {
    response
        .get("modes")
        .and_then(|modes| {
            modes
                .get("availableModes")
                .or_else(|| modes.get("available_modes"))
        })
        .and_then(Value::as_array)
        .is_some_and(|modes| {
            modes
                .iter()
                .any(|mode| mode.get("id").and_then(Value::as_str) == Some(mode_id))
        })
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
