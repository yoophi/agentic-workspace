//! 042 SC-004: in-process 경로와 HTTP 경로가 같은 대상을 동시에 바꿔도 손실이 없다(저장 주체는 하나).
//! 프로젝트 목록(ledger + JSON)과 orchestration 작업 영역(revision 검사 RMW)을 각각 두 경로에서 동시에 바꾼다.

#![allow(clippy::result_large_err)]

mod support;

use std::sync::Arc;

use serde_json::json;
use support::{
    command_request, create_request,
    http_harness::{Harness, TOKEN_DESKTOP},
    scripted_run_engine::RunScript,
    uuid_key, BenchHarness, TestRuntime,
};
use workbench_protocol::{AuthenticatedPrincipal, CallRequest, FaultCode, OperationId, Workbench};

const PER_PATH: usize = 60;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_project_creates_over_both_paths_lose_nothing() {
    let rt = TestRuntime::new();
    let dir = rt.dir.path().join("wt");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.to_string_lossy().into_owned();
    let harness = Arc::new(Harness::spawn(rt.runtime.clone() as Arc<dyn Workbench>).await);
    let mut tasks = Vec::new();
    for index in 0..PER_PATH {
        let runtime = rt.runtime.clone();
        let request = create_request(&format!("local-{index}"), &format!("local-{index}"), &dir);
        tasks.push(tokio::spawn(async move {
            runtime
                .call(AuthenticatedPrincipal::desktop(), request)
                .await
                .map(|_| ())
        }));
        let harness = Arc::clone(&harness);
        let request = create_request(&format!("http-{index}"), &format!("http-{index}"), &dir);
        tasks.push(tokio::spawn(async move {
            harness
                .call(Some(TOKEN_DESKTOP), &request)
                .await
                .map(|_| ())
        }));
    }
    for task in tasks {
        task.await.unwrap().expect("every create succeeds");
    }
    let names: std::collections::HashSet<String> = rt
        .projects()
        .iter()
        .map(|project| project["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names.len(), PER_PATH * 2, "no lost project");
    for index in 0..PER_PATH {
        assert!(names.contains(&format!("local-{index}")));
        assert!(names.contains(&format!("http-{index}")));
    }
}

async fn revision(h: &BenchHarness, bench: &str) -> serde_json::Value {
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::OrchestrationGet,
        json!({ "benchId": bench }),
    )
    .await
    .unwrap()["revision"]
        .clone()
}

fn delegate(bench: &str, goal: &str, revision: serde_json::Value) -> CallRequest {
    command_request(
        OperationId::OrchestrationDelegateGoal,
        &uuid_key(),
        json!({ "benchId": bench, "request": {
            "requestId": uuid::Uuid::new_v4().to_string(), "goal": goal,
            "expectedRevision": revision } }),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_orchestration_writes_over_both_paths_lose_nothing() {
    const GOALS: usize = 20;
    let h = Arc::new(BenchHarness::new(RunScript::default()));
    let bench = h.open().await;
    let session = h
        .call(
            &AuthenticatedPrincipal::desktop(),
            OperationId::OrchestrationBootstrap,
            json!({ "benchId": bench, "worktreePath": h.dir }),
        )
        .await
        .unwrap();
    h.start(&bench, "main-run").await.unwrap();
    h.call(
        &AuthenticatedPrincipal::desktop(),
        OperationId::OrchestrationBindCoordinator,
        json!({ "benchId": bench, "request": {
            "requestId": "bind", "panelId": "main-agent-run", "runId": "main-run",
            "state": "active", "expectedRevision": session["revision"] } }),
    )
    .await
    .unwrap();
    let harness = Arc::new(Harness::spawn(h.rt.runtime.clone() as Arc<dyn Workbench>).await);
    let mut tasks = Vec::new();
    for index in 0..GOALS {
        for over_http in [false, true] {
            let (h, harness, bench) = (Arc::clone(&h), Arc::clone(&harness), bench.clone());
            let goal = format!("goal-{}-{index}", if over_http { "http" } else { "local" });
            tasks.push(tokio::spawn(async move {
                // revision 경합은 conflict로 거절된다 — 새 revision으로 다시 시도(손실이 아니라 재시도).
                for _ in 0..500 {
                    let request = delegate(&bench, &goal, revision(&h, &bench).await);
                    let outcome = if over_http {
                        harness.call(Some(TOKEN_DESKTOP), &request).await
                    } else {
                        h.rt.runtime
                            .call(AuthenticatedPrincipal::desktop(), request)
                            .await
                    };
                    match outcome {
                        Ok(_) => return goal,
                        Err(fault) if fault.code == FaultCode::Conflict => continue,
                        Err(fault) => panic!("{goal}: {fault:?}"),
                    }
                }
                panic!("{goal}: never applied");
            }));
        }
    }
    let mut accepted = Vec::new();
    for task in tasks {
        accepted.push(task.await.unwrap());
    }
    let path =
        h.rt.paths
            .app_data_dir()
            .join("orchestration-sessions.json");
    let file = std::fs::read_to_string(path).unwrap();
    for goal in &accepted {
        assert!(
            file.contains(&format!("\"{goal}\"")),
            "{goal} was accepted but is missing from the workspace"
        );
    }
    assert_eq!(accepted.len(), GOALS * 2);
}
