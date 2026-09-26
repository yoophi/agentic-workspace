//! 세 Adapter 공통 contract suite(SC-002). fixture는 `crates/workbench-protocol/fixtures/`.
//! 여기서는 in-memory(직접 `call`)와 테스트 HTTP 경로를 돌리고 두 결과가 서로 같은지도 확인한다.
//! Tauri compat 경로는 `apps/agentic-workbench/src-tauri/src/inbound/workbench_compat.rs`의 유닛 테스트가
//! 같은 fixture로 변환 함수를 검증한다.

mod support;

use std::{fs, sync::Arc};

use serde_json::Value;
use support::{
    fixtures::{self, Fixture},
    http_harness::Harness,
    TestRuntime,
};
use workbench_core::ports::operation_ledger::LedgerState;
use workbench_protocol::{CallReply, Workbench, WorkbenchFault};

fn strip(value: &mut Value, ignore: &[String]) {
    match value {
        Value::Object(map) => {
            for key in ignore {
                map.remove(key);
            }
            map.values_mut().for_each(|child| strip(child, ignore));
        }
        Value::Array(items) => items.iter_mut().for_each(|item| strip(item, ignore)),
        _ => {}
    }
}

/// 두 경로의 관측 결과를 비교 가능한 JSON으로 만든다. requestId는 시도별 값이라 제외한다.
fn observable(result: &Result<CallReply, WorkbenchFault>, ignore: &[String]) -> Value {
    match result {
        Ok(reply) => {
            let mut value = serde_json::to_value(reply).unwrap();
            strip(&mut value, ignore);
            value
        }
        Err(fault) => serde_json::json!({
            "code": fault.code,
            "message": fault.message,
            "outcome": fault.outcome,
            "retryable": fault.retryable,
            "details": fault.details,
        }),
    }
}

fn check_after(label: &str, runtime: &TestRuntime, fixture: &Fixture) {
    let Some(after) = &fixture.expect_after else {
        return;
    };
    if let Some(expected_len) = after.projects_len {
        let projects: Vec<Value> = if runtime.paths.projects_file().exists() {
            serde_json::from_str(&fs::read_to_string(runtime.paths.projects_file()).unwrap())
                .unwrap()
        } else {
            Vec::new()
        };
        assert_eq!(
            projects.len(),
            expected_len,
            "{label}: projects.json length"
        );
    }
    if let Some(expected_applied) = after.ledger_applied {
        let applied = runtime
            .runtime
            .ledger()
            .count_by_state(LedgerState::Applied)
            .unwrap();
        assert_eq!(applied, expected_applied, "{label}: ledger applied rows");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_fixture_matches_on_in_memory_and_http_paths() {
    let all = fixtures::load_all();
    assert!(
        all.len() >= 9,
        "expected at least the US1 fixtures, found {}",
        all.len()
    );

    for fixture in &all {
        let principal = fixture.principal();
        let steps = fixture.steps();

        // in-memory: runtime.call을 직접 호출
        let mem = TestRuntime::new();
        fixtures::apply_seed(&mem.paths, &fixture.seed);
        let mut mem_results = Vec::new();
        for (index, (request, expect)) in steps.iter().enumerate() {
            let actual = mem.runtime.call(principal.clone(), request.clone()).await;
            fixtures::assert_matches(
                &format!("{} [in-memory #{index}]", fixture.name),
                &actual,
                expect,
                &fixture.ignore_fields,
            );
            mem_results.push(actual);
        }
        check_after(&format!("{} [in-memory]", fixture.name), &mem, fixture);

        // HTTP: 실제 loopback 왕복
        let http = TestRuntime::new();
        fixtures::apply_seed(&http.paths, &fixture.seed);
        let workbench: Arc<dyn Workbench> = http.runtime.clone();
        let harness = Harness::spawn(workbench).await;
        let token = Harness::token_for(&principal);
        for (index, (request, expect)) in steps.iter().enumerate() {
            let actual = harness.call(Some(token), request).await;
            fixtures::assert_matches(
                &format!("{} [http #{index}]", fixture.name),
                &actual,
                expect,
                &fixture.ignore_fields,
            );
            assert_eq!(
                observable(&actual, &fixture.ignore_fields),
                observable(&mem_results[index], &fixture.ignore_fields),
                "{} #{index}: http and in-memory diverge",
                fixture.name
            );
        }
        check_after(&format!("{} [http]", fixture.name), &http, fixture);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_without_bearer_is_unauthenticated() {
    let http = TestRuntime::new();
    let workbench: Arc<dyn Workbench> = http.runtime.clone();
    let harness = Harness::spawn(workbench).await;
    let request = workbench_protocol::CallRequest::query(
        workbench_protocol::OperationId::ProjectList,
        serde_json::json!({}),
    );
    let fault = harness.call(None, &request).await.unwrap_err();
    assert_eq!(fault.code, workbench_protocol::FaultCode::Unauthenticated);
    let fault = harness.call(Some("nope"), &request).await.unwrap_err();
    assert_eq!(fault.code, workbench_protocol::FaultCode::Unauthenticated);
}
