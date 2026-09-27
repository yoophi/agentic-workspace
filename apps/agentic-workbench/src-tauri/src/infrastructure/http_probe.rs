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

static PROBE_INSTALLED: AtomicBool = AtomicBool::new(false);

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
