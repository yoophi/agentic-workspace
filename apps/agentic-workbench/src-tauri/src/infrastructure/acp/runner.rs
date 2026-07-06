use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ClientCapabilities, ContentBlock, FileSystemCapabilities, Implementation,
        InitializeRequest, LoadSessionRequest, NewSessionRequest, PromptRequest, SessionId,
        StopReason, TextContent,
    },
};
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::{
    fs,
    future::Future,
    path::PathBuf,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
    task::JoinHandle,
};

use crate::{
    domain::{
        acp_session::AcpSessionRecord,
        events::{LifecycleStatus, RalphLoopStatus, RunEvent},
        run::{
            AgentMcpServerConfig, AgentRunRequest, ContextSizePreset, PermissionMode,
            RalphLoopRequest, ResumePolicy,
        },
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
        let agent_argv = shell_words::split(&agent_command)
            .with_context(|| format!("agent command cannot be parsed: {agent_command}"))?;
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
            // 프로필/global env(specs/008)와 보강 PATH를 함께 주입한다. env value는
            // 로그·오류에 노출하지 않는다.
            .envs(spawn_env_vars(request.agent_env.as_ref(), &enriched_path()))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .with_context(|| spawn_agent_error_context(&agent_command, &agent_argv[0]))?;

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
        let session_mcp_servers = if init.agent_capabilities.mcp_capabilities.http {
            request.mcp_servers.clone()
        } else {
            if !request.mcp_servers.is_empty() {
                sink.emit(
                    &run_id,
                    RunEvent::Diagnostic {
                        message:
                            "agent does not advertise HTTP MCP support; skipping MCP server setup"
                                .to_string(),
                    },
                );
            }
            Vec::new()
        };

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
            resume_or_create_session(
                &peer,
                &workspace,
                &session_id,
                init.agent_capabilities.load_session,
                resume_policy,
                &session_mcp_servers,
                &run_id,
                &sink,
            )
            .await?
        } else {
            create_agent_session(&peer, &workspace, &session_mcp_servers).await?
        };
        let session_id = session_setup.session_id;
        sink.emit(
            &run_id,
            lifecycle(LifecycleStatus::SessionCreated, session_id.clone()),
        );
        let permission_mode = request.permission_mode.unwrap_or_default();
        apply_permission_mode(
            &peer,
            &run_id,
            &session_id,
            &request.agent_id,
            permission_mode,
            &session_setup.response,
            &sink,
        )
        .await?;
        apply_run_configuration(
            &peer,
            &run_id,
            &session_id,
            request.model_id.as_deref(),
            request.context_size.unwrap_or_default(),
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
            permission_mode: Mutex::new(permission_mode),
            session_response: session_setup.response,
            session_record,
            session_store: self.session_store.clone(),
            mcp_servers: session_mcp_servers,
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

/// spawn에 적용할 env 목록(specs/008 R4). 사용자 env를 그대로 주입하되, PATH는
/// `사용자 PATH:보강 PATH`로 결합해 npx/node 탐색이 깨지지 않게 한다.
fn spawn_env_vars(
    agent_env: Option<&std::collections::BTreeMap<String, String>>,
    enriched: &str,
) -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> = Vec::new();
    let mut path_value = enriched.to_string();

    if let Some(env) = agent_env {
        for (key, value) in env {
            if key == "PATH" {
                if !value.trim().is_empty() {
                    path_value = format!("{value}:{enriched}");
                }
            } else {
                vars.push((key.clone(), value.clone()));
            }
        }
    }

    vars.push(("PATH".to_string(), path_value));
    vars
}

fn spawn_agent_error_context(agent_command: &str, program: &str) -> String {
    format!("failed to spawn ACP agent command `{agent_command}` using program `{program}`")
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

            run_prompt_sequence(
                sink.clone(),
                &run_id,
                initial_goal,
                ralph_loop,
                |sink, prompt| {
                    let session = session.clone();
                    async move { session.send_prompt(sink, prompt).await }
                },
            )
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
    mut send_prompt: impl FnMut(PermissionTrackingSink<S>, String) -> Fut,
) where
    S: RunEventSink,
    Fut: Future<Output = Result<String>>,
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
            RunEvent::RalphLoop {
                iteration,
                max_iterations: settings.max_iterations,
                status: RalphLoopStatus::Started,
            },
        );
        match send_loop_prompt(sink.clone(), run_id, prompt.clone(), &mut send_prompt).await {
            Ok(outcome) => {
                sink.emit(
                    run_id,
                    RunEvent::RalphLoop {
                        iteration,
                        max_iterations: settings.max_iterations,
                        status: RalphLoopStatus::Completed,
                    },
                );
                if settings.stop_on_permission && outcome.permission_requested {
                    sink.emit(
                        run_id,
                        RunEvent::Diagnostic {
                            message: format!(
                                "Ralph loop stopped after iteration {iteration}: permission was requested"
                            ),
                        },
                    );
                    return;
                }
            }
            Err(_) => {
                sink.emit(
                    run_id,
                    RunEvent::RalphLoop {
                        iteration,
                        max_iterations: settings.max_iterations,
                        status: RalphLoopStatus::Failed,
                    },
                );
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
    }

    sink.emit(
        run_id,
        RunEvent::RalphLoop {
            iteration: settings.max_iterations,
            max_iterations: settings.max_iterations,
            status: RalphLoopStatus::Stopped,
        },
    );
}

async fn send_loop_prompt<S, Fut>(
    sink: S,
    run_id: &str,
    prompt: String,
    send_prompt: &mut impl FnMut(PermissionTrackingSink<S>, String) -> Fut,
) -> Result<PromptDispatchOutcome>
where
    S: RunEventSink,
    Fut: Future<Output = Result<String>>,
{
    let permission_requested = Arc::new(AtomicBool::new(false));
    let tracking_sink = PermissionTrackingSink {
        inner: sink.clone(),
        permission_requested: permission_requested.clone(),
    };

    send_prompt(tracking_sink.clone(), prompt)
        .await
        .map_err(|err| {
            tracking_sink.emit(
                run_id,
                RunEvent::Error {
                    message: err.to_string(),
                },
            );
            err
        })?;

    Ok(PromptDispatchOutcome {
        permission_requested: permission_requested.load(Ordering::SeqCst),
    })
}

struct PromptDispatchOutcome {
    permission_requested: bool,
}

#[derive(Clone)]
struct PermissionTrackingSink<S> {
    inner: S,
    permission_requested: Arc<AtomicBool>,
}

impl<S> RunEventSink for PermissionTrackingSink<S>
where
    S: RunEventSink,
{
    fn emit(&self, run_id: &str, event: RunEvent) {
        if matches!(event, RunEvent::Permission { .. }) {
            self.permission_requested.store(true, Ordering::SeqCst);
        }
        self.inner.emit(run_id, event);
    }
}

pub struct AcpSession {
    pub run_id: String,
    pub peer: RpcPeer,
    session_id: Mutex<String>,
    workspace: PathBuf,
    resume_policy: ResumePolicy,
    permission_mode: Mutex<PermissionMode>,
    session_response: Value,
    session_record: AcpSessionRecord,
    session_store: Arc<dyn AcpSessionStore>,
    mcp_servers: Vec<AgentMcpServerConfig>,
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
                    let new_session =
                        create_agent_session(&self.peer, &self.workspace, &self.mcp_servers)
                            .await?;
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
                        *self.permission_mode.lock().await,
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

    async fn set_permission_mode<S>(&self, sink: S, mode: PermissionMode) -> Result<()>
    where
        S: RunEventSink,
    {
        sink.emit(
            &self.run_id,
            RunEvent::Diagnostic {
                message: format!("permission mode change requested: {mode:?}"),
            },
        );
        let session_id = self.session_id().await;
        apply_permission_mode(
            &self.peer,
            &self.run_id,
            &session_id,
            &self.session_record.agent_id,
            mode,
            &self.session_response,
            &sink,
        )
        .await?;
        *self.permission_mode.lock().await = mode;
        Ok(())
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

async fn create_agent_session(
    peer: &RpcPeer,
    workspace: &PathBuf,
    mcp_servers: &[AgentMcpServerConfig],
) -> Result<AcpCreatedSession> {
    let params = new_session_params(workspace, mcp_servers)?;
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

fn new_session_params(workspace: &PathBuf, mcp_servers: &[AgentMcpServerConfig]) -> Result<Value> {
    let mut params = serde_json::to_value(NewSessionRequest::new(workspace.clone()))?;
    let Value::Object(ref mut object) = params else {
        bail!("session/new params must serialize to an object");
    };
    object.insert("mcpServers".to_string(), serde_json::to_value(mcp_servers)?);
    Ok(params)
}

async fn apply_run_configuration<S>(
    peer: &RpcPeer,
    run_id: &str,
    session_id: &str,
    model_id: Option<&str>,
    context_size: ContextSizePreset,
    session_response: &Value,
    sink: &S,
) -> Result<()>
where
    S: RunEventSink,
{
    let model_id = model_id
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "providerDefault");
    if let Some(model_id) = model_id {
        apply_session_config_option(
            peer,
            run_id,
            session_id,
            session_response,
            "model",
            &["model", "modelId"],
            &[json!(model_id)],
            sink,
        )
        .await?;
    }

    if let Some((label, candidates)) = context_size_candidates(context_size) {
        apply_session_config_option(
            peer,
            run_id,
            session_id,
            session_response,
            "context size",
            &[
                "context",
                "contextSize",
                "contextWindow",
                "contextWindowTokens",
                "maxContextTokens",
            ],
            &candidates,
            sink,
        )
        .await?;
        sink.emit(
            run_id,
            RunEvent::Diagnostic {
                message: format!("requested context size preset {label}"),
            },
        );
    }

    Ok(())
}

async fn apply_session_config_option<S>(
    peer: &RpcPeer,
    run_id: &str,
    session_id: &str,
    session_response: &Value,
    label: &str,
    config_ids: &[&str],
    candidates: &[Value],
    sink: &S,
) -> Result<()>
where
    S: RunEventSink,
{
    for config_id in config_ids {
        if let Some(value) = session_config_option_value(session_response, config_id, candidates) {
            peer.request(
                "session/set_config_option",
                json!({
                    "sessionId": session_id,
                    "configId": config_id,
                    "value": value,
                }),
            )
            .await
            .map_err(rpc_to_anyhow)?;

            sink.emit(
                run_id,
                RunEvent::Diagnostic {
                    message: format!("{label} applied via config option {config_id}"),
                },
            );
            return Ok(());
        }
    }

    sink.emit(
        run_id,
        RunEvent::Diagnostic {
            message: format!(
                "{label} is not advertised by the agent; continuing with the agent default"
            ),
        },
    );
    Ok(())
}

fn context_size_candidates(context_size: ContextSizePreset) -> Option<(&'static str, Vec<Value>)> {
    match context_size {
        ContextSizePreset::Default => None,
        ContextSizePreset::Medium => Some((
            "medium",
            vec![
                json!("medium"),
                json!("64k"),
                json!("64000"),
                json!(64000),
                json!(65536),
            ],
        )),
        ContextSizePreset::Large => Some((
            "large",
            vec![
                json!("large"),
                json!("128k"),
                json!("128000"),
                json!(128000),
                json!(131072),
            ],
        )),
        ContextSizePreset::XLarge => Some((
            "xLarge",
            vec![
                json!("xLarge"),
                json!("xlarge"),
                json!("200k"),
                json!("200000"),
                json!(200000),
                json!("256k"),
                json!("262144"),
                json!(262144),
            ],
        )),
    }
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
    session_config_option_value(response, config_id, &[json!(value)]).is_some()
}

fn session_config_option_value(
    response: &Value,
    config_id: &str,
    candidates: &[Value],
) -> Option<Value> {
    let options = response
        .get("configOptions")
        .or_else(|| response.get("config_options"))
        .and_then(Value::as_array)?;

    for option in options {
        if option.get("id").and_then(Value::as_str) != Some(config_id) {
            continue;
        }

        let Some(values) = option.get("options").and_then(Value::as_array) else {
            continue;
        };

        for candidate in candidates {
            if let Some(option_value) = values
                .iter()
                .find(|option_value| config_option_value_matches(option_value, candidate))
            {
                return Some(advertised_config_value(option_value).clone());
            }
        }
    }

    None
}

fn config_option_value_matches(option_value: &Value, candidate: &Value) -> bool {
    let advertised = advertised_config_value(option_value);
    advertised == candidate
        || advertised.as_str() == candidate.as_str()
        || advertised
            .as_i64()
            .zip(candidate.as_i64())
            .is_some_and(|(advertised, candidate)| advertised == candidate)
        || advertised
            .as_str()
            .zip(candidate.as_i64())
            .is_some_and(|(advertised, candidate)| advertised == candidate.to_string())
        || advertised
            .as_i64()
            .zip(candidate.as_str())
            .is_some_and(|(advertised, candidate)| advertised.to_string() == candidate)
}

fn advertised_config_value(option_value: &Value) -> &Value {
    option_value.get("value").unwrap_or(option_value)
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

/// 기존 세션을 ACP `session/load`로 agent에 로드한다. agent가 세션을 메모리에
/// 올려야 이후 `session/prompt`가 동작한다. 호출하지 않으면 agent는 세션을
/// 모르는 상태라 `-32002 Resource not found`로 응답한다.
async fn load_agent_session(
    peer: &RpcPeer,
    session_id: &str,
    workspace: &PathBuf,
) -> Result<Value> {
    let params = serde_json::to_value(LoadSessionRequest::new(
        SessionId::new(session_id),
        workspace.clone(),
    ))?;
    let response = peer
        .request("session/load", params)
        .await
        .map_err(rpc_to_anyhow)?;
    Ok(response)
}

/// resume 요청을 처리한다. agent가 `loadSession` capability를 광고하면
/// `session/load`로 재개하고, 그렇지 않거나 로드가 실패하면 정책에 따라
/// 새 세션으로 폴백(ResumeIfAvailable)하거나 에러를 반환(ResumeRequired)한다.
async fn resume_or_create_session<S>(
    peer: &RpcPeer,
    workspace: &PathBuf,
    session_id: &str,
    load_supported: bool,
    resume_policy: ResumePolicy,
    mcp_servers: &[AgentMcpServerConfig],
    run_id: &str,
    sink: &S,
) -> Result<AcpCreatedSession>
where
    S: RunEventSink,
{
    if !load_supported {
        if !should_reissue_missing_session(resume_policy) {
            bail!("agent does not support resuming sessions (loadSession capability missing)");
        }
        sink.emit(
            run_id,
            RunEvent::Diagnostic {
                message: "agent does not support session/load; starting a new session".to_string(),
            },
        );
        return create_agent_session(peer, workspace, mcp_servers).await;
    }

    match load_agent_session(peer, session_id, workspace).await {
        Ok(response) => Ok(AcpCreatedSession {
            session_id: session_id.to_string(),
            response,
        }),
        Err(error) if should_reissue_missing_session(resume_policy) => {
            sink.emit(
                run_id,
                RunEvent::Diagnostic {
                    message: format!("resume failed ({error}); starting a new session"),
                },
            );
            create_agent_session(peer, workspace, mcp_servers).await
        }
        Err(error) => Err(error),
    }
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
    use super::{
        context_size_candidates, new_session_params, resume_session_id, run_prompt_sequence,
        session_config_option_value, should_reissue_missing_session, spawn_agent_error_context,
        spawn_env_vars,
    };
    use crate::{
        domain::{
            events::{RalphLoopStatus, RunEvent},
            run::{
                AgentMcpHttpHeader, AgentMcpServerConfig, ContextSizePreset, RalphLoopRequest,
                ResumePolicy,
            },
        },
        ports::event_sink::RunEventSink,
    };
    use anyhow::anyhow;
    use serde_json::json;
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
    fn spawn_env_vars_injects_user_env_and_combines_path_with_enriched() {
        let user_env = std::collections::BTreeMap::from([
            ("FOO".to_string(), "bar".to_string()),
            ("PATH".to_string(), "/custom/bin".to_string()),
        ]);

        let vars = spawn_env_vars(Some(&user_env), "/usr/bin:/bin");

        assert!(vars.contains(&("FOO".to_string(), "bar".to_string())));
        assert!(
            vars.contains(&("PATH".to_string(), "/custom/bin:/usr/bin:/bin".to_string())),
            "user PATH entries come first, enriched PATH is appended",
        );
    }

    #[test]
    fn spawn_env_vars_keeps_enriched_path_when_user_env_has_no_path() {
        let user_env = std::collections::BTreeMap::from([("FOO".to_string(), "bar".to_string())]);

        let vars = spawn_env_vars(Some(&user_env), "/usr/bin:/bin");

        assert!(vars.contains(&("PATH".to_string(), "/usr/bin:/bin".to_string())));

        let vars = spawn_env_vars(None, "/usr/bin:/bin");
        assert_eq!(
            vars,
            vec![("PATH".to_string(), "/usr/bin:/bin".to_string())]
        );
    }

    #[test]
    fn spawn_agent_error_context_names_command_and_program() {
        let message = spawn_agent_error_context("missing-acp --flag", "missing-acp");

        assert!(message.contains("failed to spawn ACP agent command"));
        assert!(message.contains("missing-acp --flag"));
        assert!(message.contains("missing-acp"));
    }

    #[test]
    fn new_session_params_include_mcp_servers() {
        let params = new_session_params(
            &std::path::PathBuf::from("/work/project"),
            &[AgentMcpServerConfig::Http {
                name: "agentic_workbench".to_string(),
                url: "http://127.0.0.1:1000/mcp".to_string(),
                headers: vec![AgentMcpHttpHeader {
                    name: "Authorization".to_string(),
                    value: "Bearer secret".to_string(),
                }],
            }],
        )
        .unwrap();

        assert_eq!(
            params["mcpServers"],
            json!([
                {
                    "type": "http",
                    "name": "agentic_workbench",
                    "url": "http://127.0.0.1:1000/mcp",
                    "headers": [
                        {
                            "name": "Authorization",
                            "value": "Bearer secret"
                        }
                    ]
                }
            ])
        );
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

    #[test]
    fn session_config_option_value_matches_advertised_string_values() {
        let response = json!({
            "configOptions": [
                {
                    "id": "model",
                    "options": [
                        { "value": "gpt-5" },
                        { "value": "gpt-5-codex" }
                    ]
                }
            ]
        });

        assert_eq!(
            session_config_option_value(&response, "model", &[json!("gpt-5-codex")]),
            Some(json!("gpt-5-codex"))
        );
    }

    #[test]
    fn session_config_option_value_matches_numeric_context_values() {
        let response = json!({
            "configOptions": [
                {
                    "id": "contextWindowTokens",
                    "options": [
                        { "value": 64000 },
                        { "value": 128000 }
                    ]
                }
            ]
        });

        let (_, candidates) =
            context_size_candidates(ContextSizePreset::Large).expect("large candidates");
        assert_eq!(
            session_config_option_value(&response, "contextWindowTokens", &candidates),
            Some(json!(128000))
        );
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
                move |_sink, prompt| {
                    let prompts = prompts.clone();
                    async move {
                        prompts.lock().unwrap().push(prompt);
                        Ok("end_turn".to_string())
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
            RunEvent::RalphLoop {
                iteration: 1,
                max_iterations: 2,
                status: RalphLoopStatus::Started,
            }
        )));
        assert!(events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::RalphLoop {
                iteration: 2,
                max_iterations: 2,
                status: RalphLoopStatus::Completed,
            }
        )));
        assert!(events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::RalphLoop {
                iteration: 2,
                max_iterations: 2,
                status: RalphLoopStatus::Stopped,
            }
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
                move |_sink, prompt| {
                    let prompts = prompts.clone();
                    let attempts = attempts.clone();
                    async move {
                        prompts.lock().unwrap().push(prompt);
                        let attempt = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                        if attempt == 2 {
                            Err(anyhow!("dispatch failed"))
                        } else {
                            Ok("end_turn".to_string())
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
            RunEvent::RalphLoop {
                iteration: 1,
                max_iterations: 3,
                status: RalphLoopStatus::Failed,
            }
        )));
        assert!(!events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::RalphLoop {
                iteration: 2,
                max_iterations: 3,
                status: RalphLoopStatus::Started,
            }
        )));
    }

    #[tokio::test]
    async fn ralph_loop_stops_when_permission_is_requested() {
        let sink = CollectingSink::default();
        let prompts = Arc::new(Mutex::new(Vec::new()));

        run_prompt_sequence(
            sink.clone(),
            "run-1",
            "initial".into(),
            Some(ralph_loop(3)),
            {
                let prompts = prompts.clone();
                move |sink, prompt| {
                    let prompts = prompts.clone();
                    async move {
                        prompts.lock().unwrap().push(prompt.clone());
                        if prompt == "continue" {
                            sink.emit(
                                "run-1",
                                RunEvent::Permission {
                                    permission_id: Some("permission-1".to_string()),
                                    title: "Write file".to_string(),
                                    input: None,
                                    options: Vec::new(),
                                    selected: None,
                                    requires_response: true,
                                },
                            );
                        }
                        Ok("end_turn".to_string())
                    }
                }
            },
        )
        .await;

        assert_eq!(prompts.lock().unwrap().as_slice(), ["initial", "continue"]);
        let events = sink.events.lock().unwrap();
        assert!(events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Diagnostic { message }
                if message == "Ralph loop stopped after iteration 1: permission was requested"
        )));
        assert!(!events.iter().any(|(_, event)| matches!(
            event,
            RunEvent::Diagnostic { message }
                if message == "Ralph loop iteration 2/3 started"
        )));
    }

    #[tokio::test]
    async fn ralph_loop_continues_after_permission_when_setting_is_disabled() {
        let sink = CollectingSink::default();
        let prompts = Arc::new(Mutex::new(Vec::new()));
        let mut settings = ralph_loop(2);
        settings.stop_on_permission = false;

        run_prompt_sequence(sink, "run-1", "initial".into(), Some(settings), {
            let prompts = prompts.clone();
            move |sink, prompt| {
                let prompts = prompts.clone();
                async move {
                    prompts.lock().unwrap().push(prompt.clone());
                    if prompt == "continue" {
                        sink.emit(
                            "run-1",
                            RunEvent::Permission {
                                permission_id: Some("permission-1".to_string()),
                                title: "Write file".to_string(),
                                input: None,
                                options: Vec::new(),
                                selected: None,
                                requires_response: true,
                            },
                        );
                    }
                    Ok("end_turn".to_string())
                }
            }
        })
        .await;

        assert_eq!(
            prompts.lock().unwrap().as_slice(),
            ["initial", "continue", "continue"]
        );
    }
}
