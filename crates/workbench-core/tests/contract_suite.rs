//! 세 Adapter 공통 contract suite(SC-002). fixture는 `crates/workbench-protocol/fixtures/`.
//! 여기서는 in-memory(직접 `call`)와 테스트 HTTP 경로를 돌리고 두 결과가 서로 같은지도 확인한다.
//! Tauri compat 경로는 `apps/agentic-workbench/src-tauri/src/inbound/workbench_compat.rs`의 유닛 테스트가
//! 같은 fixture로 변환 함수를 검증한다.

mod support;

use std::{fs, sync::Arc};

use serde_json::Value;
use support::{
    fixtures::{self, Fixture, SeedContext},
    git_repo,
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

fn store_len(path: &std::path::Path) -> usize {
    if !path.exists() {
        return 0;
    }
    serde_json::from_str::<Vec<Value>>(&fs::read_to_string(path).unwrap())
        .unwrap()
        .len()
}

fn check_after(label: &str, runtime: &TestRuntime, fixture: &Fixture, seed: &SeedContext) {
    let Some(after) = &fixture.expect_after else {
        return;
    };
    for (name, expected, path) in [
        (
            "projects.json",
            after.projects_len,
            runtime.paths.projects_file(),
        ),
        (
            "saved-prompts.json",
            after.saved_prompts_len,
            runtime.paths.saved_prompts_file(),
        ),
        ("goals.json", after.goals_len, runtime.paths.goals_file()),
        (
            "agent-run-settings.json",
            after.agent_run_settings_len,
            runtime.paths.agent_run_settings_file(),
        ),
    ] {
        if let Some(expected_len) = expected {
            assert_eq!(store_len(&path), expected_len, "{label}: {name} length");
        }
    }
    if let Some(expected_worktrees) = after.git_worktrees {
        let repo = seed
            .repo
            .as_ref()
            .unwrap_or_else(|| panic!("{label}: gitWorktrees needs seed.gitRepo"));
        assert_eq!(
            git_repo::worktree_count(&repo.root),
            expected_worktrees,
            "{label}: git worktree count"
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
    // 개발 중 한 묶음만 돌릴 때: `WORKBENCH_FIXTURE_FILTER=git-create cargo test --test contract_suite`
    let filter = std::env::var("WORKBENCH_FIXTURE_FILTER").ok();
    let all: Vec<Fixture> = fixtures::load_all()
        .into_iter()
        .filter(|fixture| fixture.steps.is_empty())
        .filter(|fixture| {
            filter
                .as_deref()
                .is_none_or(|prefix| fixture.name.starts_with(prefix))
        })
        .collect();
    assert!(
        filter.is_some() || all.len() >= 9,
        "expected at least the US1 fixtures, found {}",
        all.len()
    );

    for fixture in &all {
        let principal = fixture.principal();

        // in-memory: runtime.call을 직접 호출
        let mem = TestRuntime::with_adapters(fixture.adapters());
        let mem_seed = fixtures::apply_seed(&mem.paths, &fixture.seed);
        let steps = fixture.steps_with(&mem_seed);
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
        check_after(
            &format!("{} [in-memory]", fixture.name),
            &mem,
            fixture,
            &mem_seed,
        );

        // HTTP: 실제 loopback 왕복 (별도 seed → 별도 저장소 경로이므로 steps도 다시 치환)
        let http = TestRuntime::with_adapters(fixture.adapters());
        let http_seed = fixtures::apply_seed(&http.paths, &fixture.seed);
        let steps = fixture.steps_with(&http_seed);
        let workbench: Arc<dyn Workbench> = http.runtime.clone();
        let harness = Harness::spawn(workbench).await;
        let token = Harness::token_string(&principal);
        for (index, (request, expect)) in steps.iter().enumerate() {
            let actual = harness.call(Some(&token), request).await;
            fixtures::assert_matches(
                &format!("{} [http #{index}]", fixture.name),
                &actual,
                expect,
                &fixture.ignore_fields,
            );
            let mut http_value = observable(&actual, &fixture.ignore_fields);
            http_seed.normalize_paths(&mut http_value);
            let mut mem_value = observable(&mem_results[index], &fixture.ignore_fields);
            mem_seed.normalize_paths(&mut mem_value);
            assert_eq!(
                http_value, mem_value,
                "{} #{index}: http and in-memory diverge",
                fixture.name
            );
        }
        check_after(
            &format!("{} [http]", fixture.name),
            &http,
            fixture,
            &http_seed,
        );
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

/// 040: 순차 호출(`steps`) fixture. 요청마다 principal을 바꾸고, 앞 응답에서 포착한 값(`{{bench}}` 등)을 뒤 요청에
/// 넣는다. 포착 값은 두 경로에서 다르므로 비교 전에 자리표시자로 되돌린다.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_step_fixture_matches_on_in_memory_and_http_paths() {
    let filter = std::env::var("WORKBENCH_FIXTURE_FILTER").ok();
    let all: Vec<Fixture> = fixtures::load_all()
        .into_iter()
        .filter(|fixture| !fixture.steps.is_empty())
        .filter(|fixture| {
            filter
                .as_deref()
                .is_none_or(|prefix| fixture.name.starts_with(prefix))
        })
        .collect();
    for fixture in &all {
        let mem = run_steps(fixture, false).await;
        let http = run_steps(fixture, true).await;
        assert_eq!(http, mem, "{}: http and in-memory diverge", fixture.name);
    }
}

async fn run_steps(fixture: &Fixture, over_http: bool) -> Vec<Value> {
    let label = if over_http { "http" } else { "in-memory" };
    let runtime = TestRuntime::with_adapters(fixture.adapters());
    let mut seed = fixtures::apply_seed(&runtime.paths, &fixture.seed);
    let harness = if over_http {
        let workbench: Arc<dyn Workbench> = runtime.runtime.clone();
        Some(Harness::spawn(workbench).await)
    } else {
        None
    };
    let mut observed = Vec::new();
    for (index, step) in fixture.steps.iter().enumerate() {
        let mut request = step.request.clone();
        seed.substitute(&mut request);
        let request: workbench_protocol::CallRequest = serde_json::from_value(request)
            .unwrap_or_else(|error| panic!("fixture {}: bad request: {error}", fixture.name));
        let mut principal_name = Value::String(
            step.principal
                .clone()
                .unwrap_or_else(|| fixture.principal.clone()),
        );
        seed.substitute(&mut principal_name);
        let principal = fixtures::principal_named(&fixture.name, principal_name.as_str().unwrap());
        let actual = match &harness {
            Some(harness) => {
                harness
                    .call(Some(&Harness::token_string(&principal)), &request)
                    .await
            }
            None => runtime.runtime.call(principal, request).await,
        };
        let mut expect = step.expect.clone();
        if let Some(reply) = &mut expect.reply {
            seed.substitute(reply);
        }
        if let Some(fault) = &mut expect.fault {
            seed.substitute(fault);
        }
        fixtures::assert_matches(
            &format!("{} [{label} #{index}]", fixture.name),
            &actual,
            &expect,
            &fixture.ignore_fields,
        );
        if let Ok(reply) = &actual {
            let reply_json = serde_json::to_value(reply).unwrap();
            for (name, pointer) in &step.capture {
                let value = reply_json.pointer(pointer).unwrap_or_else(|| {
                    panic!(
                        "{} #{index}: capture {pointer} missing in {reply_json}",
                        fixture.name
                    )
                });
                let value = value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string());
                seed.capture(name, value);
            }
        }
        let mut value = observable(&actual, &fixture.ignore_fields);
        seed.normalize_paths(&mut value);
        seed.normalize_captured(&mut value);
        observed.push(value);
    }
    observed
}
