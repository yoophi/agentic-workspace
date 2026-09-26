//! contract fixture 로더·매처. 파일 형식은 `specs/037-workbench-seam/contracts/workbench-call.md` §5.
//! `expect`는 **부분 일치**다: 기대값에 적힌 키만 실제값과 비교하고, `ignoreFields`의 키는 실제값에서 제거한다.

use std::{fs, path::PathBuf};

use serde::Deserialize;
use serde_json::Value;
use workbench_core::infrastructure::data_paths::DataPaths;
use workbench_protocol::{AuthenticatedPrincipal, CallReply, CallRequest, WorkbenchFault};

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
}

fn default_principal() -> String {
    "desktop".into()
}

#[derive(Debug, Default, Deserialize)]
pub struct Seed {
    #[serde(default)]
    pub projects: Vec<Value>,
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
}

#[derive(Debug, Deserialize)]
pub struct ExpectAfter {
    #[serde(default, rename = "projectsLen")]
    pub projects_len: Option<usize>,
    #[serde(default, rename = "ledgerApplied")]
    pub ledger_applied: Option<usize>,
}

impl Fixture {
    pub fn principal(&self) -> AuthenticatedPrincipal {
        match self.principal.as_str() {
            "desktop" => AuthenticatedPrincipal::desktop(),
            "readonly" => AuthenticatedPrincipal::test_readonly(),
            other => panic!("fixture {}: unknown principal {other}", self.name),
        }
    }

    /// (요청, 기대) 순서쌍. 단일 `request`/`expect` 또는 `requests`/`expects` 시퀀스.
    pub fn steps(&self) -> Vec<(CallRequest, Expect)> {
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
            .map(|(request, expect)| {
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

pub fn apply_seed(paths: &DataPaths, seed: &Seed) {
    paths.ensure_dirs().expect("dirs");
    if seed.projects.is_empty() {
        let _ = fs::remove_file(paths.projects_file());
        return;
    }
    fs::write(
        paths.projects_file(),
        serde_json::to_vec_pretty(&seed.projects).expect("seed json"),
    )
    .expect("write seed");
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
