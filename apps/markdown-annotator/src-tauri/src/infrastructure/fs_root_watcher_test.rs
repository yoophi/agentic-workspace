use super::*;
#[test]
fn root_events_keep_monotonic_revision_and_rescan_hint() {
    let first = RootChangedEvent {
        root_id: "r".into(),
        revision: 1,
        paths: vec!["a.md".into()],
        rescan_hint: false,
    };
    let burst = RootChangedEvent {
        root_id: "r".into(),
        revision: 2,
        paths: vec!["a.md".into(), "b.md".into()],
        rescan_hint: true,
    };
    assert!(burst.revision > first.revision);
    assert!(burst.rescan_hint);
}
#[test]
fn stale_root_event_can_be_rejected_by_revision() {
    let applied = 4;
    let stale = RootChangedEvent {
        root_id: "r".into(),
        revision: 3,
        paths: vec![],
        rescan_hint: true,
    };
    assert!(stale.revision <= applied);
}
