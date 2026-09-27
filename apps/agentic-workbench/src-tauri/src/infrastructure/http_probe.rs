//! 042 R12(debug 빌드 전용): 앱 연결 스모크의 두 증거. release 빌드에는 이 모듈·command가 없다.
//!
//! - `AW_HTTP_DIAGNOSTIC_FILE`: 기동 직후 `{baseUrl, token, expiresAt}`를 0600 파일로 쓴다. 토큰은 운영 발급기의
//!   "Origin 없음" 토큰 — **끝점이 인증 규칙대로 응답한다**는 것만 증명한다(데스크톱 연결 증거 아님).
//! - `AW_HTTP_WEBVIEW_PROBE_FILE`: 메인 창이 로드되면 `window.eval`로 probe를 넣는다. probe는 WebView 안에서 운영
//!   command(`get_workbench_connection`)의 데스크톱 토큰과 브라우저가 붙이는 실제 Origin으로 handshake·호출·표·WS를
//!   확인하고, 상태 코드·프레임 종류만 `report_http_probe`로 돌려준다(토큰·표 문자열 없음). **데스크톱 연결 증거.**

use std::{
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
    sync::atomic::AtomicBool,
};

use serde_json::Value;

use crate::infrastructure::workbench_http::WorkbenchHttp;

pub const DIAGNOSTIC_FILE_ENV: &str = "AW_HTTP_DIAGNOSTIC_FILE";
pub const PROBE_FILE_ENV: &str = "AW_HTTP_WEBVIEW_PROBE_FILE";
/// 043 T053: 앱 자신의 transport로 흐름을 확인하는 probe. 화면 번들이 `VITE_AW_DEBUG_PROBE=1`로 빌드됐을 때만 동작한다.
pub const APP_PROBE_FILE_ENV: &str = "AW_APP_TRANSPORT_PROBE_FILE";
pub const APP_PROBE_AGENT_ENV: &str = "AW_APP_PROBE_AGENT_COMMAND";
pub const APP_PROBE_CWD_ENV: &str = "AW_APP_PROBE_CWD";

static PROBE_INSTALLED: AtomicBool = AtomicBool::new(false);
static APP_PROBE_INSTALLED: AtomicBool = AtomicBool::new(false);

fn write_owner_only(path: &Path, value: &Value) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // `mode`는 새로 만들 때만 적용된다 — 이미 있던 파일도 owner-only로 맞춘다.
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(serde_json::to_vec_pretty(value).expect("json").as_slice())
}

/// 기동 직후 진단 파일.
pub fn write_diagnostic_file(http: &WorkbenchHttp) {
    let Some(path) = std::env::var_os(DIAGNOSTIC_FILE_ENV) else {
        return;
    };
    let value = match &http.state {
        Some(state) => serde_json::to_value(state.diagnostic_connection()).expect("json"),
        None => serde_json::json!({ "error": http.start_error }),
    };
    if let Err(error) = write_owner_only(Path::new(&path), &value) {
        eprintln!("[workbench-http] failed to write the diagnostic file: {error}");
    }
}

/// 메인 창 로드 완료 때 한 번 probe를 넣는다.
pub fn install_probe(webview: &tauri::Webview, finished: bool) {
    use std::sync::atomic::Ordering;
    if finished
        && webview.label() == "main"
        && std::env::var_os(APP_PROBE_FILE_ENV).is_some()
        && !APP_PROBE_INSTALLED.swap(true, Ordering::AcqRel)
    {
        let agent = std::env::var(APP_PROBE_AGENT_ENV).unwrap_or_default();
        let cwd = std::env::var(APP_PROBE_CWD_ENV).unwrap_or_default();
        let script = APP_PROBE_SCRIPT
            .replace("__AGENT__", &serde_json::to_string(&agent).expect("json"))
            .replace("__CWD__", &serde_json::to_string(&cwd).expect("json"));
        if let Err(error) = webview.eval(&script) {
            eprintln!("[workbench-http] failed to inject the app transport probe: {error}");
        }
    }
    if !finished || webview.label() != "main" || std::env::var_os(PROBE_FILE_ENV).is_none() {
        return;
    }
    if PROBE_INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }
    if let Err(error) = webview.eval(PROBE_SCRIPT) {
        eprintln!("[workbench-http] failed to inject the WebView probe: {error}");
    }
}

/// probe 결과를 파일에 쓴다(debug 전용 command).
#[tauri::command]
pub fn report_http_probe(report: Value) -> Result<(), String> {
    let path = std::env::var_os(PROBE_FILE_ENV).ok_or("probe is not enabled")?;
    write_owner_only(Path::new(&path), &report).map_err(|error| error.to_string())
}

/// 앱 probe 결과를 파일에 쓴다(debug 전용 command).
#[tauri::command]
pub fn report_app_probe(report: Value) -> Result<(), String> {
    let path = std::env::var_os(APP_PROBE_FILE_ENV).ok_or("app probe is not enabled")?;
    write_owner_only(Path::new(&path), &report).map_err(|error| error.to_string())
}

/// 043 T053: 화면의 `window.__awDebug`(앱 transport)로 프로젝트 조회 → run 시작·이벤트 수신 → 이벤트 소켓 강제 끊기 →
/// 끊긴 뒤 보낸 prompt의 이벤트를 이어 받는지(순번 증가·중복 없음)를 확인하고, 결과만 보고한다(토큰 없음).
const APP_PROBE_SCRIPT: &str = r#"
(async () => {
  const report = { origin: location.origin, steps: {} };
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const finish = async () => { try { await invoke('report_app_probe', { report }); } catch (error) { console.error(error); } };
  const pause = () => new Promise((resolve) => setTimeout(resolve, 50));
  const waitFor = async (condition, label, limit = 20000) => {
    const started = Date.now();
    while (!(await condition())) {
      if (Date.now() - started > limit) { throw new Error('timeout: ' + label); }
      await pause();
    }
  };
  try {
    await waitFor(() => Boolean(window.__awDebug), 'debug handle');
    const debug = window.__awDebug;
    report.transport = debug.transportKind();
    report.connection = debug.connectionState();
    const projects = await debug.invoke('list_projects');
    report.steps.listProjects = Array.isArray(projects) ? 'ok' : 'unexpected';
    if (report.transport !== 'http') { report.result = 'compat'; await finish(); return; }
    const nonce = crypto.randomUUID().slice(0, 8);
    const runId = 'probe-' + nonce;
    const events = [];
    window.__awProbeEvents = events;
    await debug.listen('agent-run-event', (payload) => {
      if (payload.runId !== runId) { return; }
      const event = payload.event || {};
      events.push({ sequence: payload.sequence, type: event.type, status: event.status, text: event.text });
    });
    // runner가 목표 앞에 안내문을 붙이므로 에코는 `echo:`로 시작하고 고유 문자열로 끝나는 agent 메시지로 판정한다.
    const isEcho = (event, text) => event.type === 'agentMessage' && typeof event.text === 'string'
      && event.text.startsWith('echo:') && event.text.endsWith(text);
    const echoIndex = (text) => events.findIndex((event) => isEcho(event, text));
    const completedAfter = (index) => index >= 0 && events.slice(index + 1).some((event) => event.type === 'lifecycle' && event.status === 'promptCompleted');
    const startGoal = 'probe-start-' + nonce;
    // 일반 패널로 시작한다(`main-agent-run`은 orchestration Main Coordinator 패널이라 작업 영역이 필요하다).
    await debug.invoke('start_agent_run', {
      request: { goal: startGoal, agentId: 'fake-acp', agentCommand: __AGENT__, cwd: __CWD__, runId, autoAllow: true },
      panelId: 'probe-panel',
    });
    // 시작 정착: 시작 prompt의 에코(agent 출력)와 그 뒤 prompt 완료를 앱 transport로 받는다.
    await waitFor(() => completedAfter(echoIndex(startGoal)), 'start prompt echo and completion');
    report.steps.startEcho = 'ok';
    const capturedBeforeDrop = events.length;
    report.steps.droppedSockets = debug.dropEventSockets();
    const afterPrompt = 'after-drop-' + nonce;
    await debug.invoke('send_prompt_to_run', { runId, prompt: afterPrompt });
    // 끊긴 뒤 보낸 고유 prompt의 에코와 그 뒤 완료를 받아야 한다(잔여 이벤트의 순번 증가로 판정하지 않는다).
    await waitFor(() => completedAfter(echoIndex(afterPrompt)), 'after-drop prompt echo and completion');
    report.steps.afterDropEcho = 'ok';
    const sequences = events.map((event) => event.sequence);
    report.steps.capturedEvents = events.length;
    report.steps.capturedBeforeDrop = capturedBeforeDrop;
    report.steps.startEchoCount = events.filter((event) => isEcho(event, startGoal)).length;
    report.steps.afterDropEchoCount = events.filter((event) => isEcho(event, afterPrompt)).length;
    report.steps.firstSequence = sequences[0];
    report.steps.lastSequence = sequences[sequences.length - 1];
    report.steps.noDuplicates = new Set(sequences).size === sequences.length;
    report.steps.noGaps = sequences.every((sequence, index) => index === 0 || sequence === sequences[index - 1] + 1);
    report.steps.eventTypes = events.map((event) => event.type + (event.status ? ':' + event.status : ''));
    report.connectionAfter = debug.connectionState();
    report.result = report.steps.droppedSockets > 0 && report.steps.noDuplicates && report.steps.noGaps
      && report.steps.startEchoCount === 1 && report.steps.afterDropEchoCount === 1 ? 'ok' : 'failed';
  } catch (error) {
    report.result = 'error';
    report.error = String(error);
    report.observed = (window.__awProbeEvents || []).slice(-20);
  }
  await finish();
})();
"#;

const PROBE_SCRIPT: &str = r#"
(async () => {
  const report = { origin: location.origin, steps: {} };
  const invoke = window.__TAURI_INTERNALS__.invoke;
  try {
    const c = await invoke('get_workbench_connection');
    report.steps.connection = 'ok';
    const auth = { authorization: 'Bearer ' + c.token, 'content-type': 'application/json' };
    let r = await fetch(c.baseUrl + '/v1/system/handshake', {
      method: 'POST', headers: auth,
      body: JSON.stringify({ supportedProtocolVersions: [1], client: { name: 'aw-probe', version: '0' } }),
    });
    const handshake = await r.json();
    report.steps.handshake = r.status;
    report.steps.protocolHeader = r.headers.get('aw-protocol-version');
    report.steps.selectedProtocolVersion = handshake.selectedProtocolVersion;
    report.instanceId = handshake.instanceId;
    r = await fetch(c.baseUrl + '/v1/calls', {
      method: 'POST', headers: auth,
      body: JSON.stringify({ protocolVersion: 1, operation: 'project.list', requestId: 'req_probe_' + Date.now(), input: {} }),
    });
    report.steps.projectList = r.status;
    r = await fetch(c.baseUrl + '/v1/event-tickets', { method: 'POST', headers: auth, body: JSON.stringify({ cursors: [] }) });
    report.steps.ticket = r.status;
    const ticket = (await r.json()).ticket;
    const connect = (value) => new Promise((resolve) => {
      let settled = false;
      const done = (result) => { if (!settled) { settled = true; resolve(result); } };
      const socket = new WebSocket(c.baseUrl.replace(/^http/, 'ws') + '/v1/events?ticket=' + encodeURIComponent(value));
      socket.onmessage = (event) => { done({ opened: true, firstFrame: JSON.parse(event.data).type }); socket.close(); };
      socket.onerror = () => done({ opened: false });
      socket.onclose = () => done({ opened: false });
      setTimeout(() => done({ opened: false, timedOut: true }), 5000);
    });
    report.steps.websocket = await connect(ticket);
    report.steps.ticketReuse = await connect(ticket);
    r = await fetch(c.baseUrl + '/v1/calls', { method: 'POST', headers: { 'content-type': 'application/json' }, body: '{}' });
    report.steps.withoutToken = r.status;
  } catch (error) {
    report.error = String(error);
  }
  await invoke('report_http_probe', { report });
})();
"#;
