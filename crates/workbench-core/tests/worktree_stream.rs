//! 039 US3: worktree 변경 알림 스트림. 감시는 실제 경로별 참조 수로 공유되고(구독자 0→1에서 시작, 1→0에서 중지),
//! 경로 표기 차이는 같은 스트림으로 모이며, debounce 창 안의 변경은 알림 1회가 된다(SC-005).

use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use futures_util::StreamExt;
use serde_json::json;
use workbench_core::infrastructure::{
    event_hub::{EventHub, EventHubLimits, StartWatch},
    fs::worktree_watcher,
};
use workbench_protocol::{
    events::WORKTREE_CHANGED_V1, AuthenticatedPrincipal, EventItem, EventStream, FaultCode,
    StreamCursor, Subscription,
};

type Notify = Box<dyn Fn(serde_json::Value) + Send + Sync>;

/// 감시 시작·중지 횟수를 세는 가짜 감시. 알림 콜백을 붙잡아 테스트가 직접 발행한다.
#[derive(Default)]
struct FakeWatch {
    starts: AtomicUsize,
    stops: Arc<AtomicUsize>,
    notify: Mutex<Option<Notify>>,
}

struct StopCounter(Arc<AtomicUsize>);

impl Drop for StopCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

impl FakeWatch {
    fn start_watch(self: &Arc<Self>) -> StartWatch {
        let fake = Arc::clone(self);
        Arc::new(move |_path: &Path, notify: Notify| {
            fake.starts.fetch_add(1, Ordering::SeqCst);
            *fake.notify.lock().unwrap() = Some(notify);
            Ok(Box::new(StopCounter(Arc::clone(&fake.stops))) as Box<dyn Send>)
        })
    }

    fn fire(&self) {
        let notify = self.notify.lock().unwrap();
        (notify.as_ref().expect("watch started"))(json!({"kind": "file"}));
    }
}

#[allow(clippy::result_large_err)]
fn subscribe(
    hub: &Arc<EventHub>,
    path: &str,
) -> Result<EventStream, workbench_protocol::WorkbenchFault> {
    hub.subscribe(
        &AuthenticatedPrincipal::desktop(),
        Subscription {
            cursors: vec![StreamCursor {
                stream_id: format!("worktree:{path}"),
                epoch: "epoch-1".into(),
                after_sequence: 0,
            }],
        },
    )
}

async fn next_event(stream: &mut EventStream) -> workbench_protocol::EventEnvelope {
    match tokio::time::timeout(Duration::from_secs(3), stream.next()).await {
        Ok(Some(EventItem::Event { event })) => event,
        other => panic!("expected an event, got {other:?}"),
    }
}

#[tokio::test]
async fn watcher_starts_once_and_stops_when_the_last_subscriber_leaves() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().to_string_lossy().into_owned();
    let fake = Arc::new(FakeWatch::default());
    let hub = EventHub::with_watcher(
        "epoch-1",
        EventHubLimits::default(),
        Some(fake.start_watch()),
    );

    let first = subscribe(&hub, &path).unwrap();
    assert_eq!(
        (fake.starts.load(Ordering::SeqCst), hub.watcher_count()),
        (1, 1)
    );
    let second = subscribe(&hub, &path).unwrap();
    assert_eq!(
        (fake.starts.load(Ordering::SeqCst), hub.watcher_count()),
        (1, 1)
    );
    drop(first);
    assert_eq!(
        (fake.stops.load(Ordering::SeqCst), hub.watcher_count()),
        (0, 1)
    );
    drop(second);
    assert_eq!(
        (fake.stops.load(Ordering::SeqCst), hub.watcher_count()),
        (1, 0)
    );
    assert_eq!(fake.starts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn path_spellings_share_one_stream() {
    let tmp = tempfile::tempdir().unwrap();
    let real = tmp.path().join("repo");
    std::fs::create_dir(&real).unwrap();
    let link = tmp.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let fake = Arc::new(FakeWatch::default());
    let hub = EventHub::with_watcher(
        "epoch-1",
        EventHubLimits::default(),
        Some(fake.start_watch()),
    );

    let mut plain = subscribe(&hub, &real.to_string_lossy()).unwrap();
    let mut slash = subscribe(&hub, &format!("{}/", real.to_string_lossy())).unwrap();
    let mut linked = subscribe(&hub, &link.to_string_lossy()).unwrap();
    assert_eq!(
        (fake.starts.load(Ordering::SeqCst), hub.watcher_count()),
        (1, 1)
    );

    fake.fire();
    let events = [
        next_event(&mut plain).await,
        next_event(&mut slash).await,
        next_event(&mut linked).await,
    ];
    let canonical = std::fs::canonicalize(&real).unwrap();
    for event in &events {
        assert_eq!(
            event.stream_id,
            format!("worktree:{}", canonical.to_string_lossy())
        );
        assert_eq!(event.schema, WORKTREE_CHANGED_V1);
    }
}

#[tokio::test]
async fn missing_path_is_not_found_with_todays_message() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("missing").to_string_lossy().into_owned();
    let fake = Arc::new(FakeWatch::default());
    let hub = EventHub::with_watcher(
        "epoch-1",
        EventHubLimits::default(),
        Some(fake.start_watch()),
    );

    let fault = subscribe(&hub, &missing).unwrap_err();
    assert_eq!(fault.code, FaultCode::NotFound);
    assert_eq!(
        fault.message,
        format!("Cannot watch missing worktree path: {missing}")
    );
    assert_eq!(
        (fake.starts.load(Ordering::SeqCst), hub.watcher_count()),
        (0, 0)
    );
    assert_eq!(hub.subscription_count(), 0);
}

#[tokio::test]
async fn burst_of_changes_in_a_git_worktree_is_one_notification() {
    let tmp = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(tmp.path())
        .status()
        .expect("git");
    assert!(status.success());
    let hub = EventHub::with_watcher(
        "epoch-1",
        EventHubLimits::default(),
        Some(worktree_watcher::start_watch()),
    );
    let mut stream = subscribe(&hub, &tmp.path().to_string_lossy()).unwrap();
    // FSEvents가 감시를 붙일 시간을 준다.
    tokio::time::sleep(Duration::from_millis(200)).await;

    for name in ["a.txt", "b.txt", "c.txt"] {
        std::fs::write(tmp.path().join(name), name).unwrap();
    }
    let first = next_event(&mut stream).await;
    assert_eq!(first.body["kind"], "file");
    assert!(
        tokio::time::timeout(Duration::from_millis(1_200), stream.next())
            .await
            .is_err(),
        "a burst inside the debounce window must be one notification"
    );
}
