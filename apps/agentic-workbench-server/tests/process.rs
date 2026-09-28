//! 044 T018(research R4·R5, contracts/server-lifecycle.md §1–§3): 서버 바이너리 프로세스 시험. 한 데이터 디렉터리에
//! 데이터를 여는 서버는 하나이고, 강제 종료 뒤 다음 시작이 복구하며, 안내 파일·잠금은 소유 사용자만 읽는다. 다른
//! 데이터 디렉터리는 따로 뜨고, 모르는 저장 형식은 데이터를 건드리지 않고 거절한다.
//!
//! 동기화는 고정 sleep이 아니라 프로세스 종료 대기와 상한 있는 polling으로 한다. 이 시험이 띄운 PID는 실패해도
//! `Cleanup`이 정확히 그 PID만 끝낸다.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

use workbench_host::lifecycle::{client::verify, descriptor::read_descriptor};

const BIN: &str = env!("CARGO_BIN_EXE_agentic-workbench-server");

/// 이 시험이 시작한 프로세스만 끝낸다(실패해도 drop에서). PID 재사용으로 남의 프로세스를 죽이지 않게(044 OCR 구현 리뷰):
/// - 직접 띄운 자식은 `waitpid(WNOHANG)`로 아직 거두지 않은 내 자식일 때만 끝낸다(이미 거뒀으면 `ECHILD`라 건너뛴다).
/// - `ensure`가 띄운 서버(내 자식 아님)는 살아 있고 명령줄이 이 시험의 서버 실행 파일 + 데이터 디렉터리일 때만 끝낸다.
#[derive(Default)]
struct Cleanup(Mutex<Vec<Tracked>>);

enum Tracked {
    Child(u32),
    Server { pid: u32, data: PathBuf },
}

impl Cleanup {
    fn track_child(&self, child: &Child) {
        self.0.lock().unwrap().push(Tracked::Child(child.id()));
    }

    fn track_server(&self, pid: u32, data: &Path) {
        self.0.lock().unwrap().push(Tracked::Server {
            pid,
            data: data.to_path_buf(),
        });
    }
}

/// 아직 거두지 않은 내 자식인지(거뒀거나 내 자식이 아니면 false, 방금 끝났으면 거두고 false).
fn unreaped_child(pid: u32) -> bool {
    let mut status = 0;
    unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) == 0 }
}

/// 살아 있고 이 시험의 서버(`serve --data-dir <data>`)인지.
fn is_test_server(pid: u32, data: &Path) -> bool {
    let Ok(output) = Command::new("ps")
        .args(["-ww", "-o", "command=", "-p", &pid.to_string()])
        .output()
    else {
        return false;
    };
    let command = String::from_utf8_lossy(&output.stdout);
    command.starts_with(BIN) && command.contains(&format!("--data-dir {}", data.display()))
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        for tracked in self.0.lock().unwrap().drain(..) {
            let pid = match tracked {
                Tracked::Child(pid) if unreaped_child(pid) => pid,
                Tracked::Server { pid, data } if alive(pid) && is_test_server(pid, &data) => pid,
                _ => continue,
            };
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }
}

fn server_dir(data: &Path) -> PathBuf {
    data.join("workbench").join("server")
}

fn serve(data: &Path) -> Child {
    Command::new(BIN)
        .args(["serve", "--data-dir"])
        .arg(data)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn serve")
}

fn ensure(data: &Path) -> (i32, String) {
    let output = Command::new(BIN)
        .args(["ensure", "--data-dir"])
        .arg(data)
        .env("AW_WORKBENCH_SERVER_PATH", BIN)
        .stdin(Stdio::null())
        .output()
        .expect("run ensure");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

fn wait_until(deadline: Duration, what: &str, mut ready: impl FnMut() -> bool) {
    let until = Instant::now() + deadline;
    while Instant::now() < until {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {what}");
}

fn alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[test]
fn only_one_of_ten_concurrent_servers_opens_the_data_dir() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    let mut children: Vec<Child> = (0..10).map(|_| serve(&data)).collect();
    for child in &children {
        cleanup.track_child(child);
    }
    let mut exits: Vec<Option<i32>> = vec![None; 10];
    wait_until(Duration::from_secs(60), "nine servers to give up", || {
        for (index, child) in children.iter_mut().enumerate() {
            if exits[index].is_none()
                && let Ok(Some(status)) = child.try_wait()
            {
                exits[index] = Some(status.code().unwrap_or(-1));
            }
        }
        exits.iter().filter(|exit| exit.is_some()).count() == 9
    });
    assert!(
        exits.iter().flatten().all(|code| *code == 3),
        "losers exit with 3: {exits:?}"
    );
    let winner = exits.iter().position(|exit| exit.is_none()).unwrap();
    wait_until(Duration::from_secs(30), "the winner's descriptor", || {
        read_descriptor(&server_dir(&data))
            .ok()
            .flatten()
            .is_some_and(|descriptor| verify(&descriptor).is_ok())
    });
    let descriptor = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    assert_eq!(
        descriptor.pid,
        children[winner].id(),
        "the running server owns the descriptor"
    );

    unsafe {
        libc::kill(children[winner].id() as i32, libc::SIGTERM);
    }
    let status = children[winner].wait().unwrap();
    assert_eq!(status.code(), Some(0), "a signalled server stops cleanly");
    assert!(
        read_descriptor(&server_dir(&data)).unwrap().is_none(),
        "the stopped server removes its descriptor"
    );
}

#[test]
fn ensure_recovers_within_five_seconds_after_the_server_is_killed() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    let (code, _) = ensure(&data);
    assert_eq!(code, 0, "first ensure starts a server");
    let first = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track_server(first.pid, &data);

    unsafe {
        libc::kill(first.pid as i32, libc::SIGKILL);
    }
    wait_until(
        Duration::from_secs(10),
        "the killed server to disappear",
        || !alive(first.pid),
    );
    assert!(
        read_descriptor(&server_dir(&data)).unwrap().is_some(),
        "a killed server leaves its descriptor behind"
    );

    let started = Instant::now();
    let (code, stdout) = ensure(&data);
    let elapsed = started.elapsed();
    assert_eq!(code, 0, "{stdout}");
    let second = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track_server(second.pid, &data);
    assert_ne!(
        second.instance_id, first.instance_id,
        "a new server instance"
    );
    assert!(verify(&second).is_ok());
    assert!(elapsed < Duration::from_secs(5), "recovered in {elapsed:?}");
    assert!(
        !stdout.contains(&second.owner_token),
        "ensure never prints the owner token"
    );
}

#[test]
fn server_files_are_owner_only() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    assert_eq!(ensure(&data).0, 0);
    let descriptor = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track_server(descriptor.pid, &data);
    let mode = |path: PathBuf| fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(server_dir(&data)), 0o700);
    for file in ["server.json", "owner.lock", "startup.lock"] {
        assert_eq!(mode(server_dir(&data).join(file)), 0o600, "{file}");
    }
    assert_eq!(
        mode(server_dir(&data).join("server.log")),
        0o600,
        "server.log"
    );
}

#[test]
fn status_prints_the_live_server_state_and_blockers() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    assert_eq!(ensure(&data).0, 0);
    let descriptor = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track_server(descriptor.pid, &data);
    let output = Command::new(BIN)
        .args(["status", "--data-dir"])
        .arg(&data)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(body["state"], "serving", "{body}");
    assert!(body["activeWork"].is_object(), "{body}");
    assert_eq!(body["instanceId"], descriptor.instance_id, "{body}");
}

#[test]
fn serve_forces_a_preexisting_custom_log_to_owner_only() {
    let cleanup = Cleanup::default();
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    let log = data.join("server.log");
    fs::write(&log, b"old\n").unwrap();
    fs::set_permissions(&log, fs::Permissions::from_mode(0o666)).unwrap();
    let (mut child, _descriptor) = start_server(&cleanup, &data, &[]);
    assert_eq!(
        fs::metadata(&log).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(stop_cli(&data, &[]).0, 0);
    assert_eq!(wait_exit(&mut child, "server with custom log to stop"), 0);
}

#[test]
fn separate_data_dirs_get_separate_servers() {
    let cleanup = Cleanup::default();
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    let one = fs::canonicalize(one.path()).unwrap();
    let two = fs::canonicalize(two.path()).unwrap();
    assert_eq!(ensure(&one).0, 0);
    assert_eq!(ensure(&two).0, 0);
    let first = read_descriptor(&server_dir(&one)).unwrap().unwrap();
    let second = read_descriptor(&server_dir(&two)).unwrap().unwrap();
    cleanup.track_server(first.pid, &one);
    cleanup.track_server(second.pid, &two);
    assert_ne!(first.instance_id, second.instance_id);
    assert_ne!(first.base_url, second.base_url);
    assert!(verify(&first).is_ok() && verify(&second).is_ok());
}

#[test]
fn an_unknown_storage_schema_is_refused_without_touching_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let data = fs::canonicalize(dir.path()).unwrap();
    let ledger = data.join("workbench").join("ledger.sqlite");
    fs::create_dir_all(ledger.parent().unwrap()).unwrap();
    {
        let conn = rusqlite::Connection::open(&ledger).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_version (version, applied_at) VALUES (99, '2026-09-28T00:00:00Z');",
        )
        .unwrap();
    }
    let before = fs::read(&ledger).unwrap();
    let output = Command::new(BIN)
        .args(["serve", "--data-dir"])
        .arg(&data)
        .stdin(Stdio::null())
        .output()
        .expect("run serve");
    assert_eq!(
        output.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(&ledger).unwrap(),
        before,
        "the ledger is untouched"
    );
    assert!(read_descriptor(&server_dir(&data)).unwrap().is_none());
}

// --- 044 T044(research R7·R9·R10, contracts/server-lifecycle.md §1·§5·§6): 유휴 정지·정지 세 방식·비우기 입구 ---
//
// 활동 작업은 실제 turn이다: 가짜 ACP agent(`--end-turn-gate`)가 승인된 첫 prompt를 gate 파일이 생길 때까지 끝내지 않는다.
// "상한 안에 멈추지 않음"은 동기화가 아니라 단정이다(상한 동안 서버가 살아 있고 안내 파일이 남아 있어야 한다).

use serde_json::{Value, json};
use workbench_host::lifecycle::{calls::call, descriptor::Descriptor};

/// 이만큼 기다려도 멈추지 않으면 "멈추지 않는다"로 본다(`--idle-timeout 1`의 몇 배).
const NOT_STOPPING_FOR: Duration = Duration::from_secs(3);
/// 기동 뒤 준비 작업(작업대 열기·run 시작·상태 조회)을 해야 하는 유휴 시험의 유휴 시간(초). 1초면 부하 아래 준비가 끝나기
/// 전에 유휴 정지가 올 수 있다(044 OCR 구현 리뷰). "멈추지 않음" 구간은 이 값의 두 배로 잡아 단정의 뜻을 지킨다.
const SETUP_IDLE_TIMEOUT_SECS: u64 = 3;
const STOP_DEADLINE: Duration = Duration::from_secs(30);

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn serve_with(data: &Path, extra: &[&str]) -> Child {
    Command::new(BIN)
        .args(["serve", "--data-dir"])
        .arg(data)
        .args(extra)
        .arg("--log")
        .arg(data.join("server.log"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn serve")
}

/// 서버를 띄우고 준비(안내 파일 + 신원 확인)까지 기다린다.
fn start_server(cleanup: &Cleanup, data: &Path, extra: &[&str]) -> (Child, Descriptor) {
    let child = serve_with(data, extra);
    cleanup.track_child(&child);
    let mut ready = None;
    wait_until(STOP_DEADLINE, "the server to be ready", || {
        ready = read_descriptor(&server_dir(data))
            .ok()
            .flatten()
            .filter(|descriptor| descriptor.pid == child.id() && verify(descriptor).is_ok());
        ready.is_some()
    });
    (child, ready.unwrap())
}

fn stop_cli(data: &Path, mode: &[&str]) -> (i32, String) {
    let output = Command::new(BIN)
        .args(["stop", "--data-dir"])
        .arg(data)
        .args(mode)
        .stdin(Stdio::null())
        .output()
        .expect("run stop");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

fn owner(descriptor: &Descriptor, operation: &str, input: Value, command: bool) -> Value {
    call(
        &descriptor.base_url,
        &descriptor.owner_token,
        None,
        operation,
        input,
        command,
    )
    .unwrap_or_else(|error| panic!("{operation}: {error:?}"))
}

fn status(descriptor: &Descriptor) -> Value {
    owner(descriptor, "server.status", json!({}), false)
}

/// 프로세스가 스스로 끝날 때까지(상한) 기다리고 종료 코드를 돌려준다.
fn wait_exit(child: &mut Child, what: &str) -> i32 {
    let mut code = None;
    wait_until(STOP_DEADLINE, what, || {
        code = child
            .try_wait()
            .unwrap()
            .map(|status| status.code().unwrap_or(-1));
        code.is_some()
    });
    code.unwrap()
}

/// 상한 동안 서버가 살아 있고 안내 파일이 남아 있음을 단정한다.
fn assert_keeps_serving(child: &mut Child, data: &Path, why: &str) {
    assert_keeps_serving_for(child, data, why, NOT_STOPPING_FOR);
}

fn assert_keeps_serving_for(child: &mut Child, data: &Path, why: &str, window: Duration) {
    let until = Instant::now() + window;
    while Instant::now() < until {
        assert!(
            child.try_wait().unwrap().is_none(),
            "the server stopped although {why}"
        );
        assert!(
            read_descriptor(&server_dir(data)).unwrap().is_some(),
            "{why}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// 첫 turn이 `gate` 파일이 생길 때까지 끝나지 않는 run을 연다. busy run이 보일 때까지 기다린다.
fn start_gated_run(descriptor: &Descriptor, work: &Path, gate: &Path) {
    let bench = owner(
        descriptor,
        "bench.open",
        json!({ "workingDirectory": work }),
        true,
    )["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    let agent = format!(
        "python3 {} --end-turn-gate {} --log {}",
        repo_path("crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py")
            .display(),
        gate.display(),
        work.join("agent.log").display()
    );
    owner(
        descriptor,
        "run.start",
        json!({ "benchId": bench, "request": { "goal": "hold the turn", "agentId": "fake-acp",
            "agentCommand": agent, "cwd": work, "runId": "r1", "autoAllow": true } }),
        true,
    );
    wait_until(STOP_DEADLINE, "a busy run", || {
        status(descriptor)["activeWork"]["busyRuns"] == 1
    });
}

fn workspace() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(dir.path()).unwrap();
    let data = root.join("data");
    let work = root.join("work");
    fs::create_dir_all(&data).unwrap();
    fs::create_dir_all(&work).unwrap();
    (dir, data, work)
}

#[test]
fn an_idle_server_stops_after_the_idle_timeout_and_removes_its_descriptor() {
    let cleanup = Cleanup::default();
    let (_dir, data, _work) = workspace();
    let (mut child, _) = start_server(&cleanup, &data, &["--idle-timeout", "1"]);
    assert_eq!(wait_exit(&mut child, "the idle stop"), 0);
    assert!(
        read_descriptor(&server_dir(&data)).unwrap().is_none(),
        "descriptor removed"
    );
}

#[test]
fn active_work_prevents_the_idle_stop_until_it_ends() {
    let cleanup = Cleanup::default();
    let (_dir, data, work) = workspace();
    let gate = work.join("end-turn");
    let idle = SETUP_IDLE_TIMEOUT_SECS.to_string();
    let (mut child, descriptor) = start_server(&cleanup, &data, &["--idle-timeout", &idle]);
    start_gated_run(&descriptor, &work, &gate);
    assert_keeps_serving_for(
        &mut child,
        &data,
        "a turn is in progress",
        Duration::from_secs(2 * SETUP_IDLE_TIMEOUT_SECS),
    );
    fs::write(&gate, b"").unwrap();
    // turn이 끝나면 세션은 살아 있어도(쉬는 세션) 유휴로 멈추고, 멈출 때 그 세션을 취소한다.
    assert_eq!(wait_exit(&mut child, "the idle stop after the turn"), 0);
    assert!(read_descriptor(&server_dir(&data)).unwrap().is_none());
}

#[test]
fn default_stop_is_refused_with_exit_5_and_wait_stops_once_the_work_ends() {
    let cleanup = Cleanup::default();
    let (_dir, data, work) = workspace();
    let gate = work.join("end-turn");
    let (mut child, descriptor) = start_server(&cleanup, &data, &[]);
    start_gated_run(&descriptor, &work, &gate);

    let (code, stdout) = stop_cli(&data, &[]);
    assert_eq!(code, 5, "{stdout}");
    let blockers: Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(blockers["activeWork"]["busyRuns"], 1, "{blockers}");
    assert!(
        child.try_wait().unwrap().is_none(),
        "a refused default stop changes nothing"
    );
    assert_eq!(status(&descriptor)["state"], "serving");

    let mut waiting = Command::new(BIN)
        .args(["stop", "--data-dir"])
        .arg(&data)
        .arg("--wait")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    cleanup.track_child(&waiting);
    wait_until(STOP_DEADLINE, "draining", || {
        status(&descriptor)["state"] == "drainingWait"
    });
    let draining_status = Command::new(BIN)
        .args(["status", "--data-dir"])
        .arg(&data)
        .output()
        .unwrap();
    assert_eq!(
        draining_status.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&draining_status.stderr)
    );
    let draining_body: Value = serde_json::from_slice(&draining_status.stdout).unwrap();
    assert_eq!(draining_body["state"], "drainingWait", "{draining_body}");
    assert_keeps_serving(&mut child, &data, "the turn has not ended");
    assert!(waiting.try_wait().unwrap().is_none(), "stop --wait waits");
    fs::write(&gate, b"").unwrap();
    assert_eq!(wait_exit(&mut child, "the wait stop"), 0);
    assert_eq!(wait_exit(&mut waiting, "stop --wait to return"), 0);
    assert!(read_descriptor(&server_dir(&data)).unwrap().is_none());
}

#[test]
fn force_stops_a_server_with_active_work() {
    let cleanup = Cleanup::default();
    let (_dir, data, work) = workspace();
    let (mut child, descriptor) = start_server(&cleanup, &data, &[]);
    start_gated_run(&descriptor, &work, &work.join("never"));
    let (code, stdout) = stop_cli(&data, &["--force"]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(wait_exit(&mut child, "the forced stop"), 0);
    assert!(read_descriptor(&server_dir(&data)).unwrap().is_none());
}

#[test]
fn new_work_is_refused_while_draining() {
    let cleanup = Cleanup::default();
    let (_dir, data, work) = workspace();
    let gate = work.join("end-turn");
    let (mut child, descriptor) = start_server(&cleanup, &data, &[]);
    start_gated_run(&descriptor, &work, &gate);
    let drained = owner(&descriptor, "server.stop", json!({ "mode": "wait" }), true);
    assert_eq!(drained["state"], "drainingWait");
    let (ready_status, ready) = workbench_host::lifecycle::client::request(
        &descriptor.base_url,
        "GET",
        "/health/ready",
        None,
        Some(&descriptor.owner_token),
    )
    .unwrap();
    assert_eq!(ready_status, 503, "{ready}");
    assert_eq!(ready["ready"], false, "{ready}");
    assert_eq!(ready["state"], "draining", "{ready}");
    let (handshake_status, handshake) = workbench_host::lifecycle::client::request(
        &descriptor.base_url,
        "POST",
        "/v1/system/handshake",
        Some(&json!({ "supportedProtocolVersions": [1] })),
        Some(&descriptor.owner_token),
    )
    .unwrap();
    assert_eq!(handshake_status, 200, "{handshake}");
    assert_eq!(handshake["state"], "draining", "{handshake}");
    let refused = call(
        &descriptor.base_url,
        &descriptor.owner_token,
        None,
        "bench.open",
        json!({ "workingDirectory": work }),
        true,
    )
    .expect_err("N while draining");
    match refused {
        workbench_host::lifecycle::calls::CallError::Fault { status, code, .. } => {
            assert_eq!((status, code.as_str()), (503, "draining"));
        }
        other => panic!("{other:?}"),
    }
    assert!(
        owner(&descriptor, "bench.list", json!({}), false).is_array(),
        "Q still answers"
    );
    fs::write(&gate, b"").unwrap();
    assert_eq!(wait_exit(&mut child, "the wait stop"), 0);
}

/// 이전 세대에서 적용 여부를 모르게 끝난 변경 하나를 ledger에 남긴다(다음 시작 복구가 `unknown`으로 닫는다).
fn leave_interrupted_update(cleanup: &Cleanup, data: &Path) {
    let (mut child, _) = start_server(cleanup, data, &[]);
    assert_eq!(stop_cli(data, &[]).0, 0);
    assert_eq!(wait_exit(&mut child, "the first server to stop"), 0);
    let conn = rusqlite::Connection::open(data.join("workbench").join("ledger.sqlite")).unwrap();
    conn.execute(
        "INSERT INTO operation_ledger (execution_id, principal_kind, operation, contract_revision, idempotency_key,
            input_fingerprint, aggregate, reserved_resource_id, state, result_json, revision, request_id,
            created_at, updated_at, expires_at)
         VALUES ('exec-interrupted', 'desktop', 'savedPrompt.update', 1, 'interrupted', 'fp', 'saved_prompts', NULL,
            'pending', NULL, NULL, 'r-interrupted', '2026-09-28T00:00:00Z', '2026-09-28T00:00:00Z', NULL)",
        [],
    )
    .unwrap();
}

#[test]
fn a_server_with_only_unknown_ledger_records_stops_when_idle() {
    let cleanup = Cleanup::default();
    let (_dir, data, _work) = workspace();
    leave_interrupted_update(&cleanup, &data);
    let idle = SETUP_IDLE_TIMEOUT_SECS.to_string();
    let (mut child, descriptor) = start_server(&cleanup, &data, &["--idle-timeout", &idle]);
    let current = status(&descriptor);
    assert_eq!(current["unresolvedOperations"], 1, "{current}");
    assert_eq!(current["activeWork"]["pendingOperations"], 0, "{current}");
    assert_eq!(
        wait_exit(&mut child, "the idle stop with only unknown records"),
        0
    );
    assert!(read_descriptor(&server_dir(&data)).unwrap().is_none());
}

#[test]
fn a_server_with_only_unknown_ledger_records_stops_on_wait() {
    let cleanup = Cleanup::default();
    let (_dir, data, _work) = workspace();
    leave_interrupted_update(&cleanup, &data);
    let (mut child, descriptor) = start_server(&cleanup, &data, &[]);
    assert_eq!(status(&descriptor)["unresolvedOperations"], 1);
    let (code, stdout) = stop_cli(&data, &["--wait"]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        wait_exit(&mut child, "the wait stop with only unknown records"),
        0
    );
}

#[test]
fn sigterm_is_a_forced_stop_even_with_active_work() {
    let cleanup = Cleanup::default();
    let (_dir, data, work) = workspace();
    let (mut child, descriptor) = start_server(&cleanup, &data, &[]);
    start_gated_run(&descriptor, &work, &work.join("never"));
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    assert_eq!(wait_exit(&mut child, "the SIGTERM stop"), 0);
    assert!(read_descriptor(&server_dir(&data)).unwrap().is_none());
    let log = fs::read_to_string(data.join("server.log")).unwrap();
    assert!(log.contains("SIGTERM: force stop"), "{log}");
}

// --- 044 OCR 구현 리뷰(host M2, contracts/server-lifecycle.md §3): ensure의 대기 루프 ---

/// `ensure`를 자식 프로세스로 띄운다(끝날 때까지 기다리지 않음).
fn spawn_ensure(cleanup: &Cleanup, data: &Path) -> Child {
    let child = Command::new(BIN)
        .args(["ensure", "--data-dir"])
        .arg(data)
        .env("AW_WORKBENCH_SERVER_PATH", BIN)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn ensure");
    cleanup.track_child(&child);
    child
}

/// `startup.lock`을 다른 프로세스(ensure)가 쥐고 있는지.
fn startup_lock_held(data: &Path) -> bool {
    workbench_host::lifecycle::lock::startup_lock(data, Duration::ZERO).is_err()
}

/// 서버가 소유 잠금을 잡고 안내 파일을 쓰기 전에 죽은 경우(또는 아직 준비 전): ensure는 그 잠금이 풀릴 때까지 기다렸다가
/// 서버를 띄운다. 예전 루프는 잠금을 다시 시도하지 않아 상한까지 기다린 뒤 실패했다.
#[test]
fn ensure_starts_a_server_once_a_held_owner_lock_is_released() {
    let cleanup = Cleanup::default();
    let (_dir, data, _work) = workspace();
    workbench_host::lifecycle::lock::ensure_server_dir(&data).unwrap();
    let held = workbench_host::lifecycle::lock::try_owner_lock(&data)
        .unwrap()
        .expect("the test holds owner.lock");
    let mut ensuring = spawn_ensure(&cleanup, &data);
    wait_until(STOP_DEADLINE, "ensure to be waiting", || {
        startup_lock_held(&data)
    });
    assert!(
        ensuring.try_wait().unwrap().is_none(),
        "ensure waits while the owner lock is held"
    );
    drop(held);
    assert_eq!(wait_exit(&mut ensuring, "ensure after the lock is free"), 0);
    let descriptor = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track_server(descriptor.pid, &data);
    assert!(verify(&descriptor).is_ok());
}

/// 비우는 서버는 붙을 대상이 아니다: ensure는 그 서버를 돌려주지 않고, 그 서버가 끝나면 새 서버를 띄운다.
#[test]
fn ensure_does_not_attach_to_a_draining_server_and_starts_a_new_one_after_it_stops() {
    let cleanup = Cleanup::default();
    let (_dir, data, work) = workspace();
    let gate = work.join("end-turn");
    let (mut old, descriptor) = start_server(&cleanup, &data, &[]);
    start_gated_run(&descriptor, &work, &gate);
    let drained = owner(&descriptor, "server.stop", json!({ "mode": "wait" }), true);
    assert_eq!(drained["state"], "drainingWait");

    let mut ensuring = spawn_ensure(&cleanup, &data);
    // ensure가 시작 잠금을 잡았거나(대기 중) 이미 끝났을 때까지 — 끝났다면 비우는 서버를 돌려준 것이다.
    wait_until(STOP_DEADLINE, "ensure to start", || {
        startup_lock_held(&data) || ensuring.try_wait().unwrap().is_some()
    });
    let until = Instant::now() + NOT_STOPPING_FOR;
    while Instant::now() < until {
        assert!(
            ensuring.try_wait().unwrap().is_none(),
            "ensure returned a draining server"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    fs::write(&gate, b"").unwrap();
    assert_eq!(wait_exit(&mut old, "the draining server to stop"), 0);
    assert_eq!(
        wait_exit(&mut ensuring, "ensure after the old server stopped"),
        0
    );
    let fresh = read_descriptor(&server_dir(&data)).unwrap().unwrap();
    cleanup.track_server(fresh.pid, &data);
    assert_ne!(
        fresh.instance_id, descriptor.instance_id,
        "a new server instance"
    );
    assert!(verify(&fresh).is_ok());
    assert_eq!(status(&fresh)["state"], "serving");
}
