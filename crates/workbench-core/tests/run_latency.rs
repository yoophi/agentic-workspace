//! 040 T061(R14): `Workbench.call(run.sendPrompt)` 경유가 엔진 직접 호출보다 늘리는 지연(dispatch·소유 검사·세대 멱등
//! 표)을 잰다. 기계에 따라 값이 흔들리므로 `#[ignore]` — `cargo test -p workbench-core --test run_latency -- --ignored
//! --nocapture`로 돌려 결과를 tasks Notes에 기록한다. 기준: p95 증가 < 5ms.

mod support;

use std::time::{Duration, Instant};

use serde_json::json;
use support::{scripted_run_engine::RunScript, uuid_key, BenchHarness};
use workbench_protocol::OperationId;

const ITERATIONS: usize = 500;

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() * 95 / 100]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "timing measurement; run manually and record in tasks Notes"]
async fn send_prompt_through_workbench_adds_under_5ms_at_p95() {
    let h = BenchHarness::new(RunScript::default());
    let bench = h.open().await;
    h.start(&bench, "r1").await.unwrap();
    let engine = h.rt.runtime.run_engine().clone();

    let mut direct = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let sink = h.rt.runtime.benches().run_sink(&bench);
        let started = Instant::now();
        engine.send_prompt("r1", "p".into(), sink).await.unwrap();
        direct.push(started.elapsed());
    }

    let mut via_call = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        h.keyed(
            OperationId::RunSendPrompt,
            &uuid_key(),
            json!({"benchId": bench, "runId": "r1", "prompt": "p"}),
        )
        .await
        .unwrap();
        via_call.push(started.elapsed());
    }

    let (direct, via_call) = (p95(direct), p95(via_call));
    let added = via_call.saturating_sub(direct);
    println!(
        "run.sendPrompt p95: direct {direct:?}, via Workbench.call {via_call:?}, added {added:?}"
    );
    assert!(
        added < Duration::from_millis(5),
        "p95 added latency {added:?} >= 5ms"
    );
}
