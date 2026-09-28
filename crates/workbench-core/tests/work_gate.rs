//! 044 T010: 작업 관문(WorkGate)의 실행 수명 계약(research R14, Codex 재검토 E1). 이벤트 추정이 아니라 실행 진입점의 동기
//! 예약과 실행 future 종료의 해제로 "바쁜 run"을 센다.
//!
//! (i) 대기열 prompt만 남은 구간(앞 prompt 완료와 다음 prompt 전송 사이)에도 바쁨이 0이 되지 않는다.
//! (ii) RPC 오류로 끝난 prompt 뒤에는 예약이 남지 않는다.
//! (iii) 정지 판정과 예약이 교차해도 "멈춘 뒤 실행"이 없다.
//! (iv) 시작의 초기 prompt 순서(Ralph 반복 사이 지연 포함) 동안 바쁨이 0이 되지 않고, 순서가 끝나면 세션이 살아 있어도 0이다.
//!
//! (i)(ii)(iv)는 실제 `AcpRunEngine`과 최소 ACP agent 프로세스(`support/agents/fake_acp_permission_agent.py`)를 쓴다. agent의
//! 턴 종료 문(`--end-turn-gate`)으로 turn이 진행 중인 구간을 시험이 정한다. 기다림은 조건 폴링(상한)이다.

use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use serde_json::{json, Value};
use support::{command_request, uuid_key, TestRuntime};
use workbench_core::application::{
    work_gate::{
        AdmitRefused, DrainMode, GateEvent, GateState, LaunchCancel, LaunchState, ReservationKind,
        WorkGate,
    },
    workbench_runtime::RuntimeAdapters,
};
use workbench_protocol::{AuthenticatedPrincipal, OperationId, Workbench};

mod support;

fn agent_command(extra: &str, log: &Path) -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/support/agents/fake_acp_permission_agent.py");
    format!(
        "python3 {} --log {} {extra}",
        script.display(),
        log.display()
    )
}

fn agent_log(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// 조건 폴링(20ms 간격, 상한 10초). 상한을 넘기면 실패한다.
async fn wait_for(label: &str, mut condition: impl FnMut() -> bool) {
    for _ in 0..500 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("condition not reached: {label}");
}

fn ended(lines: &[String], needle: &str) -> bool {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix("prompt-text:"))
        .filter(|rest| rest.contains(needle))
        .map(|rest| rest.split(':').next().unwrap_or_default().to_owned())
        .any(|id| lines.iter().any(|line| line == &format!("end_turn:{id}")))
}

async fn call(rt: &TestRuntime, operation: OperationId, input: Value) -> Value {
    rt.runtime
        .call(
            AuthenticatedPrincipal::desktop(),
            command_request(operation, &uuid_key(), input),
        )
        .await
        .unwrap_or_else(|fault| panic!("{operation:?}: {fault:?}"))
        .output()
        .cloned()
        .unwrap_or(Value::Null)
}

struct Setup {
    rt: TestRuntime,
    bench: String,
    work: String,
    log: std::path::PathBuf,
    gate_file: std::path::PathBuf,
    events: Arc<Mutex<Vec<GateEvent>>>,
}

async fn setup() -> Setup {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let log = rt.dir.path().join("agent.log");
    let gate_file = rt.dir.path().join("end-turn.gate");
    let work = rt.dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let work = std::fs::canonicalize(work)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let bench = call(
        &rt,
        OperationId::BenchOpen,
        json!({ "workingDirectory": work }),
    )
    .await["benchId"]
        .as_str()
        .unwrap()
        .to_owned();
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    rt.runtime
        .work_gate()
        .set_observer(Arc::new(move |event: &GateEvent| {
            sink.lock().unwrap().push(event.clone())
        }));
    Setup {
        rt,
        bench,
        work,
        log,
        gate_file,
        events,
    }
}

/// run의 바쁨(A-turn 예약 수) 변화 기록.
fn busy_history(events: &Mutex<Vec<GateEvent>>, run: &str) -> Vec<usize> {
    events
        .lock()
        .unwrap()
        .iter()
        .filter(|event| event.run.as_deref() == Some(run) && event.kind == ReservationKind::Turn)
        .map(|event| event.run_busy_after)
        .collect()
}

/// 기록이 0보다 크게 시작하고, 0은 마지막에만 있다(중간 공백 없음).
fn assert_no_gap(history: &[usize], label: &str) {
    assert!(!history.is_empty(), "{label}: the run was never reserved");
    let last_zero_allowed = history.len() - 1;
    for (index, busy) in history.iter().enumerate() {
        if *busy == 0 {
            assert_eq!(
                index, last_zero_allowed,
                "{label}: busy dropped to 0 mid-way: {history:?}"
            );
        }
    }
    assert_eq!(
        *history.last().unwrap(),
        0,
        "{label}: still busy at the end: {history:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_queued_prompt_keeps_the_run_busy_between_turns() {
    let s = setup().await;
    let command = agent_command(
        &format!("--end-turn-gate {}", s.gate_file.display()),
        &s.log,
    );
    std::fs::write(&s.gate_file, "").unwrap(); // 시작 목표 turn은 곧바로 끝낸다
    call(&s.rt, OperationId::RunStart, json!({ "benchId": s.bench, "request": {
        "goal": "first", "agentId": "fake-acp", "agentCommand": command, "cwd": s.work, "runId": "r1", "autoAllow": true } })).await;
    wait_for("start goal ended", || ended(&agent_log(&s.log), "first")).await;
    // agent 기록의 end_turn은 응답 전이다: 엔진이 초기 순서를 실제로 끝내야(A-turn 해제) 다음 send_prompt가 거절되지 않는다.
    wait_for("initial sequence released", || {
        s.rt.runtime.work_gate().busy_run_count("r1") == 0
    })
    .await;
    std::fs::remove_file(&s.gate_file).unwrap(); // 이제부터 turn은 문이 열릴 때까지 진행 중
    s.events.lock().unwrap().clear();

    call(
        &s.rt,
        OperationId::RunSendPrompt,
        json!({ "benchId": s.bench, "runId": "r1", "prompt": "p1" }),
    )
    .await;
    wait_for("p1 reached the agent", || {
        agent_log(&s.log).iter().any(|l| l.contains("\"p1\""))
    })
    .await;
    let sink = s.rt.runtime.benches().run_sink(&s.bench);
    s.rt.runtime
        .run_engine()
        .queue_prompt("r1", "p2".into(), sink)
        .await
        .expect("queue p2");
    // p1 turn 진행 중 + p2 대기열.
    assert!(
        s.rt.runtime.work_gate().busy_run_count("r1") >= 1,
        "busy while p1 runs and p2 waits"
    );
    std::fs::write(&s.gate_file, "").unwrap(); // p1 끝 → 엔진이 p2를 보냄 → p2 끝
    wait_for("p2 ended", || ended(&agent_log(&s.log), "p2")).await;
    wait_for("reservations released", || {
        s.rt.runtime.work_gate().busy_run_count("r1") == 0
    })
    .await;
    assert_no_gap(&busy_history(&s.events, "r1"), "p1 → queued p2");
    s.rt.runtime.close_all_benches().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_prompt_that_fails_with_an_rpc_error_releases_its_reservation() {
    let s = setup().await;
    let command = agent_command("--rpc-error-text rpc-boom", &s.log);
    call(&s.rt, OperationId::RunStart, json!({ "benchId": s.bench, "request": {
        "goal": "first", "agentId": "fake-acp", "agentCommand": command, "cwd": s.work, "runId": "r1", "autoAllow": true } })).await;
    wait_for("start goal ended", || ended(&agent_log(&s.log), "first")).await;
    // agent 기록의 end_turn은 응답 전이다: 엔진이 초기 순서를 실제로 끝내야(A-turn 해제) 다음 send_prompt가 거절되지 않는다.
    wait_for("initial sequence released", || {
        s.rt.runtime.work_gate().busy_run_count("r1") == 0
    })
    .await;
    s.events.lock().unwrap().clear();

    call(
        &s.rt,
        OperationId::RunSendPrompt,
        json!({ "benchId": s.bench, "runId": "r1", "prompt": "rpc-boom" }),
    )
    .await;
    wait_for("agent answered with an rpc error", || {
        agent_log(&s.log)
            .iter()
            .any(|l| l.starts_with("rpc-error:"))
    })
    .await;
    wait_for("reservation released after the rpc error", || {
        s.rt.runtime.work_gate().busy_run_count("r1") == 0
    })
    .await;
    let history = busy_history(&s.events, "r1");
    assert_no_gap(&history, "rpc error prompt");
    assert!(
        history.contains(&1),
        "the failing prompt was reserved: {history:?}"
    );
    s.rt.runtime.close_all_benches().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_initial_prompt_sequence_with_ralph_iterations_stays_busy_and_the_idle_session_is_not()
{
    let s = setup().await;
    let command = agent_command("", &s.log);
    call(&s.rt, OperationId::RunStart, json!({ "benchId": s.bench, "request": {
        "goal": "first", "agentId": "fake-acp", "agentCommand": command, "cwd": s.work, "runId": "r1", "autoAllow": true,
        "ralphLoop": { "enabled": true, "maxIterations": 2, "promptTemplate": "loop-iteration",
                       "stopOnError": false, "stopOnPermission": false, "delayMs": 200 } } })).await;
    wait_for("three prompts ended", || {
        let lines = agent_log(&s.log);
        lines.iter().filter(|l| l.starts_with("end_turn:")).count() == 3
    })
    .await;
    wait_for("sequence reservation released", || {
        s.rt.runtime.work_gate().busy_run_count("r1") == 0
    })
    .await;
    assert_no_gap(&busy_history(&s.events, "r1"), "start + 2 Ralph iterations");
    // 세션은 살아 있다(다음 prompt를 기다림) — 쉬는 세션은 바쁘지 않다.
    assert!(
        s.rt.runtime
            .run_engine()
            .active_owner_of("r1")
            .await
            .is_some(),
        "session still alive"
    );
    assert_eq!(s.rt.runtime.work_gate().busy_run_count("r1"), 0);
    s.rt.runtime.close_all_benches().await;
}

#[test]
fn a_stop_decision_and_concurrent_reservations_never_overlap() {
    for _ in 0..1000 {
        let gate = WorkGate::new();
        gate.begin_drain(DrainMode::Wait);
        let reserver = {
            let gate = gate.clone();
            std::thread::spawn(move || {
                let mut after_stop = 0;
                for _ in 0..50 {
                    match gate.reserve(ReservationKind::Turn, Some("r1")) {
                        Ok(reservation) => {
                            // 예약이 살아 있는 동안에는 정지가 성립할 수 없다.
                            assert!(!gate.is_stopping(), "stopped while a reservation is alive");
                            drop(reservation);
                        }
                        Err(_) => after_stop += 1,
                    }
                }
                after_stop
            })
        };
        let mut stopped = false;
        for _ in 0..50 {
            if gate.try_stop(|| 0) {
                stopped = true;
                break;
            }
            std::thread::yield_now();
        }
        let refused = reserver.join().unwrap();
        if stopped {
            assert!(gate.is_stopping());
            assert!(
                gate.reserve(ReservationKind::Turn, Some("r1")).is_err(),
                "no reservation after stopping"
            );
        } else {
            assert_eq!(refused, 0);
        }
    }
}

/// R14 표 4·5·5': task 기동 토큰. `Pending`에서 G 아래 `Registered{runId}`로 전이하며 T-start를 A-turn으로 원자적으로
/// 인계한다. 전이 전 취소는 전이를 막고(실행 0), 전이 뒤 취소는 실제 run id를 돌려준다. 확정 없이 drop되면 `Failed`.
#[test]
fn a_launch_token_hands_task_start_over_to_a_turn_atomically() {
    let gate = WorkGate::new();
    let ticket = gate.issue_launch().expect("serving gate issues a launch");
    let token = ticket.token();
    assert_eq!(gate.launch_state(token), Some(LaunchState::Pending));
    assert_eq!(
        gate.reservation_counts().get(&ReservationKind::TaskStart),
        Some(&1)
    );

    let turn = gate
        .register_launch(ticket, "run-1")
        .expect("pending token registers");
    assert_eq!(
        gate.launch_state(token),
        Some(LaunchState::Registered("run-1".into()))
    );
    assert_eq!(
        gate.reservation_counts().get(&ReservationKind::TaskStart),
        None
    );
    assert_eq!(gate.busy_run_count("run-1"), 1);
    assert_eq!(
        gate.cancel_launch(token),
        LaunchCancel::Registered("run-1".into())
    );
    drop(turn);
    assert_eq!(gate.reservation_total(), 0);

    // 전이 전 취소: 전이하지 않고 T-start도 남기지 않는다.
    let ticket = gate.issue_launch().unwrap();
    let token = ticket.token();
    assert_eq!(gate.cancel_launch(token), LaunchCancel::Prevented);
    assert!(
        gate.register_launch(ticket, "run-2").is_err(),
        "a cancelled token never registers"
    );
    assert_eq!(gate.launch_state(token), Some(LaunchState::Cancelled));
    assert_eq!(gate.busy_run_count("run-2"), 0);
    assert_eq!(gate.reservation_total(), 0);

    // 확정 없이 drop(준비 실패·abort) → Failed, 예약 해제.
    let ticket = gate.issue_launch().unwrap();
    let token = ticket.token();
    drop(ticket);
    assert_eq!(gate.launch_state(token), Some(LaunchState::Failed));
    assert_eq!(gate.cancel_launch(token), LaunchCancel::Unknown);
    assert_eq!(gate.reservation_total(), 0);

    gate.force_stop();
    assert!(gate.issue_launch().is_err(), "no launch after stopping");
}

/// T011: 런타임이 관문의 상태와 활동 작업(예약 파생 부분)을 보고한다.
#[tokio::test(flavor = "multi_thread")]
async fn the_runtime_reports_gate_state_and_active_work() {
    let rt = TestRuntime::with_adapters(RuntimeAdapters::production());
    let gate = rt.runtime.work_gate().clone();
    assert_eq!(rt.runtime.server_state(), GateState::Serving);
    let idle = rt.runtime.active_work();
    assert_eq!((idle.busy_runs, idle.accepted_calls), (0, 0));

    let turn = gate.reserve(ReservationKind::Turn, Some("run-a")).unwrap();
    let queued = gate.reserve(ReservationKind::Turn, Some("run-a")).unwrap();
    let call = gate.reserve(ReservationKind::Call, None).unwrap();
    let busy = rt.runtime.active_work();
    assert_eq!(
        (busy.busy_runs, busy.accepted_calls),
        (1, 1),
        "busy runs count runs, not reservations"
    );

    drop((turn, queued, call));
    let after = rt.runtime.active_work();
    assert_eq!((after.busy_runs, after.accepted_calls), (0, 0));
    gate.begin_drain(DrainMode::Wait);
    assert_eq!(
        rt.runtime.server_state(),
        GateState::Draining(DrainMode::Wait)
    );
}

/// OCR 구현 리뷰(core 2): 호출 입구는 비우기·정지 판정과 C-call 예약을 **한 잠금 G 아래에서** 한다. 입구 판정과 예약이
/// 따로면 서빙 중 판정을 통과한 새 작업이 그 뒤 시작한 비우기 안에서 예약·실행된다.
#[test]
fn admission_checks_the_drain_and_reserves_the_call_under_one_lock() {
    let gate = WorkGate::new();
    // 서빙 중 받은 새 작업은 받는 순간 예약을 쥔다 — 뒤이은 비우기의 정지 판정이 그 호출을 놓치지 않는다.
    let admitted = gate
        .admit(true, true)
        .expect("serving admits new work")
        .expect("a call reservation");
    gate.begin_drain(DrainMode::Wait);
    assert!(
        !gate.try_stop(|| 0),
        "an admission made before the drain holds its reservation"
    );
    // 비우기 중: 새 작업은 거절(예약 없음), 그 밖은 예약과 함께 받는다.
    assert_eq!(gate.admit(true, true).err(), Some(AdmitRefused::Draining));
    assert_eq!(
        gate.reservation_total(),
        1,
        "a refused admission reserves nothing"
    );
    let control = gate
        .admit(false, true)
        .expect("control passes while draining");
    assert!(control.is_some());
    assert!(gate.admit(false, false).expect("query").is_none());
    drop((admitted, control));
    assert!(gate.try_stop(|| 0));
    // 정지 중: 모두 거절.
    assert_eq!(gate.admit(false, false).err(), Some(AdmitRefused::Stopping));
    assert_eq!(gate.admit(true, true).err(), Some(AdmitRefused::Stopping));
}

/// Codex 구현 리뷰(high): 유휴 비우기의 정지 판정은 세대를 읽고 파생 값을 기다린 뒤 전이한다. 그 사이 임대가 비우기를
/// 서빙으로 되돌렸다면(`resume_serving`) 낡은 판정이 서빙을 `stopping`으로 바꾸면 안 된다.
#[test]
fn a_stop_decision_read_before_a_lease_resumes_serving_does_not_stop() {
    let gate = WorkGate::new();
    gate.begin_drain(DrainMode::Idle);
    let generation = gate.activity_generation(); // 판정 시작(파생 값 읽기 전)
    assert!(
        gate.resume_serving(),
        "a lease returns the idle drain to serving"
    );
    assert!(
        !gate.try_stop_at(generation, || 0),
        "a decision started before the lease must not stop the serving server"
    );
    assert_eq!(gate.state(), GateState::Serving);
    // 비우기에 다시 들어가도 옛 판정은 쓸 수 없다(상태 전이도 세대를 바꾼다).
    gate.begin_drain(DrainMode::Idle);
    assert!(!gate.try_stop_at(generation, || 0));
    assert_eq!(gate.state(), GateState::Draining(DrainMode::Idle));
    // 새로 시작한 판정은 전이한다.
    let fresh = gate.activity_generation();
    assert!(gate.try_stop_at(fresh, || 0));
}

/// OCR 4차 M1: 작업대 닫기(`forget_bench_exchanges`)보다 늦게 도착한 진행 중 전달이 닫힌 작업대의 교환 기록을 되살리지
/// 않는다(`failedExchangeDeliveries`에 닫힌 작업대가 남지 않음). 닫기와 같은 관문 잠금 아래에서 거절·무시한다.
#[test]
fn a_delivery_arriving_after_the_bench_closed_leaves_no_record() {
    use workbench_core::application::work_gate::DeliveryRefused;
    let gate = WorkGate::new();
    gate.forget_bench_exchanges("bench-a");
    assert_eq!(
        gate.begin_exchange_delivery("bench-a", "q1", "r1").err(),
        Some(DeliveryRefused::BenchClosed),
        "a late delivery to a closed bench is refused"
    );
    gate.record_failed_delivery("bench-a", "q1");
    assert!(
        gate.failed_deliveries().is_empty(),
        "a closed bench leaves no failure record: {:?}",
        gate.failed_deliveries()
    );
    assert!(
        gate.begin_exchange_delivery("bench-b", "q1", "r2").is_ok(),
        "another bench is unaffected"
    );
}
