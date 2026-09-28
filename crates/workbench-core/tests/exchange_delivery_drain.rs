//! 044 T037(research R7 K 경로 1, R14 표 3·3', Codex 설계 리뷰 C2·C5·E2): 교환 전달 prompt(`run.sendPrompt` +
//! `continuation`)는 이미 요청된 교환을 가리키고 서버가 조건을 확인할 때만, 교환마다 **한 번** 받는다. 비우기 중에도
//! 받고(K), 조건이 어긋나거나 이미 소비됐으면 거절한다. 같은 키 재시도는 기존 멱등 결과다. 전달은 엔진 대기열 경로라,
//! 다른 prompt가 turn을 먼저 잡아도 그 뒤에 전달된다(즉시 전송은 바쁨으로 버려질 수 있다, E2).

#![allow(clippy::result_large_err)]

mod support;

use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use serde_json::{json, Value};
use support::{
    command_request, scripted_run_engine::RunScript, uuid_key, BenchHarness, TestRuntime,
};
use workbench_core::application::{work_gate::DrainMode, workbench_runtime::RuntimeAdapters};
use workbench_protocol::{
    AuthenticatedPrincipal, FaultCode, OperationId, Outcome, Workbench, WorkbenchFault,
};

const EXCHANGE: &str = "q1";
const KEY: &str = "exchange-delivery:q1";

/// 작업대 하나에 run 둘(r1 = 보내는 쪽, r2 = 대상)과 교환 작업 영역을 만든다.
async fn prepare(h: &BenchHarness) -> String {
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    h.start(&bench, "r2").await.unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeSyncWorkspace,
        json!({"benchId": bench, "request": {
        "worktreePath": h.dir, "revision": 1, "focusedPanelId": "main",
        "panels": [
            {"panelId": "main", "title": "Main", "runId": "r1", "status": "running"},
            {"panelId": "extra", "title": "Extra", "runId": "r2", "status": "running"}
        ]}}),
    )
    .await
    .unwrap();
    bench
}

async fn send_exchange(h: &BenchHarness, bench: &str, request_id: &str, delivery: &str) {
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeSend,
        json!({"benchId": bench, "request": {
            "requestId": request_id, "sourcePanelId": "main", "sourceRunId": "r1",
            "targetPanelId": "extra", "targetRunId": "r2",
            "message": "hello peer", "delivery": delivery}}),
    )
    .await
    .unwrap();
}

async fn acknowledge(h: &BenchHarness, bench: &str, request_id: &str, outcome: &str) {
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::ExchangeAcknowledge,
        json!({"benchId": bench, "request": {
            "requestId": request_id, "targetPanelId": "extra", "outcome": outcome, "reason": null}}),
    )
    .await
    .unwrap();
}

fn delivery_input(bench: &str, run: &str, prompt: &str, request_id: &str) -> Value {
    json!({"benchId": bench, "runId": run, "prompt": prompt,
        "continuation": {"exchangeRequestId": request_id}})
}

async fn deliver(h: &BenchHarness, key: &str, input: Value) -> Result<Value, WorkbenchFault> {
    h.keyed(OperationId::RunSendPrompt, key, input).await
}

fn prompts(h: &BenchHarness) -> usize {
    h.engine.prompts.load(Ordering::SeqCst)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_valid_delivery_is_accepted_once_and_marks_the_exchange_consumed() {
    let h = BenchHarness::new(RunScript::default());
    let bench = prepare(&h).await;
    send_exchange(&h, &bench, EXCHANGE, "send").await;
    // 043: 화면은 전송 전에 확인한다. 확인(`delivered`) 뒤에도 전달 prompt는 아직 소비되지 않았다.
    acknowledge(&h, &bench, EXCHANGE, "delivered").await;
    let before = prompts(&h);

    deliver(
        &h,
        KEY,
        delivery_input(&bench, "r2", "hello peer", EXCHANGE),
    )
    .await
    .expect("a valid continuation is accepted");
    assert_eq!(prompts(&h), before + 1);
    assert!(h.rt.runtime.work_gate().exchange_consumed(&bench, EXCHANGE));

    // 같은 키·같은 입력 재시도는 기존 멱등 결과(효과 없음).
    deliver(
        &h,
        KEY,
        delivery_input(&bench, "r2", "hello peer", EXCHANGE),
    )
    .await
    .expect("a same-key retry replays");
    assert_eq!(prompts(&h), before + 1, "no second execution");

    // 같은 교환으로 다른 키·다른 내용의 둘째 prompt는 거절(1회 소비).
    let second = deliver(
        &h,
        "exchange-delivery:q1-again",
        delivery_input(&bench, "r2", "something else", EXCHANGE),
    )
    .await
    .expect_err("a consumed exchange cannot carry a second prompt");
    assert_ne!(second.code, FaultCode::Internal, "{second:?}");
    assert_eq!(second.outcome, Outcome::NotApplied, "{second:?}");
    assert_eq!(prompts(&h), before + 1, "single effect");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_deliveries_of_one_exchange_have_a_single_effect() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    let bench = prepare(&h).await;
    send_exchange(&h, &bench, EXCHANGE, "queue").await;
    let before = prompts(&h);
    let mut tasks = Vec::new();
    for n in 0..8 {
        let h = Arc::clone(&h);
        let input = delivery_input(&bench, "r2", &format!("copy {n}"), EXCHANGE);
        let key = if n == 0 {
            KEY.to_owned()
        } else {
            format!("exchange-delivery:q1-{n}")
        };
        tasks.push(tokio::spawn(async move { deliver(&h, &key, input).await }));
    }
    let mut ok = 0;
    for task in tasks {
        if task.await.unwrap().is_ok() {
            ok += 1;
        }
    }
    // 키가 exchange-delivery:q1인 것 하나만 조건을 만족한다. 나머지는 키 조건으로 거절되고, 소비는 1회다.
    assert_eq!(ok, 1, "exactly one delivery wins");
    assert_eq!(prompts(&h), before + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_key_deliveries_with_different_prompts_have_a_single_effect() {
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    let bench = prepare(&h).await;
    send_exchange(&h, &bench, EXCHANGE, "send").await;
    let before = prompts(&h);
    let mut tasks = Vec::new();
    for n in 0..8 {
        let h = Arc::clone(&h);
        let input = delivery_input(&bench, "r2", &format!("copy {n}"), EXCHANGE);
        tasks.push(tokio::spawn(async move { deliver(&h, KEY, input).await }));
    }
    let mut ok = 0;
    for task in tasks {
        if task.await.unwrap().is_ok() {
            ok += 1;
        }
    }
    assert_eq!(
        ok, 1,
        "one delivery; the others are a different payload or already consumed"
    );
    assert_eq!(prompts(&h), before + 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn each_unsatisfied_condition_is_refused_without_consuming() {
    let h = BenchHarness::new(RunScript::default());
    let bench = prepare(&h).await;
    send_exchange(&h, &bench, EXCHANGE, "send").await;
    send_exchange(&h, &bench, "q-draft", "draft").await;
    send_exchange(&h, &bench, "q-rejected", "send").await;
    acknowledge(&h, &bench, "q-rejected", "rejected").await;
    let other = h.open().await;
    let before = prompts(&h);
    let gate = Arc::clone(h.rt.runtime.work_gate());

    // 키가 exchange-delivery:<id>가 아님.
    let wrong_key = deliver(
        &h,
        "some-other-key",
        delivery_input(&bench, "r2", "hello peer", EXCHANGE),
    )
    .await
    .expect_err("the key must be exchange-delivery:<requestId>");
    assert_eq!(wrong_key.code, FaultCode::InvalidArgument, "{wrong_key:?}");
    // 대상 run이 아님(교환의 대상은 r2).
    let wrong_run = deliver(
        &h,
        KEY,
        delivery_input(&bench, "r1", "hello peer", EXCHANGE),
    )
    .await
    .expect_err("the target run must be this run");
    assert_eq!(
        wrong_run.code,
        FaultCode::PreconditionFailed,
        "{wrong_run:?}"
    );
    // 없는 교환.
    let missing = deliver(
        &h,
        "exchange-delivery:nope",
        delivery_input(&bench, "r2", "hello peer", "nope"),
    )
    .await
    .expect_err("an unknown exchange is refused");
    assert_eq!(missing.code, FaultCode::NotFound, "{missing:?}");
    // 다른 작업대에서 이 교환을 가리킴(작업대 범위 밖).
    let foreign = deliver(
        &h,
        KEY,
        delivery_input(&other, "r2", "hello peer", EXCHANGE),
    )
    .await
    .expect_err("the exchange must be in the caller's bench");
    assert!(
        matches!(foreign.code, FaultCode::NotFound | FaultCode::Forbidden),
        "{foreign:?}"
    );
    // draft 교환은 사용자가 직접 보낸다(K 아님).
    let draft = deliver(
        &h,
        "exchange-delivery:q-draft",
        delivery_input(&bench, "r2", "hello peer", "q-draft"),
    )
    .await
    .expect_err("a draft exchange is not delivered by continuation");
    assert_eq!(draft.code, FaultCode::PreconditionFailed, "{draft:?}");
    // 거절로 확인된 교환.
    let rejected = deliver(
        &h,
        "exchange-delivery:q-rejected",
        delivery_input(&bench, "r2", "hello peer", "q-rejected"),
    )
    .await
    .expect_err("a rejected exchange is not delivered");
    assert_eq!(rejected.code, FaultCode::PreconditionFailed, "{rejected:?}");
    // steer에 continuation.
    let steer = h
        .keyed(
            OperationId::RunSteer,
            &uuid_key(),
            delivery_input(&bench, "r2", "hello peer", EXCHANGE),
        )
        .await
        .expect_err("continuation belongs to run.sendPrompt only");
    assert_eq!(steer.code, FaultCode::InvalidArgument, "{steer:?}");

    assert_eq!(prompts(&h), before, "no refused call had an effect");
    for id in [EXCHANGE, "nope", "q-draft", "q-rejected"] {
        assert!(!gate.exchange_consumed(&bench, id), "{id} not consumed");
    }
    // 조건을 모두 만족하면 그대로 받는다.
    deliver(
        &h,
        KEY,
        delivery_input(&bench, "r2", "hello peer", EXCHANGE),
    )
    .await
    .expect("the valid delivery still succeeds");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn while_draining_only_a_valid_delivery_is_accepted() {
    let h = BenchHarness::new(RunScript::default());
    let bench = prepare(&h).await;
    send_exchange(&h, &bench, EXCHANGE, "send").await;
    send_exchange(&h, &bench, "q2", "send").await;
    // 비우기 전에 한 교환을 이미 소비한다.
    deliver(
        &h,
        "exchange-delivery:q2",
        delivery_input(&bench, "r2", "hello peer", "q2"),
    )
    .await
    .unwrap();
    h.rt.runtime.work_gate().begin_drain(DrainMode::Wait);
    let before = prompts(&h);

    // continuation 없는 prompt는 새 작업(N).
    let plain = h
        .keyed(
            OperationId::RunSendPrompt,
            &uuid_key(),
            json!({"benchId": bench, "runId": "r2", "prompt": "new work"}),
        )
        .await
        .expect_err("a plain prompt is new work while draining");
    assert_eq!(
        (plain.code, plain.outcome),
        (FaultCode::Draining, Outcome::NotApplied),
        "{plain:?}"
    );
    // 조건이 어긋난 continuation도 N.
    let wrong_run = deliver(
        &h,
        KEY,
        delivery_input(&bench, "r1", "hello peer", EXCHANGE),
    )
    .await
    .expect_err("an unsatisfied continuation is new work while draining");
    assert_eq!(wrong_run.code, FaultCode::Draining, "{wrong_run:?}");
    // 이미 소비된 교환의 둘째 prompt도 N.
    let consumed = deliver(
        &h,
        "exchange-delivery:q2-again",
        delivery_input(&bench, "r2", "again", "q2"),
    )
    .await
    .expect_err("a consumed exchange is new work while draining");
    assert_eq!(consumed.code, FaultCode::Draining, "{consumed:?}");
    assert_eq!(prompts(&h), before, "no refused call had an effect");

    // 조건을 만족하는 전달은 비우기 중에도 받는다(K).
    deliver(
        &h,
        KEY,
        delivery_input(&bench, "r2", "hello peer", EXCHANGE),
    )
    .await
    .expect("a valid continuation is accepted while draining");
    assert_eq!(prompts(&h), before + 1);
}

// ---------- E2: 실제 AcpRunEngine + 가짜 ACP agent ----------

fn agent_command(log: &std::path::Path, end_turn_gate: &std::path::Path) -> String {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/agents/fake_acp_permission_agent.py");
    format!(
        "python3 {} --log {} --end-turn-gate {}",
        script.display(),
        log.display(),
        end_turn_gate.display()
    )
}

fn agent_log(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn prompt_ids_with(lines: &[String], needle: &str) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix("prompt-text:"))
        .filter(|rest| rest.contains(needle))
        .map(|rest| rest.split(':').next().unwrap_or_default().to_owned())
        .collect()
}

/// 본문에 `needle`이 든 prompt가 agent에서 끝날 때까지(조건 폴링, 상한 15초).
async fn wait_finished(path: &std::path::Path, needle: &str) -> Vec<String> {
    for _ in 0..750 {
        let lines = agent_log(path);
        if prompt_ids_with(&lines, needle)
            .iter()
            .any(|id| lines.iter().any(|line| line == &format!("end_turn:{id}")))
        {
            return lines;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("agent did not finish {needle:?}: {:?}", agent_log(path));
}

/// 본문에 `needle`이 든 prompt를 agent가 받을 때까지.
async fn wait_received(path: &std::path::Path, needle: &str) {
    for _ in 0..750 {
        if !prompt_ids_with(&agent_log(path), needle).is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("agent did not receive {needle:?}: {:?}", agent_log(path));
}

async fn wait_idle(rt: &TestRuntime, run: &str) {
    for _ in 0..750 {
        if rt.runtime.work_gate().busy_run_count(run) == 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("run {run} never went idle");
}

async fn rt_call(rt: &TestRuntime, operation: OperationId, key: &str, input: Value) -> Value {
    rt.runtime
        .call(
            AuthenticatedPrincipal::desktop(),
            command_request(operation, key, input),
        )
        .await
        .unwrap_or_else(|fault| panic!("{operation:?}: {fault:?}"))
        .output()
        .cloned()
        .unwrap_or(Value::Null)
}

/// E2: 대상 run이 다른 prompt(coordinator 알림 같은 내부 prompt)로 바쁜 순간에 교환 전달이 와도, 엔진 대기열로 들어가
/// 그 turn 뒤에 agent에 전달된다. 즉시 전송 경로라면 "앞 prompt에 응답 중"으로 거절돼 오류 이벤트만 남고 버려진다.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_delivery_arriving_while_another_prompt_holds_the_turn_is_delivered_after_it() {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let log = rt.dir.path().join("agent.log");
    let gate = rt.dir.path().join("end-turn-gate");
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let bench = rt_call(
        &rt,
        OperationId::BenchOpen,
        &uuid_key(),
        json!({ "workingDirectory": work }),
    )
    .await["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::write(&gate, b"").unwrap(); // 시작 prompt는 곧바로 끝나게
    for run in ["r1", "r2"] {
        rt_call(
            &rt,
            OperationId::RunStart,
            &uuid_key(),
            json!({ "benchId": bench, "request": {
                "goal": format!("start {run}"), "agentId": "fake-acp",
                "agentCommand": agent_command(&log, &gate),
                "cwd": work, "runId": run, "autoAllow": true } }),
        )
        .await;
    }
    wait_finished(&log, "start r2").await;
    wait_idle(&rt, "r1").await;
    wait_idle(&rt, "r2").await;
    rt_call(
        &rt,
        OperationId::ExchangeSyncWorkspace,
        &uuid_key(),
        json!({"benchId": bench, "request": {
        "worktreePath": work, "revision": 1, "focusedPanelId": "main",
        "panels": [
            {"panelId": "main", "title": "Main", "runId": "r1", "status": "running"},
            {"panelId": "extra", "title": "Extra", "runId": "r2", "status": "running"}
        ]}}),
    )
    .await;
    rt_call(
        &rt,
        OperationId::ExchangeSend,
        &uuid_key(),
        json!({"benchId": bench, "request": {
            "requestId": EXCHANGE, "sourcePanelId": "main", "sourceRunId": "r1",
            "targetPanelId": "extra", "targetRunId": "r2",
            "message": "peer exchange body", "delivery": "send"}}),
    )
    .await;

    // 다른 prompt가 r2의 turn을 먼저 잡는다(문을 닫아 turn이 끝나지 않게).
    std::fs::remove_file(&gate).unwrap();
    rt_call(
        &rt,
        OperationId::RunSendPrompt,
        &uuid_key(),
        json!({"benchId": bench, "runId": "r2", "prompt": "internal notification"}),
    )
    .await;
    wait_received(&log, "internal notification").await;

    // 그 turn이 진행 중인 동안 교환 전달이 온다.
    rt_call(
        &rt,
        OperationId::RunSendPrompt,
        KEY,
        delivery_input(&bench, "r2", "peer exchange body", EXCHANGE),
    )
    .await;
    assert!(rt.runtime.work_gate().exchange_consumed(&bench, EXCHANGE));
    assert!(
        rt.runtime.work_gate().busy_run_count("r2") >= 1,
        "the queued delivery keeps the run busy"
    );

    // 문을 열면 앞 turn이 끝나고, 교환 prompt가 그 뒤에 agent에 전달돼 끝난다.
    std::fs::write(&gate, b"").unwrap();
    let lines = wait_finished(&log, "peer exchange body").await;
    assert_eq!(
        prompt_ids_with(&lines, "peer exchange body").len(),
        1,
        "delivered exactly once: {lines:?}"
    );
    let notification = prompt_ids_with(&lines, "internal notification");
    let delivery = prompt_ids_with(&lines, "peer exchange body");
    let position = |id: &str| {
        lines
            .iter()
            .position(|line| line.starts_with(&format!("prompt-text:{id}:")))
            .unwrap()
    };
    assert!(
        position(&notification[0]) < position(&delivery[0]),
        "the delivery follows the turn that held the run: {lines:?}"
    );
    wait_idle(&rt, "r2").await;
}
