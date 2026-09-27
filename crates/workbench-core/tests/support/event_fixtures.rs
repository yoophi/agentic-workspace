//! 이벤트 구독 fixture 로더·실행기(039). 형식: `specs/039-workbench-events/contracts/workbench-events.md` §7.
//! 같은 fixture를 in-memory(`runtime.events`)와 테스트 WebSocket으로 실행하고, 정규화한 아이템 목록을 돌려준다.

use std::{fs, path::PathBuf, sync::Arc, time::Duration};

use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use workbench_core::infrastructure::event_hub::EventHubLimits;
use workbench_protocol::{
    events::{EventFrame, StreamKind, RUN_EVENT_V1},
    AuthenticatedPrincipal, EventItem, EventStream, StreamCursor, Subscription, Workbench,
    WorkbenchFault,
};

use super::{http_harness::Harness, stub_adapters, TestRuntime};

const ITEM_WAIT: Duration = Duration::from_secs(3);
const QUIET_WAIT: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub run_journal_capacity: Option<usize>,
    pub max_retained_runs: Option<usize>,
    pub max_tombstones: Option<usize>,
    pub subscriber_queue: Option<usize>,
    pub max_subscriptions: Option<usize>,
    pub max_cursors: Option<usize>,
}

impl Limits {
    fn apply(&self) -> EventHubLimits {
        let base = EventHubLimits::default();
        EventHubLimits {
            run_journal_capacity: self
                .run_journal_capacity
                .unwrap_or(base.run_journal_capacity),
            max_retained_runs: self.max_retained_runs.unwrap_or(base.max_retained_runs),
            max_tombstones: self.max_tombstones.unwrap_or(base.max_tombstones),
            subscriber_queue: self.subscriber_queue.unwrap_or(base.subscriber_queue),
            max_subscriptions: self.max_subscriptions.unwrap_or(base.max_subscriptions),
            max_cursors: self.max_cursors.unwrap_or(base.max_cursors),
        }
    }
}

/// fixture 단계. `publish`는 run 스트림에 `count`개(마지막 하나에 `terminal`), `hold`는 구독 하나를 붙잡아 둔다,
/// `write`는 `{{tmp}}` 아래 파일을 쓴다(worktree 알림용), `sleepMs`는 기다린다.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Step {
    Publish {
        publish: String,
        #[serde(default = "one")]
        count: usize,
        #[serde(default)]
        terminal: bool,
    },
    Hold {
        hold: Vec<CursorSpec>,
    },
    Write {
        write: String,
        #[serde(default)]
        content: String,
    },
    Sleep {
        #[serde(rename = "sleepMs")]
        sleep_ms: u64,
    },
}

fn one() -> usize {
    1
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CursorSpec {
    pub stream_id: String,
    #[serde(default = "epoch_placeholder")]
    pub epoch: String,
    #[serde(default)]
    pub after_sequence: u64,
}

fn epoch_placeholder() -> String {
    "{{epoch}}".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Expect {
    #[serde(default)]
    pub fault: Option<Value>,
    #[serde(default)]
    pub items: Vec<Value>,
    /// true면 기대 아이템 뒤에 스트림이 끝나야 한다. false면 열린 채 조용해야 한다.
    #[serde(default)]
    pub end: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventFixture {
    pub name: String,
    #[serde(default = "desktop")]
    pub principal: String,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub setup: Vec<Step>,
    pub subscribe: Vec<CursorSpec>,
    #[serde(default)]
    pub after: Vec<Step>,
    pub expect: Expect,
    /// WebSocket 경로에서 재현할 수 없는 fixture(구독자 대기열 초과 등).
    #[serde(default)]
    pub in_memory_only: bool,
}

fn desktop() -> String {
    "desktop".into()
}

impl EventFixture {
    pub fn principal(&self) -> AuthenticatedPrincipal {
        match self.principal.as_str() {
            "desktop" => AuthenticatedPrincipal::desktop(),
            "readonly" => AuthenticatedPrincipal::test_readonly(),
            "noscope" => super::http_harness::noscope_principal(),
            other => panic!("{}: unknown principal {other}", self.name),
        }
    }
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../workbench-protocol/fixtures/events")
}

pub fn load_all() -> Vec<EventFixture> {
    let mut paths: Vec<PathBuf> = fs::read_dir(fixtures_dir())
        .expect("events fixtures dir")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            serde_json::from_str(&fs::read_to_string(&path).expect("read"))
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        })
        .collect()
}

/// 한 경로의 실행 결과.
#[derive(Debug)]
pub enum Outcome {
    Fault(WorkbenchFault),
    Items { items: Vec<Value>, ended: bool },
}

pub struct Context {
    pub rt: TestRuntime,
    pub tmp: tempfile::TempDir,
    held: Vec<EventStream>,
}

impl Context {
    pub fn new(fixture: &EventFixture) -> Self {
        let mut adapters = stub_adapters(Vec::new(), Vec::new());
        adapters.event_limits = fixture.limits.apply();
        Self {
            rt: TestRuntime::with_adapters(adapters),
            tmp: tempfile::tempdir().expect("tmp"),
            held: Vec::new(),
        }
    }

    fn substitute(&self, text: &str) -> String {
        text.replace("{{epoch}}", self.rt.runtime.epoch())
            .replace("{{tmp}}", &self.tmp.path().to_string_lossy())
    }

    pub fn cursors(&self, specs: &[CursorSpec]) -> Vec<StreamCursor> {
        specs
            .iter()
            .map(|spec| StreamCursor {
                stream_id: self.substitute(&spec.stream_id),
                epoch: self.substitute(&spec.epoch),
                after_sequence: spec.after_sequence,
            })
            .collect()
    }

    pub fn run_steps(&mut self, steps: &[Step], principal: &AuthenticatedPrincipal) {
        for step in steps {
            match step {
                Step::Publish {
                    publish,
                    count,
                    terminal,
                } => {
                    let run = publish.strip_prefix("run:").expect("publish run:<id>");
                    for index in 0..*count {
                        let last = index + 1 == *count;
                        self.rt.runtime.events_hub().publish_state(
                            StreamKind::Run,
                            run,
                            RUN_EVENT_V1,
                            json!({"type": "diagnostic", "message": format!("{index}")}),
                            *terminal && last,
                            &mut |_| {},
                        );
                    }
                }
                Step::Hold { hold } => {
                    let stream = self
                        .rt
                        .runtime
                        .events(
                            principal.clone(),
                            Subscription {
                                cursors: self.cursors(hold),
                            },
                        )
                        .expect("hold subscription");
                    self.held.push(stream);
                }
                Step::Write { write, content } => {
                    let path = PathBuf::from(self.substitute(write));
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).expect("dirs");
                    }
                    fs::write(path, content).expect("write");
                }
                Step::Sleep { sleep_ms } => std::thread::sleep(Duration::from_millis(*sleep_ms)),
            }
        }
    }
}

/// 비교용 정규화: `{event: {...}}` 또는 `{gap: {...}}`.
pub fn normalize_item(item: &EventItem) -> Value {
    match item {
        EventItem::Event { event } => json!({ "event": event }),
        EventItem::Gap { gap } => json!({ "gap": gap }),
    }
}

fn normalize_frame(frame: &EventFrame) -> Option<Value> {
    match frame {
        EventFrame::Event { event } => Some(json!({ "event": event })),
        EventFrame::Gap { gap } => Some(json!({ "gap": gap })),
        _ => None,
    }
}

pub async fn run_in_memory(fixture: &EventFixture) -> (Outcome, Context) {
    let principal = fixture.principal();
    let mut ctx = Context::new(fixture);
    ctx.run_steps(&fixture.setup, &principal);
    let subscription = Subscription {
        cursors: ctx.cursors(&fixture.subscribe),
    };
    let mut stream = match ctx.rt.runtime.events(principal.clone(), subscription) {
        Ok(stream) => stream,
        Err(fault) => return (Outcome::Fault(fault), ctx),
    };
    ctx.run_steps(&fixture.after, &principal);
    let mut items = Vec::new();
    for _ in 0..fixture.expect.items.len() {
        match tokio::time::timeout(ITEM_WAIT, stream.next()).await {
            Ok(Some(item)) => items.push(normalize_item(&item)),
            _ => break,
        }
    }
    let ended = matches!(
        tokio::time::timeout(QUIET_WAIT, stream.next()).await,
        Ok(None)
    );
    (Outcome::Items { items, ended }, ctx)
}

pub async fn run_ws(fixture: &EventFixture) -> (Outcome, Context) {
    let principal = fixture.principal();
    let mut ctx = Context::new(fixture);
    ctx.run_steps(&fixture.setup, &principal);
    let workbench: Arc<dyn Workbench> = ctx.rt.runtime.clone();
    let harness = Harness::spawn(workbench).await;
    let baseline = ctx.rt.runtime.events_hub().subscription_count();
    let mut ws = harness
        .subscribe(
            Harness::token_for(&principal),
            ctx.cursors(&fixture.subscribe),
        )
        .await;
    // 서버 쪽 구독이 등록될 때까지 기다린 뒤 `after`를 실행한다(또는 fault 프레임).
    let mut first = None;
    for _ in 0..200 {
        if ctx.rt.runtime.events_hub().subscription_count() > baseline {
            break;
        }
        if let Some(frame) = ws.next_frame(Duration::from_millis(10)).await {
            first = Some(frame);
            break;
        }
    }
    if let Some(EventFrame::Fault { fault }) = &first {
        return (Outcome::Fault(fault.clone()), ctx);
    }
    ctx.run_steps(&fixture.after, &principal);
    let mut items: Vec<Value> = first
        .as_ref()
        .and_then(normalize_frame)
        .into_iter()
        .collect();
    while items.len() < fixture.expect.items.len() {
        match ws.next_frame(ITEM_WAIT).await {
            Some(EventFrame::Fault { fault }) => return (Outcome::Fault(fault), ctx),
            Some(frame) => {
                if let Some(value) = normalize_frame(&frame) {
                    items.push(value);
                }
            }
            None => break,
        }
    }
    // WS는 "조용함"과 "닫힘"을 시간 안에서 구별하기 어렵다: 끝나야 하는 fixture에서만 닫힘을 확인한다.
    let ended = fixture.expect.end && ws.next_frame(QUIET_WAIT).await.is_none();
    drop(harness);
    (Outcome::Items { items, ended }, ctx)
}

/// 기대값(부분 일치)과 비교한다.
pub fn check(label: &str, fixture: &EventFixture, outcome: &Outcome, ctx: &Context) {
    let mut expected_fault = fixture.expect.fault.clone();
    match (outcome, expected_fault.take()) {
        (Outcome::Fault(fault), Some(expected)) => {
            let actual = serde_json::to_value(fault).unwrap();
            subset(label, &expected, &actual);
        }
        (Outcome::Items { items, ended }, None) => {
            assert_eq!(
                items.len(),
                fixture.expect.items.len(),
                "{label}: item count, got {items:#?}"
            );
            for (expected, actual) in fixture.expect.items.iter().zip(items) {
                let mut expected = expected.clone();
                substitute_value(&mut expected, ctx);
                subset(label, &expected, actual);
            }
            if fixture.expect.end {
                assert!(
                    *ended,
                    "{label}: stream should end after the expected items"
                );
            }
        }
        (other, expected) => panic!("{label}: expected fault {expected:?}, got {other:?}"),
    }
}

fn substitute_value(value: &mut Value, ctx: &Context) {
    match value {
        Value::String(text) => *text = ctx.substitute(text),
        Value::Array(items) => items
            .iter_mut()
            .for_each(|item| substitute_value(item, ctx)),
        Value::Object(map) => map
            .values_mut()
            .for_each(|item| substitute_value(item, ctx)),
        _ => {}
    }
}

fn subset(label: &str, expected: &Value, actual: &Value) {
    match (expected, actual) {
        (Value::Object(exp), Value::Object(act)) => {
            for (key, value) in exp {
                let actual = act
                    .get(key)
                    .unwrap_or_else(|| panic!("{label}: missing {key} in {actual}"));
                subset(label, value, actual);
            }
        }
        (Value::Array(exp), Value::Array(act)) => {
            assert_eq!(exp.len(), act.len(), "{label}: array length");
            for (e, a) in exp.iter().zip(act) {
                subset(label, e, a);
            }
        }
        (e, a) => assert_eq!(e, a, "{label}"),
    }
}

/// 두 경로 비교용: 시도·경로마다 다른 값(eventId·occurredAt·epoch·실제 임시 경로)을 지운다.
pub fn comparable(outcome: &Outcome, ctx: &Context) -> Value {
    match outcome {
        Outcome::Fault(fault) => json!({"fault": {"code": fault.code, "message": fault.message}}),
        Outcome::Items { items, .. } => {
            let mut items = Value::Array(items.clone());
            strip(&mut items, ctx);
            json!({ "items": items })
        }
    }
}

fn strip(value: &mut Value, ctx: &Context) {
    match value {
        Value::Object(map) => {
            for key in ["eventId", "occurredAt", "epoch"] {
                map.remove(key);
            }
            map.values_mut().for_each(|item| strip(item, ctx));
        }
        Value::Array(items) => items.iter_mut().for_each(|item| strip(item, ctx)),
        Value::String(text) => {
            let tmp = ctx.tmp.path().to_string_lossy().into_owned();
            let canonical = fs::canonicalize(ctx.tmp.path())
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !canonical.is_empty() {
                *text = text.replace(&canonical, "{{tmp}}");
            }
            *text = text.replace(&tmp, "{{tmp}}");
        }
        _ => {}
    }
}
