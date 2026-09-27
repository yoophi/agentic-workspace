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
    // 사용자 값: fixture가 적은 요청(치환 전). 앞 응답에서 포착해 넣은 서버 id는 자리표시자로 이미 되돌아간다.
    let inputs: Vec<Value> = fixture
        .steps
        .iter()
        .map(|step| step.request.clone())
        .collect();
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
        let deadline = step.until.as_ref().map(|until| {
            std::time::Instant::now() + std::time::Duration::from_millis(until.timeout_ms)
        });
        let actual = loop {
            let actual = match &harness {
                Some(harness) => {
                    harness
                        .call(Some(&Harness::token_string(&principal)), &request)
                        .await
                }
                None => {
                    runtime
                        .runtime
                        .call(principal.clone(), request.clone())
                        .await
                }
            };
            // 041: 조건 대기 — 관찰 가능한 상태(알림 전달 완료 등)가 될 때까지 같은 조회를 반복한다.
            let Some(until) = &step.until else {
                break actual;
            };
            let reached = actual.as_ref().is_ok_and(|reply| {
                serde_json::to_value(reply)
                    .ok()
                    .and_then(|json| json.pointer(&until.pointer).cloned())
                    .as_ref()
                    == Some(&until.equals)
            });
            if reached {
                break actual;
            }
            assert!(
                std::time::Instant::now() < deadline.expect("deadline"),
                "{} [{label} #{index}]: {} never became {} (last: {actual:?})",
                fixture.name,
                until.pointer,
                until.equals
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
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
    masked(&observed, &inputs)
}

/// 041: 서버가 새로 만든 uuid(보고·명령·알림·과제 id 등)는 두 경로에서 다를 수밖에 없다. 한 실행 안에서 **처음 나온
/// 순서대로** 번호를 붙인 자리표시자(`{{uuid#N}}`)로 일대일로 바꾼다 — 같은 id는 같은 자리표시자, 다른 id는 다른
/// 자리표시자라서 참조가 뒤바뀌거나 잘못 연결되면 경로 비교가 잡는다. 요청 입력에 들어 있던 uuid 모양 문자열(사용자
/// 값)은 바꾸지 않고 그대로 비교한다.
#[derive(Default)]
struct GeneratedIds {
    inputs: std::collections::HashSet<String>,
    assigned: std::collections::HashMap<String, String>,
}

impl GeneratedIds {
    fn note_input(&mut self, value: &Value) {
        match value {
            Value::String(text) => {
                self.inputs.insert(text.clone());
            }
            Value::Array(items) => items.iter().for_each(|item| self.note_input(item)),
            Value::Object(map) => map.values().for_each(|item| self.note_input(item)),
            _ => {}
        }
    }

    fn placeholder(&mut self, id: &str) -> String {
        let next = self.assigned.len() + 1;
        self.assigned
            .entry(id.to_owned())
            .or_insert_with(|| format!("{{{{uuid#{next}}}}}"))
            .clone()
    }

    /// 문자열 안의 하이픈 uuid(36자)를 찾아 바꾼다(요청 fingerprint처럼 JSON 문자열에 박힌 id 포함).
    fn mask_text(&mut self, text: &str) -> String {
        const LEN: usize = 36;
        let mut out = String::with_capacity(text.len());
        let mut index = 0;
        while index < text.len() {
            let end = index + LEN;
            if text.is_char_boundary(index)
                && end <= text.len()
                && text.is_char_boundary(end)
                && text.as_bytes()[index + 8] == b'-'
                && uuid::Uuid::parse_str(&text[index..end]).is_ok()
                && !self.inputs.contains(&text[index..end])
            {
                out.push_str(&self.placeholder(&text[index..end]));
                index = end;
                continue;
            }
            let next = (index + 1..=text.len())
                .find(|position| text.is_char_boundary(*position))
                .unwrap_or(text.len());
            out.push_str(&text[index..next]);
            index = next;
        }
        out
    }

    fn mask(&mut self, value: &mut Value) {
        match value {
            Value::String(text) if !self.inputs.contains(text.as_str()) => {
                *text = self.mask_text(text);
            }
            Value::Array(items) => items.iter_mut().for_each(|item| self.mask(item)),
            Value::Object(map) => map.values_mut().for_each(|item| self.mask(item)),
            _ => {}
        }
    }
}

fn masked(observations: &[Value], inputs: &[Value]) -> Vec<Value> {
    let mut ids = GeneratedIds::default();
    inputs.iter().for_each(|input| ids.note_input(input));
    observations
        .iter()
        .map(|value| {
            let mut value = value.clone();
            ids.mask(&mut value);
            value
        })
        .collect()
}

#[test]
fn generated_id_masking_keeps_identity_and_user_values() {
    let (a, b) = (
        "11111111-1111-4111-8111-111111111111",
        "22222222-2222-4222-8222-222222222222",
    );
    let user = "33333333-3333-4333-8333-333333333333";
    let first = [
        serde_json::json!({ "id": a, "other": b }),
        serde_json::json!({ "ref": a, "input": user }),
    ];
    // 같은 모양, 다른 실제 값: 가려진 뒤 같아야 한다.
    let (c, d) = (
        "44444444-4444-4444-8444-444444444444",
        "55555555-5555-4555-8555-555555555555",
    );
    let same = [
        serde_json::json!({ "id": c, "other": d }),
        serde_json::json!({ "ref": c, "input": user }),
    ];
    let inputs = [serde_json::json!({ "requestId": user })];
    assert_eq!(masked(&first, &inputs), masked(&same, &inputs));
    // 변이: 참조가 다른 객체를 가리킨다 → 달라야 한다.
    let wrong_ref = [
        serde_json::json!({ "id": c, "other": d }),
        serde_json::json!({ "ref": d, "input": user }),
    ];
    assert_ne!(masked(&first, &inputs), masked(&wrong_ref, &inputs));
    // 변이: 두 id를 서로 바꾼다 → 달라야 한다.
    let swapped = [
        serde_json::json!({ "id": d, "other": c }),
        serde_json::json!({ "ref": c, "input": user }),
    ];
    assert_ne!(masked(&first, &inputs), masked(&swapped, &inputs));
    // 사용자 입력 uuid는 가리지 않는다: 값이 다르면 비교가 잡는다.
    let other_user = [
        serde_json::json!({ "id": c, "other": d }),
        serde_json::json!({ "ref": c, "input": "66666666-6666-4666-8666-666666666666" }),
    ];
    assert_ne!(masked(&first, &inputs), masked(&other_user, &inputs));
    // 문자열 안에 박힌 id도 같은 표로 바뀐다: 같은 참조면 같고, 다른 id를 가리키면 다르다.
    let embedded = |id: &str, reference: &str| {
        [serde_json::json!({ "id": id, "fingerprint": format!("{{\"ref\":\"{reference}\"}}") })]
    };
    assert_eq!(
        masked(&embedded(a, a), &inputs),
        masked(&embedded(c, c), &inputs)
    );
    assert_ne!(
        masked(&embedded(a, a), &inputs),
        masked(&embedded(c, d), &inputs)
    );
}
