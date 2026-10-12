//! gh-22: production ACP engine + real HTTP/WS, no model requests or personal files.
mod support;

use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use support::{
    command_request,
    http_harness::{Harness, TOKEN_DESKTOP, TOKEN_DESKTOP2},
    uuid_key, TestRuntime,
};
use workbench_core::application::workbench_runtime::RuntimeAdapters;
use workbench_protocol::{CallRequest, OperationId, StreamCursor, Workbench};

async fn command(h: &Harness, op: OperationId, input: Value) -> Value {
    h.call(
        Some(TOKEN_DESKTOP),
        &command_request(op, &uuid_key(), input),
    )
    .await
    .unwrap_or_else(|fault| panic!("{op}: {fault:?}"))
    .output()
    .cloned()
    .unwrap_or(Value::Null)
}
async fn replay(h: &Harness, bench: &str, run: &str) -> Value {
    h.call(
        Some(TOKEN_DESKTOP),
        &CallRequest::query(
            OperationId::RunReplay,
            json!({"benchId":bench,"runId":run,"afterSequence":0}),
        ),
    )
    .await
    .unwrap()
    .output()
    .unwrap()
    .clone()
}
async fn until(h: &Harness, bench: &str, run: &str, ready: impl Fn(&Value) -> bool) -> Value {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        let snapshot = replay(h, bench, run).await;
        if ready(&snapshot) {
            return snapshot;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "run never reached expected state: {snapshot}"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
fn events(snapshot: &Value) -> impl Iterator<Item = &Value> {
    snapshot["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| &item["event"])
}
fn status_count(snapshot: &Value, status: &str) -> usize {
    events(snapshot).filter(|e| e["status"] == status).count()
}
fn agent_command(log: &std::path::Path) -> String {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/agents/fake_acp_permission_agent.py");
    format!(
        "python3 {} --log {} --log-session --echo",
        script.display(),
        log.display()
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_idle_session_is_ready_before_the_first_user_prompt_and_retries_do_not_launch_twice() {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let h = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let log = rt.dir.path().join("agent.log");
    let bench = command(
        &h,
        OperationId::BenchOpen,
        json!({"workingDirectory": work}),
    )
    .await["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    let request = command_request(
        OperationId::RunCreateSession,
        &uuid_key(),
        json!({"benchId": bench, "request": {
            "goal":"metadata-not-prompt", "agentId":"fake-acp", "agentCommand":agent_command(&log),
            "cwd":work,"runId":"idle-1","permissionMode":"default"
        }}),
    );
    let first = h.call(Some(TOKEN_DESKTOP), &request).await.unwrap();
    let duplicate = h.call(Some(TOKEN_DESKTOP), &request).await.unwrap();
    assert_eq!(first.output(), duplicate.output());
    assert_eq!(first.output().unwrap()["id"], "idle-1");
    let ready = until(&h, &bench, "idle-1", |s| {
        status_count(s, "sessionCreated") == 1
    })
    .await;
    assert_eq!(status_count(&ready, "promptSent"), 0);
    assert!(!events(&ready).any(|e| e["type"] == "permission"));
    let log_before_send = std::fs::read_to_string(&log).unwrap();
    assert_eq!(
        log_before_send.lines().collect::<Vec<_>>(),
        vec!["initialize", "session/new"]
    );
    let denied = h
        .call(
            Some(TOKEN_DESKTOP2),
            &command_request(
                OperationId::RunSendPrompt,
                &uuid_key(),
                json!({"benchId":bench,"runId":"idle-1","prompt":"not-owner"}),
            ),
        )
        .await
        .unwrap_err();
    assert_eq!(denied.code, workbench_protocol::FaultCode::Forbidden);

    let mut ws = h
        .subscribe(
            TOKEN_DESKTOP,
            vec![StreamCursor {
                stream_id: "run:idle-1".into(),
                epoch: rt.runtime.epoch().to_owned(),
                after_sequence: ready["lastSequence"].as_u64().unwrap(),
            }],
        )
        .await;
    for (turn, text) in [(1, "first-user-message"), (2, "second-user-message")] {
        // SessionCreated guarantees registry attachment: immediately sending must succeed.
        command(
            &h,
            OperationId::RunSendPrompt,
            json!({"benchId":bench,"runId":"idle-1","prompt":text}),
        )
        .await;
        let asked = until(&h, &bench, "idle-1", |s| {
            events(s)
                .filter(|e| e["type"] == "permission" && e["requiresResponse"] == true)
                .count()
                >= turn
        })
        .await;
        let permission = events(&asked)
            .filter(|e| e["type"] == "permission" && e["requiresResponse"] == true)
            .last()
            .unwrap();
        command(&h, OperationId::RunRespondPermission, json!({"benchId":bench,"runId":"idle-1","permissionId":permission["permissionId"],"optionId":"allow"})).await;
        until(&h, &bench, "idle-1", |s| {
            status_count(s, "promptCompleted") >= turn
        })
        .await;
    }
    let mut frames = Vec::new();
    while !frames
        .iter()
        .any(|f: &Value| f.to_string().contains("echo:second-user-message"))
    {
        let frame = ws
            .next_frame(Duration::from_secs(5))
            .await
            .expect("WS streaming output");
        frames.push(serde_json::to_value(frame).unwrap());
    }
    assert!(frames
        .iter()
        .any(|f| f.to_string().contains("echo:first-user-message")));
    let log_after = std::fs::read_to_string(&log).unwrap();
    assert_eq!(
        log_after
            .lines()
            .filter(|line| line.starts_with("prompt:"))
            .count(),
        2
    );
    assert!(!log_after.contains("metadata-not-prompt"));
    assert!(!log_after.contains("not-owner"));
    command(
        &h,
        OperationId::RunCancel,
        json!({"benchId":bench,"runId":"idle-1"}),
    )
    .await;
    until(&h, &bench, "idle-1", |s| s["terminal"] == true).await;
    drop(ws);
    command(&h, OperationId::BenchClose, json!({"benchId":bench})).await;
    h.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn startup_failure_never_reports_ready_and_run_start_still_submits_its_goal() {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let h = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let bench = command(
        &h,
        OperationId::BenchOpen,
        json!({"workingDirectory": work}),
    )
    .await["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    command(&h, OperationId::RunCreateSession, json!({"benchId": bench, "request": {
        "goal":"idle", "agentId":"fake-acp", "agentCommand":"/usr/bin/false", "cwd":work,"runId":"failed-idle"
    }})).await;
    let failed = until(&h, &bench, "failed-idle", |s| {
        events(s).any(|e| e["type"] == "error")
    })
    .await;
    assert_eq!(status_count(&failed, "sessionCreated"), 0);
    assert_eq!(status_count(&failed, "promptSent"), 0);
    let invalid = h.call(Some(TOKEN_DESKTOP), &command_request(OperationId::RunCreateSession, &uuid_key(), json!({"benchId": bench, "request": {
        "goal":"idle", "agentId":"fake-acp", "ralphLoop": {"enabled":true,"maxIterations":1,"promptTemplate":"continue","stopOnError":true,"stopOnPermission":true,"delayMs":0}
    }}))).await.unwrap_err();
    assert_eq!(invalid.code, workbench_protocol::FaultCode::InvalidArgument);
    let log = rt.dir.path().join("old-start.log");
    command(&h, OperationId::RunStart, json!({"benchId": bench, "request": {
        "goal":"legacy-initial-goal", "agentId":"fake-acp", "agentCommand":agent_command(&log), "cwd":work,"runId":"legacy-start","autoAllow":true
    }})).await;
    until(&h, &bench, "legacy-start", |s| {
        status_count(s, "promptCompleted") == 1
    })
    .await;
    assert!(std::fs::read_to_string(&log)
        .unwrap()
        .contains("legacy-initial-goal"));
    command(&h, OperationId::BenchClose, json!({"benchId":bench})).await;
    h.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelling_during_session_creation_kills_the_process_and_never_becomes_ready() {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let h = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let log = rt.dir.path().join("initializing.log");
    let pid_file = rt.dir.path().join("pid");
    let gate = rt.dir.path().join("session-gate");
    let bench = command(&h, OperationId::BenchOpen, json!({"workingDirectory":work})).await
        ["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    command(&h, OperationId::RunCreateSession, json!({"benchId":bench,"request":{
        "goal":"idle", "agentId":"fake-acp", "agentCommand":format!("{} --session-gate {} --pid-file {}", agent_command(&log), gate.display(), pid_file.display()),
        "cwd":work,"runId":"cancel-before-ready"
    }})).await;
    tokio::time::timeout(Duration::from_secs(10), async {
        while !std::fs::read_to_string(&log)
            .unwrap_or_default()
            .contains("session/new")
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("agent entered session/new");
    assert_eq!(
        status_count(
            &replay(&h, &bench, "cancel-before-ready").await,
            "sessionCreated"
        ),
        0
    );
    command(
        &h,
        OperationId::RunCancel,
        json!({"benchId":bench,"runId":"cancel-before-ready"}),
    )
    .await;
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while std::process::Command::new("kill")
            .args(["-0", pid.trim()])
            .output()
            .unwrap()
            .status
            .success()
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancelled initialization process exited");
    let cancelled = until(&h, &bench, "cancel-before-ready", |s| s["terminal"] == true).await;
    assert_eq!(status_count(&cancelled, "sessionCreated"), 0);
    assert_eq!(status_count(&cancelled, "promptSent"), 0);
    command(&h, OperationId::BenchClose, json!({"benchId":bench})).await;
    h.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_idle_sessions_eof_ends_the_run_even_when_the_agent_process_remains_alive() {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let h = Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await;
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let log = rt.dir.path().join("eof.log");
    let pid_file = rt.dir.path().join("pid");
    let bench = command(&h, OperationId::BenchOpen, json!({"workingDirectory":work})).await
        ["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    command(&h, OperationId::RunCreateSession, json!({"benchId":bench,"request":{
        "goal":"idle", "agentId":"fake-acp", "agentCommand":format!("{} --close-stdout-after-session --pid-file {}", agent_command(&log), pid_file.display()),
        "cwd":work,"runId":"eof-idle"
    }})).await;
    let terminal = until(&h, &bench, "eof-idle", |s| s["terminal"] == true).await;
    assert!(events(&terminal).any(|e| e["type"] == "error"
        && e["message"]
            .as_str()
            .is_some_and(|m| m.contains("connection closed"))));
    assert_eq!(status_count(&terminal, "promptSent"), 0);
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    assert!(
        !std::process::Command::new("kill")
            .args(["-0", pid.trim()])
            .output()
            .unwrap()
            .status
            .success(),
        "EOF agent was reaped before terminal publication"
    );
    command(&h, OperationId::BenchClose, json!({"benchId":bench})).await;
    h.shutdown().await;
}
