//! 044 T034·T035: 실제 서버 바이너리(`serve`)와 그 안내 파일로, 데스크톱이 떠난 뒤 소유자 스모크 클라이언트
//! `specs/044-standalone-server/reviews/app-smoke/owner-check.py`가 같은 run을 조회·관찰·취소하는지 확인한다.
//! 실제 `AcpRunEngine`과 가짜 ACP agent 프로세스(`--echo`)를 쓰고, 취소 뒤 agent 프로세스가 끝나는지도 본다.
//! 신원 증명이 틀린 끝점에는 스크립트가 소유자 토큰을 보내지 않는다(가짜 끝점이 받은 헤더로 확인).
//!
//! 동기화는 상한 있는 polling과 프로세스 종료 대기로 한다. 이 시험이 띄운 서버 PID는 실패해도 `Cleanup`이 끝낸다.

use std::{
    io::{Read, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use workbench_host::lifecycle::{
    calls::call,
    client::verify,
    descriptor::{Descriptor, read_descriptor, write_descriptor},
    identity::OwnerIdentity,
};

const BIN: &str = env!("CARGO_BIN_EXE_agentic-workbench-server");
const ORIGIN: &str = "tauri://localhost";

struct Cleanup(Vec<u32>);

impl Drop for Cleanup {
    fn drop(&mut self) {
        for pid in self.0.drain(..) {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
    }
}

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn owner_check_script() -> PathBuf {
    repo_path("specs/044-standalone-server/reviews/app-smoke/owner-check.py")
}

fn wait_until(what: &str, mut ready: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(30);
    while Instant::now() < until {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for {what}");
}

fn agent_alive(marker: &str) -> bool {
    Command::new("pgrep")
        .args(["-f", marker])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn ok(result: Result<Value, workbench_host::lifecycle::calls::CallError>, what: &str) -> Value {
    result.unwrap_or_else(|error| panic!("{what}: {error:?}"))
}

fn run_owner_check(data: &Path, run_id: &str) -> (i32, Value, String) {
    let output = Command::new("python3")
        .arg(owner_check_script())
        .args(["--data-dir"])
        .arg(data)
        .args([
            "--run-id",
            run_id,
            "--prompt",
            "owner-check-script",
            "--timeout",
            "20",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("run owner-check.py");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let report = stdout
        .lines()
        .last()
        .and_then(|line| serde_json::from_str(line).ok())
        .unwrap_or(Value::Null);
    (
        output.status.code().unwrap_or(-1),
        report,
        format!("{stdout}{}", String::from_utf8_lossy(&output.stderr)),
    )
}

#[test]
fn owner_check_script_observes_and_cancels_a_run_after_the_desktop_left() {
    let dir = tempfile::tempdir().unwrap();
    let data = std::fs::canonicalize(dir.path()).unwrap().join("data");
    std::fs::create_dir_all(&data).unwrap();
    let work = std::fs::canonicalize(dir.path()).unwrap().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let mut server: Child = Command::new(BIN)
        .args(["serve", "--data-dir"])
        .arg(&data)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn serve");
    let mut cleanup = Cleanup(vec![server.id()]);
    let server_dir = data.join("workbench/server");
    wait_until("the server's verified descriptor", || {
        read_descriptor(&server_dir)
            .ok()
            .flatten()
            .is_some_and(|descriptor| verify(&descriptor).is_ok())
    });
    let descriptor = read_descriptor(&server_dir).unwrap().unwrap();
    let base = descriptor.base_url.clone();
    let owner = descriptor.owner_token.clone();

    // 데스크톱: 임대 → 창 토큰 → 작업대 → 실제 ACP run(에코).
    let lease = ok(
        call(
            &base,
            &owner,
            None,
            "lease.acquire",
            json!({ "clientKind": "desktop", "clientId": "app" }),
            true,
        ),
        "lease.acquire",
    );
    let token = ok(
        call(
            &base,
            &owner,
            None,
            "desktop.issueWindowToken",
            json!({ "label": "session-1", "incarnation": "i1", "origin": ORIGIN }),
            true,
        ),
        "issueWindowToken",
    )["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let bench = ok(
        call(
            &base,
            &token,
            Some(ORIGIN),
            "bench.open",
            json!({ "workingDirectory": work }),
            true,
        ),
        "bench.open",
    )["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    let log = dir.path().join(format!("agent-{}.log", uuid_like()));
    let marker = log.display().to_string();
    let agent = format!(
        "python3 {} --echo --log {}",
        repo_path("crates/workbench-core/tests/support/agents/fake_acp_permission_agent.py")
            .display(),
        log.display()
    );
    ok(
        call(
            &base,
            &token,
            Some(ORIGIN),
            "run.start",
            json!({ "benchId": bench, "request": { "goal": "script-start", "agentId": "fake-acp",
                "agentCommand": agent, "cwd": work, "runId": "r1", "autoAllow": true } }),
            true,
        ),
        "run.start",
    );
    // 시작 출력이 나온 뒤(정착) 데스크톱이 떠난다.
    wait_until("the start echo", || {
        call(
            &base,
            &owner,
            None,
            "run.replay",
            json!({ "benchId": bench, "runId": "r1", "afterSequence": 0 }),
            false,
        )
        .ok()
        .and_then(|replay| replay["events"].as_array().cloned())
        .is_some_and(|events| {
            events.iter().any(|item| {
                item["event"]["type"] == "agentMessage"
                    && item["event"]["text"]
                        .as_str()
                        .is_some_and(|text| text.ends_with("script-start"))
            })
        })
    });
    ok(
        call(
            &base,
            &owner,
            None,
            "desktop.retireWindow",
            json!({ "label": "session-1", "incarnation": "i1", "closeBench": false }),
            true,
        ),
        "retireWindow",
    );
    ok(
        call(
            &base,
            &owner,
            None,
            "lease.release",
            json!({ "leaseId": lease["leaseId"] }),
            true,
        ),
        "lease.release",
    );
    assert!(
        agent_alive(&marker),
        "the agent keeps running after the desktop left"
    );

    let (code, report, output) = run_owner_check(&data, "r1");
    assert_eq!(code, 0, "owner-check.py: {output}");
    assert_eq!(report["result"], "ok", "{report}");
    assert_eq!(report["steps"]["identify"], "ok", "{report}");
    assert_eq!(report["steps"]["liveEcho"], true, "{report}");
    assert_eq!(report["steps"]["liveAfterReplay"], true, "{report}");
    assert_eq!(report["steps"]["cancelled"], true, "{report}");
    assert!(
        !output.contains(&owner),
        "the script never prints the owner token"
    );
    wait_until(
        "the agent process to exit after the script's cancel",
        || !agent_alive(&marker),
    );

    unsafe {
        libc::kill(server.id() as i32, libc::SIGTERM);
    }
    let status = server.wait().expect("server exit");
    cleanup.0.clear();
    assert_eq!(status.code(), Some(0), "serve stops on SIGTERM");
}

#[test]
fn owner_check_script_sends_no_owner_token_to_an_impostor_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let seen = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut raw = Vec::new();
            let mut buffer = [0u8; 4096];
            // 머리를 끝까지 읽고, content-length만큼 본문을 더 읽는다.
            while let Ok(read) = stream.read(&mut buffer) {
                if read == 0 {
                    break;
                }
                raw.extend_from_slice(&buffer[..read]);
                if let Some(split) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&raw[..split]).to_lowercase();
                    let length = head
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if raw.len() >= split + 4 + length {
                        break;
                    }
                }
            }
            seen.lock()
                .unwrap()
                .push(String::from_utf8_lossy(&raw).into_owned());
            let body = json!({ "instanceId": "impostor", "proof": "00" }).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().to_path_buf();
    let server_dir = data.join("workbench/server");
    std::fs::create_dir_all(&server_dir).unwrap();
    let identity = OwnerIdentity::generate();
    write_descriptor(
        &server_dir,
        &Descriptor::for_test(&identity, &format!("http://{address}")),
    )
    .unwrap();

    let (code, report, output) = run_owner_check(&data, "r1");
    assert_eq!(code, 2, "identity failure exits 2: {output}");
    assert_eq!(report["result"], "identity-failed", "{report}");
    let requests = requests.lock().unwrap();
    assert!(
        !requests.is_empty(),
        "the script asked for the identity proof"
    );
    assert!(
        requests
            .iter()
            .all(|request| !request.to_lowercase().contains("authorization")
                && !request.contains(identity.token())),
        "no owner token reached the impostor: {requests:?}"
    );
}

fn uuid_like() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    )
}
