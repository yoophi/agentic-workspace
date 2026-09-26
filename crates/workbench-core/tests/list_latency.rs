//! SC-001 보조 측정. CI에서는 `#[ignore]`이며 quickstart §1에서 수동 실행한다:
//! `cargo test -p workbench-core --test list_latency -- --ignored --nocapture`

mod support;

use std::time::{Duration, Instant};

use serde_json::json;
use support::{fixtures::Seed, TestRuntime};
use workbench_protocol::{AuthenticatedPrincipal, CallRequest, OperationId, Workbench};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "latency measurement; run manually with --ignored --nocapture"]
async fn project_list_p95_under_5ms_with_50_projects() {
    let runtime = TestRuntime::new();
    let seed = Seed {
        projects: (0..50)
            .map(|index| {
                json!({
                    "id": format!("project-{index}"),
                    "name": format!("Project {index}"),
                    "workingDirectory": format!("/tmp/project-{index}"),
                    "description": null
                })
            })
            .collect(),
    };
    support::fixtures::apply_seed(&runtime.paths, &seed);

    let mut samples = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let request = CallRequest::query(OperationId::ProjectList, json!({}));
        let started = Instant::now();
        let reply = runtime
            .runtime
            .call(AuthenticatedPrincipal::desktop(), request)
            .await
            .unwrap();
        samples.push(started.elapsed());
        assert_eq!(reply.output().unwrap().as_array().unwrap().len(), 50);
    }
    samples.sort();
    let p50 = samples[samples.len() / 2];
    let p95 = samples[(samples.len() as f64 * 0.95) as usize - 1];
    let max = *samples.last().unwrap();
    println!("project.list in-memory latency: p50={p50:?} p95={p95:?} max={max:?}");
    assert!(p95 < Duration::from_millis(5), "p95 {p95:?} exceeds 5ms");
}
