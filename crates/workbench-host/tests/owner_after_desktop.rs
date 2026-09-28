//! 044 T034(US1, 5단계 완료 기준 (a)의 서버 쪽): 데스크톱이 떠난 뒤에도 서버가 run을 계속 소유하고, 소유자 자격
//! 증명(안내 파일에서 읽고 신원 증명을 거친 것)으로 같은 run을 조회·관찰·취소할 수 있다.
//!
//! 흐름(앱 종료 흉내, research R8·contracts/desktop-client.md §3): 소유자가 임대를 잡고 창 토큰을 발급 → 창 토큰으로 작업대를
//! 열고 run 시작 → 앱 종료처럼 창을 `closeBench:false`로 폐기하고 임대를 놓는다(작업대는 닫지 않음) → 소유자가 안내 파일로
//! 서버를 확인(`verify`: identify 증명 → handshake)한 뒤 `bench.list`에서 run을 보고, `run.replay`로 지난 출력을 받고, 이벤트
//! 구독으로 새 출력을 받고, `run.cancel`로 끝낸다. 가짜 엔진과 실제 `AcpRunEngine`(가짜 ACP agent 프로세스) 두 경우를 본다.
//! 실제 엔진에서는 취소 뒤 agent 프로세스가 끝나는 것까지 확인한다.

use std::{sync::Arc, time::Duration};

use futures_util::StreamExt;
use serde_json::{Value, json};
use workbench_core::{
    application::workbench_runtime::RuntimeAdapters,
    testing::scripted_run_engine::{RunScript, ScriptedRunEngine},
};
use workbench_host::{
    assembly::{HostAssembly, HostOptions, assemble},
    lifecycle::{
        calls::{CallError, call},
        client::verify,
        descriptor::{Descriptor, read_descriptor, write_descriptor},
        identity::OwnerIdentity,
        lock::ensure_server_dir,
    },
};

const ORIGIN: &str = "tauri://localhost";
const DEADLINE: Duration = Duration::from_secs(15);

struct Server {
    runtime: tokio::runtime::Runtime,
    host: Option<HostAssembly>,
    data_dir: std::path::PathBuf,
    dir: tempfile::TempDir,
    work: String,
}

impl Server {
    fn start(adapters: RuntimeAdapters) -> Self {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("data");
        let work = dir.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        let work = std::fs::canonicalize(work)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let identity = OwnerIdentity::generate();
        let mut options =
            HostOptions::new(data_dir.clone(), adapters, "test", runtime.handle().clone());
        options.owner = Some(identity.identity().clone());
        let host = assemble(options).expect("assembly");
        // `serve`와 같은 안내 파일을 쓴다 — 소유자 클라이언트는 이 파일만 보고 서버를 찾는다.
        let server_dir = ensure_server_dir(&std::fs::canonicalize(&data_dir).unwrap()).unwrap();
        write_descriptor(
            &server_dir,
            &Descriptor::for_endpoint(
                "server",
                &identity,
                identity.token(),
                host.runtime.epoch(),
                host.http.as_ref().unwrap().base_url(),
                "test",
            ),
        )
        .unwrap();
        Self {
            runtime,
            host: Some(host),
            data_dir,
            dir,
            work,
        }
    }

    fn descriptor(&self) -> Descriptor {
        let server_dir = std::fs::canonicalize(&self.data_dir)
            .unwrap()
            .join("workbench/server");
        read_descriptor(&server_dir).unwrap().expect("server.json")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(host) = self.host.take() {
            self.runtime.block_on(host.shutdown());
        }
    }
}

fn ok(result: Result<Value, CallError>, what: &str) -> Value {
    result.unwrap_or_else(|error| panic!("{what}: {error:?}"))
}

/// 데스크톱 쪽: 임대 → 창 토큰 → 작업대 → run 시작. 앱 종료 흉내(창 폐기 `closeBench:false` + 임대 해제)까지 한다.
fn desktop_starts_a_run_and_quits(base: &str, owner: &str, work: &str, request: Value) -> String {
    let lease = ok(
        call(
            base,
            owner,
            None,
            "lease.acquire",
            json!({ "clientKind": "desktop", "clientId": "app-1" }),
            true,
        ),
        "lease.acquire",
    );
    let window = ok(
        call(
            base,
            owner,
            None,
            "desktop.issueWindowToken",
            json!({ "label": "session-1", "incarnation": "i1", "origin": ORIGIN }),
            true,
        ),
        "issueWindowToken",
    );
    let token = window["token"].as_str().unwrap().to_owned();
    let bench = ok(
        call(
            base,
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
    ok(
        call(
            base,
            &token,
            Some(ORIGIN),
            "run.start",
            json!({ "benchId": bench, "request": request }),
            true,
        ),
        "run.start",
    );
    // 앱 종료: 창 이벤트 없이 `Exit`만 온 경우(R8 관측)에 데스크톱이 하는 일 — 창은 작업대를 닫지 않고 폐기되고 임대가 풀린다.
    let retired = ok(
        call(
            base,
            owner,
            None,
            "desktop.retireWindow",
            json!({ "label": "session-1", "incarnation": "i1", "closeBench": false }),
            true,
        ),
        "retireWindow",
    );
    assert_eq!(
        retired["closedBenches"],
        json!([]),
        "app quit must not close benches: {retired}"
    );
    ok(
        call(
            base,
            owner,
            None,
            "lease.release",
            json!({ "leaseId": lease["leaseId"] }),
            true,
        ),
        "lease.release",
    );
    // 폐기된 창 토큰으로는 더 부를 수 없다.
    match call(base, &token, Some(ORIGIN), "bench.list", json!({}), false) {
        Err(CallError::Fault { status, .. }) => assert_eq!(status, 401),
        other => panic!("a retired window token must be refused: {other:?}"),
    }
    bench
}

fn bench_runs(base: &str, owner: &str, bench: &str) -> Vec<Value> {
    let list = ok(
        call(base, owner, None, "bench.list", json!({}), false),
        "bench.list",
    );
    list.as_array()
        .unwrap()
        .iter()
        .find(|item| item["benchId"] == bench)
        .map(|item| item["runs"].as_array().cloned().unwrap_or_default())
        .unwrap_or_default()
}

fn wait_for(what: &str, mut condition: impl FnMut() -> bool) {
    let started = std::time::Instant::now();
    while !condition() {
        assert!(started.elapsed() < DEADLINE, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn texts(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter(|event| event["type"] == "agentMessage")
        .filter_map(|event| event["text"].as_str().map(str::to_owned))
        .collect()
}

/// 소유자 쪽 전 과정. `output_of(prompt)`는 그 prompt에 대한 agent 출력 문자열 판정.
fn owner_observes_and_cancels(
    server: &Server,
    bench: &str,
    start_output: impl Fn(&str) -> bool,
    prompt_output: impl Fn(&str) -> bool,
) {
    // 안내 파일만 보고 서버를 확인한다: 신원 증명 → handshake → 준비.
    let descriptor = server.descriptor();
    let verified = verify(&descriptor).expect("owner verifies the descriptor's server");
    let base = verified.base_url.clone();
    let owner = descriptor.owner_token.clone();

    // 조회: 데스크톱이 연 작업대와 그 run이 남아 있다.
    wait_for("the run to be listed", || {
        bench_runs(&base, &owner, bench)
            .iter()
            .any(|run| run["runId"] == "r1")
    });

    // 지난 출력: run.replay.
    let mut replay = Value::Null;
    wait_for("the start output in the replay", || {
        replay = ok(
            call(
                &base,
                &owner,
                None,
                "run.replay",
                json!({ "benchId": bench, "runId": "r1", "afterSequence": 0 }),
                false,
            ),
            "run.replay",
        );
        let events: Vec<Value> = replay["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["event"].clone())
            .collect();
        texts(&events).iter().any(|text| start_output(text))
    });
    let last = replay["lastSequence"].as_u64().unwrap();
    assert!(last > 0, "{replay}");

    // 새 출력: 마지막 순번 뒤부터 구독하고, 소유자가 prompt를 보내 그 출력이 구독으로 온다.
    let ticket = {
        let (status, body) = workbench_host::lifecycle::client::request(
            &base,
            "POST",
            "/v1/event-tickets",
            Some(&json!({ "cursors": [{
                "streamId": "run:r1", "epoch": verified.server_epoch, "afterSequence": last }] })),
            Some(&owner),
        )
        .expect("ticket request");
        assert_eq!(status, 200, "{body}");
        body["ticket"].as_str().unwrap().to_owned()
    };
    let ws_url = format!(
        "{}/v1/events?ticket={ticket}",
        base.replacen("http://", "ws://", 1)
    );
    let received = server.runtime.block_on(async {
        let (mut socket, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .expect("websocket");
        let hello = next_frame(&mut socket).await;
        assert_eq!(hello["type"], "hello", "{hello}");
        let base = base.clone();
        let owner = owner.clone();
        let bench = bench.to_owned();
        tokio::task::spawn_blocking(move || {
            ok(
                call(
                    &base,
                    &owner,
                    None,
                    "run.sendPrompt",
                    json!({ "benchId": bench, "runId": "r1", "prompt": "owner-after-desktop" }),
                    true,
                ),
                "owner run.sendPrompt",
            )
        })
        .await
        .unwrap();
        let mut received = Vec::new();
        let outcome = tokio::time::timeout(DEADLINE, async {
            loop {
                let frame = next_frame(&mut socket).await;
                if frame["type"] != "event" {
                    continue;
                }
                let sequence = frame["event"]["sequence"].as_u64().unwrap();
                received.push(sequence);
                let body = &frame["event"]["body"];
                if body["type"] == "agentMessage"
                    && body["text"].as_str().is_some_and(&prompt_output)
                {
                    return;
                }
            }
        })
        .await;
        assert!(
            outcome.is_ok(),
            "no live output for the owner's prompt: {received:?}"
        );
        received
    });
    assert!(
        received.iter().all(|sequence| *sequence > last),
        "live events continue after the replay: last={last} {received:?}"
    );

    // 취소: run이 끝나 목록에서 빠진다.
    ok(
        call(
            &base,
            &owner,
            None,
            "run.cancel",
            json!({ "benchId": bench, "runId": "r1" }),
            true,
        ),
        "run.cancel",
    );
    wait_for("the cancelled run to leave the bench", || {
        !bench_runs(&base, &owner, bench)
            .iter()
            .any(|run| run["runId"] == "r1")
    });
}

async fn next_frame<S>(socket: &mut S) -> Value
where
    S: futures_util::Stream<
            Item = Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        > + Unpin,
{
    loop {
        let message = tokio::time::timeout(DEADLINE, socket.next())
            .await
            .expect("frame within the deadline")
            .expect("socket open")
            .expect("frame");
        if let tokio_tungstenite::tungstenite::Message::Text(text) = message {
            return serde_json::from_str(&text).unwrap();
        }
    }
}

#[test]
fn the_owner_keeps_a_scripted_run_after_the_desktop_quits() {
    let engine = Arc::new(ScriptedRunEngine::new(RunScript::default()));
    let mut adapters = RuntimeAdapters::production();
    adapters.run_engine = Some(engine.clone());
    let server = Server::start(adapters);
    let descriptor = server.descriptor();
    let bench = desktop_starts_a_run_and_quits(
        &descriptor.base_url,
        &descriptor.owner_token,
        &server.work,
        json!({ "goal": "g", "agentId": "codex", "runId": "r1" }),
    );
    // 가짜 엔진의 시작은 출력 대신 Started를 내므로, 시작 뒤 첫 prompt의 출력으로 지난 출력을 만든다.
    {
        let base = descriptor.base_url.clone();
        ok(
            call(
                &base,
                &descriptor.owner_token,
                None,
                "run.sendPrompt",
                json!({ "benchId": bench, "runId": "r1", "prompt": "before-owner-check" }),
                true,
            ),
            "seed prompt",
        );
    }
    owner_observes_and_cancels(
        &server,
        &bench,
        |text| text == "before-owner-check",
        |text| text == "owner-after-desktop",
    );
    assert_eq!(engine.run_count(), 0, "the scripted run ended");
}

#[test]
fn the_owner_keeps_a_real_acp_run_after_the_desktop_quits_and_cancel_ends_the_agent_process() {
    let server = Server::start(RuntimeAdapters::production());
    let log = server
        .dir
        .path()
        .join(format!("agent-{}.log", uuid::Uuid::new_v4().simple()));
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../workbench-core/tests/support/agents/fake_acp_permission_agent.py");
    let command = format!(
        "python3 {} --echo --log {}",
        script.display(),
        log.display()
    );
    let descriptor = server.descriptor();
    let bench = desktop_starts_a_run_and_quits(
        &descriptor.base_url,
        &descriptor.owner_token,
        &server.work,
        json!({ "goal": "owner-start", "agentId": "fake-acp", "agentCommand": command,
                "cwd": server.work, "runId": "r1", "autoAllow": true }),
    );
    let marker = log.display().to_string();
    wait_for("the agent process of the started run", || {
        agent_alive(&marker)
    });
    owner_observes_and_cancels(
        &server,
        &bench,
        |text| text.starts_with("echo:") && text.ends_with("owner-start"),
        |text| text == "echo:owner-after-desktop",
    );
    wait_for("the agent process to exit after run.cancel", || {
        !agent_alive(&marker)
    });
}

/// 이 시험이 띄운 agent 프로세스(고유한 로그 경로가 명령줄에 있다)가 살아 있는가.
fn agent_alive(marker: &str) -> bool {
    std::process::Command::new("pgrep")
        .args(["-f", marker])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
