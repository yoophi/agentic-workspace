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
/// `refresh`: SC-004d 창 새로고침 1회 전달 시나리오(새로고침마다 다시 넣는다). `quit`: 044 T035 앱 종료 전 준비 시나리오
/// (run을 띄우고 살려 둔 채 `ready-to-quit`을 보고한다, 한 번만 넣는다). 기본은 스트림·재연결 시나리오.
pub const APP_PROBE_SCENARIO_ENV: &str = "AW_APP_PROBE_SCENARIO";
/// 044 T046 `close-token`: 이 창 토큰(`{baseUrl, token, origin}`)을 넘기는 0600 비밀 파일. 보고서와 따로 둔다 — 스모크 스크립트는
/// 이 파일로 닫기 전·뒤 같은 토큰의 인증 결과(상태 코드만)를 확인한다.
pub const APP_PROBE_SECRET_FILE_ENV: &str = "AW_APP_PROBE_SECRET_FILE";

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
    // `refresh` 시나리오는 새로고침마다 다시 넣는다(단계는 화면의 sessionStorage가 이어 준다).
    let scenario = std::env::var(APP_PROBE_SCENARIO_ENV).unwrap_or_default();
    let refresh = scenario == "refresh";
    if finished
        && webview.label() == "main"
        && std::env::var_os(APP_PROBE_FILE_ENV).is_some()
        && (refresh || !APP_PROBE_INSTALLED.swap(true, Ordering::AcqRel))
    {
        let agent = std::env::var(APP_PROBE_AGENT_ENV).unwrap_or_default();
        let cwd = std::env::var(APP_PROBE_CWD_ENV).unwrap_or_default();
        let script = app_probe_template(&scenario)
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

/// 시나리오별 앱 probe 템플릿. `close-token`은 `quit` 흐름에 창 토큰 넘기기를 더한다.
fn app_probe_template(scenario: &str) -> String {
    match scenario {
        "refresh" => APP_REFRESH_PROBE_SCRIPT.to_owned(),
        "quit" => APP_QUIT_PROBE_SCRIPT.to_owned(),
        "close-token" => APP_QUIT_PROBE_SCRIPT
            .replace("scenario: 'quit'", "scenario: 'close-token'")
            .replace("    report.phase = 'ready-to-quit';\n", CLOSE_TOKEN_STEP),
        _ => APP_PROBE_SCRIPT.to_owned(),
    }
}

/// T046: 이 창의 토큰을 비밀 파일로만 넘기고, 그 토큰(+ 창 Origin)으로 handshake한 상태 코드만 보고서에 싣는다.
const CLOSE_TOKEN_STEP: &str = r#"    const c = await invoke('get_workbench_connection');
    await invoke('report_app_probe_secret', { secret: { baseUrl: c.baseUrl, token: c.token, origin: location.origin } });
    const before = await fetch(c.baseUrl + '/v1/system/handshake', {
      method: 'POST', headers: { authorization: 'Bearer ' + c.token, 'content-type': 'application/json' },
      body: JSON.stringify({ supportedProtocolVersions: [1], client: { name: 'aw-close-token-probe', version: '0' } }),
    });
    report.steps.tokenBeforeClose = before.status;
    report.phase = 'ready-to-close';
"#;

/// 창 토큰 비밀 파일(debug 전용 command, `close-token` 시나리오).
#[tauri::command]
pub fn report_app_probe_secret(secret: Value) -> Result<(), String> {
    let path =
        std::env::var_os(APP_PROBE_SECRET_FILE_ENV).ok_or("probe secret file is not enabled")?;
    write_owner_only(Path::new(&path), &secret).map_err(|error| error.to_string())
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

/// 044 T035: 앱 종료 뒤 소유자 확인(`reviews/app-smoke/owner-check.py`) 앞 단계. 앱 transport(`__awDebug`)로 에코 agent
/// run을 시작하고 시작 에코와 prompt 완료를 받은 뒤, 모드(`external`·`embedded`)·transport·작업대 id·runId와
/// `phase: "ready-to-quit"`을 보고한다. run은 취소하지 않고 살려 둔다(앱 종료 뒤 소유자가 같은 run을 이어 본다). 토큰은 싣지 않는다.
const APP_QUIT_PROBE_SCRIPT: &str = r#"
(async () => {
  const report = { origin: location.origin, scenario: 'quit', steps: {} };
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
    report.mode = await invoke('get_workbench_mode');
    report.transport = debug.transportKind();
    report.connection = debug.connectionState();
    const nonce = crypto.randomUUID().slice(0, 8);
    const runId = 'quit-' + nonce;
    report.runId = runId;
    const events = [];
    window.__awProbeEvents = events;
    await debug.listen('agent-run-event', (payload) => {
      if (payload.runId !== runId) { return; }
      const event = payload.event || {};
      events.push({ sequence: payload.sequence, type: event.type, status: event.status, text: event.text });
    });
    // runner가 목표 앞에 안내문을 붙이므로 에코는 `echo:`로 시작하고 고유 문자열로 끝나는 agent 메시지로 판정한다.
    const startGoal = 'quit-start-' + nonce;
    const echoIndex = () => events.findIndex((event) => event.type === 'agentMessage' && typeof event.text === 'string'
      && event.text.startsWith('echo:') && event.text.endsWith(startGoal));
    const completedAfter = (index) => index >= 0 && events.slice(index + 1).some((event) => event.type === 'lifecycle' && event.status === 'promptCompleted');
    await debug.invoke('start_agent_run', {
      request: { goal: startGoal, agentId: 'fake-acp', agentCommand: __AGENT__, cwd: __CWD__, runId, autoAllow: true },
      panelId: 'probe-panel',
    });
    await waitFor(() => completedAfter(echoIndex()), 'start prompt echo and completion');
    report.steps.startEcho = 'ok';
    report.steps.capturedEvents = events.length;
    report.steps.lastSequence = events.length ? events[events.length - 1].sequence : null;
    // 이 창의 작업대 id(없으면 열지 않는다) — 외부 서버 모드면 서버 작업대, 소유자가 bench.list에서 같은 id를 본다.
    report.benchId = await invoke('ensure_window_bench', { open: false, hint: null });
    report.phase = 'ready-to-quit';
    report.result = report.benchId ? 'ok' : 'failed';
  } catch (error) {
    report.result = 'error';
    report.error = String(error);
    report.observed = (window.__awProbeEvents || []).slice(-20);
  }
  await finish();
})();
"#;

/// SC-004d(043): 교환 요청을 받아 대상 run에 `exchange-delivery:<requestId>` 키로 보낸 뒤 **확인 전에 창을 새로고침**한다.
/// 새로 부팅한 창(원장 없음)은 구독 시작 재조정으로 같은 교환을 다시 받아 같은 키로 다시 보내고 확인한다. 서버 상태
/// `delivered`, 앱이 받은 run 스트림의 그 메시지 에코 1회를 보고한다(agent 기록의 수는 스모크 스크립트가 센다).
const APP_REFRESH_PROBE_SCRIPT: &str = r#"
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const STATE_KEY = 'aw-refresh-probe';
  const state = JSON.parse(sessionStorage.getItem(STATE_KEY) || 'null');
  const report = { origin: location.origin, scenario: 'refresh', phase: state ? 2 : 1, steps: {} };
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
    if (report.transport !== 'http') { throw new Error('not on the network path'); }
    const events = [];
    const isEcho = (event, runId, text) => event.runId === runId && event.type === 'agentMessage'
      && typeof event.text === 'string' && event.text.startsWith('echo:') && event.text.endsWith(text);
    const completedAfterEcho = (runId, text) => {
      const index = events.findIndex((event) => isEcho(event, runId, text));
      return index >= 0 && events.slice(index + 1).some((event) => event.runId === runId && event.type === 'lifecycle' && event.status === 'promptCompleted');
    };
    await debug.listen('agent-run-event', (payload) => {
      const event = payload.event || {};
      events.push({ runId: payload.runId, sequence: payload.sequence, type: event.type, status: event.status, text: event.text });
    });
    // 운영 코드의 교환 원장과 키(`createExchangeReconciler`, `exchangeDeliveryKey`). 라우팅은 화면 패널 대신 probe가
    // 대상 run에 교환 키로 보낸다(패널이 키를 싣는 부분은 T036 화면 시험이 근거).
    const routeTo = (runId) => (request) => {
      void debug.invoke('send_prompt_to_run', { runId, prompt: request.message }, { idempotencyKey: debug.exchangeDeliveryKey(request.requestId) });
      return { routed: true };
    };
    if (!state) {
      const nonce = crypto.randomUUID().slice(0, 8);
      const runA = 'refresh-a-' + nonce;
      const runB = 'refresh-b-' + nonce;
      for (const [runId, panelId] of [[runA, 'pa'], [runB, 'pb']]) {
        const goal = 'refresh-start-' + runId;
        await debug.invoke('start_agent_run', {
          request: { goal, agentId: 'fake-acp', agentCommand: __AGENT__, cwd: __CWD__, runId, autoAllow: true },
          panelId,
        });
        await waitFor(() => completedAfterEcho(runId, goal), 'start settled ' + runId);
      }
      await debug.invoke('sync_agent_workspace', { request: {
        worktreePath: __CWD__, revision: 1, focusedPanelId: 'pa',
        panels: [
          { panelId: 'pa', title: 'A', runId: runA, status: 'running' },
          { panelId: 'pb', title: 'B', runId: runB, status: 'running' },
        ],
      } });
      const requestId = 'refresh-x-' + nonce;
      const message = 'refresh-message-' + nonce;
      sessionStorage.setItem(STATE_KEY, JSON.stringify({ requestId, message, runB, nonce }));
      // 1단계 원장: 라우팅(교환 키로 전송)을 마치고, 확인을 보내는 순간 창을 새로고침한다(확인 전 새로고침).
      const reconciler = debug.createExchangeReconciler({
        route: routeTo(runB),
        acknowledge: async () => {
          await waitFor(() => completedAfterEcho(runB, message), 'first delivery reached the agent');
          report.steps.firstDelivery = 'ok';
          await finish();
          location.reload();
          await new Promise(() => undefined);
        },
      });
      await debug.listen('agent-exchange-requested', (request) => reconciler.handleRequested(request));
      await debug.invoke('send_agent_exchange', { request: {
        requestId, sourcePanelId: 'pa', targetPanelId: 'pb', message, delivery: 'queue',
      } });
      return;
    }
    // 2단계: 새로고침 뒤 원장이 빈 창. 구독 시작 재조정이 확인 전 교환을 요청으로 다시 넘긴다.
    const { requestId, message, runB, nonce } = state;
    let routed = 0;
    let acknowledged = 0;
    const reconciler = debug.createExchangeReconciler({
      route: (request) => { routed += 1; return routeTo(runB)(request); },
      acknowledge: async (ack) => {
        await debug.invoke('acknowledge_agent_exchange', { request: ack });
        acknowledged += 1;
      },
    });
    await debug.listen('agent-exchange-requested', (request) => reconciler.handleRequested(request));
    await debug.listen('agent-exchange-status', (exchange) => reconciler.observeStatus(exchange));
    await waitFor(async () => {
      const list = await debug.invoke('list_agent_exchanges');
      return list.some((item) => item.requestId === requestId && item.status === 'delivered');
    }, 'exchange delivered after refresh');
    // 결정적 장벽: 같은 run에 다른 고유 prompt를 보내 끝나기를 기다린다. 세션의 prompt는 차례로 처리되므로 재전송된
    // 교환이 agent에 갔다면 이 prompt 전에 도착했다.
    const barrier = 'refresh-barrier-' + nonce;
    await debug.invoke('send_prompt_to_run', { runId: runB, prompt: barrier });
    await waitFor(() => completedAfterEcho(runB, barrier), 'barrier prompt completed');
    report.steps.routedAfterRefresh = routed;
    report.steps.acknowledgedAfterRefresh = acknowledged;
    report.steps.echoCountInAppStream = events.filter((event) => isEcho(event, runB, message)).length;
    report.steps.barrier = barrier;
    report.steps.message = message;
    report.result = routed === 1 && acknowledged === 1 && report.steps.echoCountInAppStream === 1 ? 'ok' : 'failed';
    sessionStorage.removeItem(STATE_KEY);
  } catch (error) {
    report.result = 'error';
    report.error = String(error);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_probe_reports_ready_to_quit_and_leaves_the_run_alive() {
        let script = APP_QUIT_PROBE_SCRIPT
            .replace("__AGENT__", "\"agent\"")
            .replace("__CWD__", "\"/work\"");
        assert!(!script.contains("__AGENT__") && !script.contains("__CWD__"));
        assert!(script.contains("scenario: 'quit'"));
        assert!(script.contains("report.phase = 'ready-to-quit'"));
        assert!(script.contains("invoke('get_workbench_mode')"));
        assert!(script.contains("invoke('ensure_window_bench', { open: false, hint: null })"));
        // 앱 종료 뒤 소유자가 같은 run을 이어 보므로 probe는 run을 끝내지 않고 토큰을 싣지 않는다.
        for forbidden in ["cancel", "token", "get_workbench_connection"] {
            assert!(
                !script.contains(forbidden),
                "quit probe must not use {forbidden}"
            );
        }
    }

    /// T046(SC-006): `close-token`은 `quit` 흐름에 "이 창 토큰을 비밀 파일로 넘기고, 닫기 전 그 토큰의 handshake 상태
    /// 코드만 보고"를 더한다. 토큰은 보고서에 들어가지 않는다(비밀 파일 command에만 넘긴다).
    #[test]
    fn close_token_probe_hands_the_window_token_only_to_the_secret_file() {
        let script = app_probe_template("close-token")
            .replace("__AGENT__", "\"agent\"")
            .replace("__CWD__", "\"/work\"");
        assert!(script.contains("scenario: 'close-token'"));
        assert!(script.contains("report.phase = 'ready-to-close'"));
        assert!(script.contains("invoke('report_app_probe_secret', { secret: { baseUrl: c.baseUrl, token: c.token, origin: location.origin } })"));
        assert!(script.contains("report.steps.tokenBeforeClose = "));
        // 보고서에 토큰을 싣는 대입이 없다.
        for forbidden in [
            "report.token",
            "report.steps.token =",
            "report.secret",
            "{ report, token",
        ] {
            assert!(
                !script.contains(forbidden),
                "close-token report must not carry {forbidden}"
            );
        }
        assert!(
            !script.contains("cancel"),
            "the run stays alive until the window closes"
        );
        assert_eq!(app_probe_template("quit"), APP_QUIT_PROBE_SCRIPT);
    }
}
