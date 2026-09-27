//! 039 SC-003·SC-004: 이벤트 구독 fixture(`crates/workbench-protocol/fixtures/events`)를 in-memory와 테스트 WebSocket
//! 두 경로로 실행하고, 각각 기대값과 맞는지·서로 같은지 확인한다.

mod support;

use support::event_fixtures::{self, check, comparable, run_in_memory, run_ws};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn every_event_fixture_matches_on_in_memory_and_ws_paths() {
    let filter = std::env::var("WORKBENCH_FIXTURE_FILTER").ok();
    let all: Vec<_> = event_fixtures::load_all()
        .into_iter()
        .filter(|fixture| {
            filter
                .as_deref()
                .is_none_or(|prefix| fixture.name.starts_with(prefix))
        })
        .collect();
    assert!(filter.is_some() || all.len() >= 15, "found {}", all.len());
    for fixture in &all {
        let (memory, memory_ctx) = run_in_memory(fixture).await;
        check(
            &format!("{} [in-memory]", fixture.name),
            fixture,
            &memory,
            &memory_ctx,
        );
        if fixture.in_memory_only {
            continue;
        }
        let (ws, ws_ctx) = run_ws(fixture).await;
        check(&format!("{} [ws]", fixture.name), fixture, &ws, &ws_ctx);
        assert_eq!(
            comparable(&ws, &ws_ctx),
            comparable(&memory, &memory_ctx),
            "{}: ws and in-memory diverge",
            fixture.name
        );
    }
}
