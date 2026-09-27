//! contract fixture 로더·매처. 파일 형식은 `specs/037-workbench-seam/contracts/workbench-call.md` §5와
//! `specs/038-workbench-domains/contracts/workbench-operations.md` §4(seed 확장·`{{repo}}` 치환).
//! `expect`는 **부분 일치**다: 기대값에 적힌 키만 실제값과 비교하고, `ignoreFields`의 키는 실제값에서 제거한다.

use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::Deserialize;
use serde_json::Value;
use workbench_core::infrastructure::data_paths::DataPaths;
use workbench_protocol::{AuthenticatedPrincipal, CallReply, CallRequest, WorkbenchFault};

use super::git_repo::{self, BuiltRepo, GitRepoSeed};

#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub name: String,
    #[serde(default = "default_principal")]
    pub principal: String,
    #[serde(default)]
    pub seed: Seed,
    #[serde(default)]
    pub request: Option<Value>,
    #[serde(default)]
    pub requests: Vec<Value>,
    #[serde(default)]
    pub expect: Option<Expect>,
    #[serde(default)]
    pub expects: Vec<Expect>,
    #[serde(default, rename = "expectAfter")]
    pub expect_after: Option<ExpectAfter>,
    #[serde(default, rename = "ignoreFields")]
    pub ignore_fields: Vec<String>,
    /// 040: 요청마다 principal·값 포착을 지정하는 순차 호출. 있으면 `request(s)`/`expect(s)` 대신 쓴다.
    #[serde(default)]
    pub steps: Vec<StepSpec>,
    /// 040: 가짜 run 엔진 동작.
    #[serde(default, rename = "runScript")]
    pub run_script: Option<super::scripted_run_engine::RunScript>,
}

/// 040 순차 호출 한 단계. `capture`는 이 단계 응답 JSON(`CallReply` 직렬화)의 JSON pointer를 `{{name}}`으로 저장한다.
#[derive(Debug, Clone, Deserialize)]
pub struct StepSpec {
    #[serde(default)]
    pub principal: Option<String>,
    pub request: Value,
    pub expect: Expect,
    #[serde(default)]
    pub capture: BTreeMap<String, String>,
}

fn default_principal() -> String {
    "desktop".into()
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seed {
    #[serde(default)]
    pub projects: Vec<Value>,
    #[serde(default)]
    pub saved_prompts: Vec<Value>,
    #[serde(default)]
    pub goals: Vec<Value>,
    #[serde(default)]
    pub agent_run_settings: Vec<Value>,
    #[serde(default)]
    pub git_repo: Option<GitRepoSeed>,
    /// US3: stub agent catalog.
    #[serde(default)]
    pub agents: Vec<Value>,
    /// US3: stub provider 세션 목록.
    #[serde(default)]
    pub provider_sessions: Vec<Value>,
    /// 040: 작업대 대상 디렉터리를 만들고 `{{dir}}`(실제 경로)로 치환한다.
    #[serde(default)]
    pub bench_dir: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Expect {
    #[serde(default)]
    pub reply: Option<Value>,
    #[serde(default)]
    pub fault: Option<Value>,
    /// describe 응답의 각 operation에 `inputSchema`/`outputSchema` 객체가 있는지 확인한다.
    #[serde(default, rename = "schemaPresent")]
    pub schema_present: bool,
    /// reply 최상위에 **없어야** 하는 키(예: Git 변경의 `revision`). 부분 일치로는 부재를 표현할 수 없어서 둔다.
    #[serde(default)]
    pub absent: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectAfter {
    #[serde(default)]
    pub projects_len: Option<usize>,
    #[serde(default)]
    pub saved_prompts_len: Option<usize>,
    #[serde(default)]
    pub goals_len: Option<usize>,
    #[serde(default)]
    pub agent_run_settings_len: Option<usize>,
    #[serde(default)]
    pub ledger_applied: Option<usize>,
    /// `git worktree list`의 항목 수(main 포함).
    #[serde(default)]
    pub git_worktrees: Option<usize>,
}

/// seed 적용 결과. 요청·기대의 `{{...}}` 자리표시자를 채운다.
#[derive(Debug, Default)]
pub struct SeedContext {
    pub repo: Option<BuiltRepo>,
    substitutions: BTreeMap<String, String>,
    captured: Vec<(String, String)>,
}

impl SeedContext {
    /// 040: 순차 호출에서 포착한 값. 두 경로 비교 전에 값 → 자리표시자로 되돌린다.
    pub fn capture(&mut self, name: &str, value: String) {
        self.substitutions
            .insert(format!("{{{{{name}}}}}"), value.clone());
        self.captured.push((value, format!("{{{{{name}}}}}")));
    }

    pub fn normalize_captured(&self, value: &mut Value) {
        let pairs: Vec<(&String, &str)> = self
            .captured
            .iter()
            .map(|(actual, placeholder)| (actual, placeholder.as_str()))
            .collect();
        normalize(value, &pairs);
    }

    fn from_repo(repo: Option<BuiltRepo>) -> Self {
        let mut substitutions = BTreeMap::new();
        if let Some(repo) = &repo {
            substitutions.insert(
                "{{repo}}".to_owned(),
                git_repo::canonical_string(&repo.root),
            );
            substitutions.insert("{{repoName}}".to_owned(), repo.name.clone());
            substitutions.insert(
                "{{repoParent}}".to_owned(),
                git_repo::canonical_string(repo.parent()),
            );
            substitutions.insert("{{headHash}}".to_owned(), repo.head().to_owned());
            for (index, hash) in repo.commits.iter().enumerate() {
                substitutions.insert(format!("{{{{commit:{index}}}}}"), hash.clone());
            }
        }
        Self {
            repo,
            substitutions,
            captured: Vec::new(),
        }
    }

    /// 두 경로(in-memory·HTTP)는 서로 다른 임시 저장소를 쓰므로, 결과를 비교하기 전에 저장소 경로를 자리표시자로
    /// 되돌린다. 긴 경로(`{{repo}}`)를 먼저 바꾼다 — `{{repoParent}}`는 그 접두어다. 해시는 결정적이라 두지 않는다.
    pub fn normalize_paths(&self, value: &mut Value) {
        let mut pairs: Vec<(&String, &str)> = ["{{repo}}", "{{repoParent}}", "{{dir}}"]
            .into_iter()
            .filter_map(|key| self.substitutions.get(key).map(|actual| (actual, key)))
            .collect();
        pairs.sort_by_key(|(actual, _)| std::cmp::Reverse(actual.len()));
        normalize(value, &pairs);
    }

    pub fn substitute(&self, value: &mut Value) {
        if self.substitutions.is_empty() {
            return;
        }
        match value {
            Value::String(text) => {
                if text.contains("{{") {
                    let mut out = text.clone();
                    for (from, to) in &self.substitutions {
                        out = out.replace(from, to);
                    }
                    *text = out;
                }
            }
            Value::Array(items) => items.iter_mut().for_each(|item| self.substitute(item)),
            Value::Object(map) => map.values_mut().for_each(|item| self.substitute(item)),
            _ => {}
        }
    }
}

fn normalize(value: &mut Value, pairs: &[(&String, &str)]) {
    match value {
        Value::String(text) => {
            for (actual, placeholder) in pairs {
                if text.contains(actual.as_str()) {
                    *text = text.replace(actual.as_str(), placeholder);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| normalize(item, pairs)),
        Value::Object(map) => map.values_mut().for_each(|item| normalize(item, pairs)),
        _ => {}
    }
}

impl Seed {
    /// `seed.agents`·`seed.providerSessions`로 stub 어댑터를 만든다(038 US3).
    pub fn adapters(&self) -> workbench_core::application::workbench_runtime::RuntimeAdapters {
        let agents = self
            .agents
            .iter()
            .map(|agent| serde_json::from_value(agent.clone()).expect("seed agent"))
            .collect();
        let sessions = self
            .provider_sessions
            .iter()
            .map(|session| serde_json::from_value(session.clone()).expect("seed provider session"))
            .collect();
        super::stub_adapters(agents, sessions)
    }
}

/// 이름 → principal. `desktop2`는 데스크톱과 같은 scope의 다른 주체, `agent:<runId>`는 run에 묶인 agent(040).
pub fn principal_named(fixture: &str, name: &str) -> AuthenticatedPrincipal {
    match name {
        "desktop" => AuthenticatedPrincipal::desktop(),
        "readonly" => AuthenticatedPrincipal::test_readonly(),
        "desktop2" => AuthenticatedPrincipal::test_as("desktop2"),
        other => match other.strip_prefix("agent:") {
            Some(run_id) => AuthenticatedPrincipal::agent(run_id),
            None => panic!("fixture {fixture}: unknown principal {other}"),
        },
    }
}

impl Fixture {
    pub fn principal(&self) -> AuthenticatedPrincipal {
        principal_named(&self.name, &self.principal)
    }

    /// seed의 stub 어댑터 + `runScript`의 가짜 엔진(040).
    pub fn adapters(&self) -> workbench_core::application::workbench_runtime::RuntimeAdapters {
        let agents = self
            .seed
            .agents
            .iter()
            .map(|agent| serde_json::from_value(agent.clone()).expect("seed agent"))
            .collect();
        let sessions = self
            .seed
            .provider_sessions
            .iter()
            .map(|session| serde_json::from_value(session.clone()).expect("seed provider session"))
            .collect();
        super::stub_adapters_with(
            agents,
            sessions,
            self.run_script.clone().unwrap_or_default(),
        )
        .0
    }

    /// (요청, 기대) 순서쌍. 치환 없음(037 호환).
    pub fn steps(&self) -> Vec<(CallRequest, Expect)> {
        self.steps_with(&SeedContext::default())
    }

    /// (요청, 기대) 순서쌍. `{{repo}}` 등 자리표시자를 seed 결과로 치환한다.
    pub fn steps_with(&self, ctx: &SeedContext) -> Vec<(CallRequest, Expect)> {
        let requests: Vec<Value> = match (&self.request, self.requests.is_empty()) {
            (Some(single), true) => vec![single.clone()],
            (None, false) => self.requests.clone(),
            _ => panic!("fixture {}: use exactly one of request/requests", self.name),
        };
        let expects: Vec<Expect> = match (&self.expect, self.expects.is_empty()) {
            (Some(single), true) => vec![single.clone()],
            (None, false) => self.expects.clone(),
            _ => panic!("fixture {}: use exactly one of expect/expects", self.name),
        };
        assert_eq!(
            requests.len(),
            expects.len(),
            "fixture {}: requests/expects length mismatch",
            self.name
        );
        requests
            .into_iter()
            .zip(expects)
            .map(|(mut request, mut expect)| {
                ctx.substitute(&mut request);
                if let Some(reply) = &mut expect.reply {
                    ctx.substitute(reply);
                }
                if let Some(fault) = &mut expect.fault {
                    ctx.substitute(fault);
                }
                let request: CallRequest = serde_json::from_value(request)
                    .unwrap_or_else(|error| panic!("fixture {}: bad request: {error}", self.name));
                (request, expect)
            })
            .collect()
    }
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("workbench-protocol")
        .join("fixtures")
}

pub fn load_all() -> Vec<Fixture> {
    let mut paths: Vec<PathBuf> = fs::read_dir(fixtures_dir())
        .expect("fixtures dir")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "no fixtures found in {}",
        fixtures_dir().display()
    );
    paths
        .into_iter()
        .map(|path| {
            let contents = fs::read_to_string(&path).expect("read fixture");
            serde_json::from_str::<Fixture>(&contents)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        })
        .collect()
}

pub fn load_by_prefix(prefix: &str) -> Vec<Fixture> {
    load_all()
        .into_iter()
        .filter(|fixture| fixture.name.starts_with(prefix))
        .collect()
}

fn write_or_remove(path: PathBuf, items: &[Value]) {
    if items.is_empty() {
        let _ = fs::remove_file(path);
        return;
    }
    fs::write(path, serde_json::to_vec_pretty(items).expect("seed json")).expect("write seed");
}

/// seed를 데이터 디렉터리와(있으면) Git 저장소로 만든다. Git 저장소는 `<app_data_dir>/repos/` 아래에 생긴다.
pub fn apply_seed(paths: &DataPaths, seed: &Seed) -> SeedContext {
    paths.ensure_dirs().expect("dirs");
    write_or_remove(paths.projects_file(), &seed.projects);
    write_or_remove(paths.saved_prompts_file(), &seed.saved_prompts);
    write_or_remove(paths.goals_file(), &seed.goals);
    write_or_remove(paths.agent_run_settings_file(), &seed.agent_run_settings);
    let repo = seed.git_repo.as_ref().map(|git_seed| {
        let parent = paths.app_data_dir().join("repos");
        fs::create_dir_all(&parent).expect("repos dir");
        git_repo::build(git_seed, &parent)
    });
    let mut ctx = SeedContext::from_repo(repo);
    if seed.bench_dir {
        let dir = paths.app_data_dir().join("bench-work");
        fs::create_dir_all(&dir).expect("bench dir");
        ctx.substitutions
            .insert("{{dir}}".to_owned(), git_repo::canonical_string(&dir));
    }
    ctx
}

fn strip_ignored(value: &mut Value, ignore: &[String]) {
    match value {
        Value::Object(map) => {
            for key in ignore {
                map.remove(key);
            }
            for child in map.values_mut() {
                strip_ignored(child, ignore);
            }
        }
        Value::Array(items) => items
            .iter_mut()
            .for_each(|item| strip_ignored(item, ignore)),
        _ => {}
    }
}

/// `expected`에 적힌 것만 `actual`과 비교한다. 배열은 길이와 요소별 부분 일치.
fn subset_matches(expected: &Value, actual: &Value, path: &str, mismatches: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Object(exp), Value::Object(act)) => {
            for (key, exp_value) in exp {
                match act.get(key) {
                    Some(act_value) => {
                        subset_matches(exp_value, act_value, &format!("{path}/{key}"), mismatches)
                    }
                    None => {
                        mismatches.push(format!("{path}/{key}: missing (expected {exp_value})"))
                    }
                }
            }
        }
        (Value::Array(exp), Value::Array(act)) => {
            if exp.len() != act.len() {
                mismatches.push(format!(
                    "{path}: length {} != expected {}",
                    act.len(),
                    exp.len()
                ));
                return;
            }
            for (index, (e, a)) in exp.iter().zip(act).enumerate() {
                subset_matches(e, a, &format!("{path}/{index}"), mismatches);
            }
        }
        (exp, act) => {
            if exp != act {
                mismatches.push(format!("{path}: {act} != expected {exp}"));
            }
        }
    }
}

pub fn assert_matches(
    label: &str,
    actual: &Result<CallReply, WorkbenchFault>,
    expect: &Expect,
    ignore: &[String],
) {
    let mut mismatches = Vec::new();
    match (&expect.reply, &expect.fault, actual) {
        (Some(expected_reply), None, Ok(reply)) => {
            let mut actual_value = serde_json::to_value(reply).expect("reply json");
            strip_ignored(&mut actual_value, ignore);
            subset_matches(expected_reply, &actual_value, "reply", &mut mismatches);
            for key in &expect.absent {
                if actual_value.get(key).is_some() {
                    mismatches.push(format!(
                        "reply/{key}: expected absent, got {}",
                        actual_value[key]
                    ));
                }
            }
            if expect.schema_present {
                let operations = actual_value["output"]["operations"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                for (index, operation) in operations.iter().enumerate() {
                    if !operation["inputSchema"].is_object()
                        || !operation["outputSchema"].is_object()
                    {
                        mismatches.push(format!("reply/output/operations/{index}: schema missing"));
                    }
                }
            }
        }
        (None, Some(expected_fault), Err(fault)) => {
            let actual_value = serde_json::to_value(fault).expect("fault json");
            subset_matches(expected_fault, &actual_value, "fault", &mut mismatches);
        }
        (Some(_), None, Err(fault)) => {
            mismatches.push(format!("expected reply, got fault {fault}"))
        }
        (None, Some(_), Ok(reply)) => {
            mismatches.push(format!("expected fault, got reply {reply:?}"))
        }
        _ => panic!("{label}: expect must have exactly one of reply/fault"),
    }
    assert!(
        mismatches.is_empty(),
        "{label}:\n  {}",
        mismatches.join("\n  ")
    );
}
