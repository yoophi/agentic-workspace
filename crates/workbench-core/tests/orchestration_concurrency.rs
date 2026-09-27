//! 041 research R1: orchestration 저장소는 파일 하나에 모든 작업 영역을 담으므로, 읽기-수정-쓰기는 저장소 전체
//! 경계 하나로 직렬화되어야 한다. 서로 다른 작업 영역의 동시 변경도, 같은 작업 영역의 동시 변경도 잃지 않는다.
//! 같은 파일을 여는 저장소 인스턴스가 여럿이어도(과도기 AW는 호출마다 새로 연다) 같은 경계를 쓴다.

use std::{sync::Arc, thread};

use workbench_core::{
    domain::agent_orchestration::OrchestrationSession,
    infrastructure::fs::orchestration_store::JsonOrchestrationRepository,
    ports::orchestration_repository::{OrchestrationRepository, OrchestrationTransaction},
};

const THREADS: usize = 8;
const PER_THREAD: usize = 25;

fn seeded(dir: &tempfile::TempDir, ids: &[&str]) -> std::path::PathBuf {
    let path = dir.path().join("orchestration-sessions.json");
    let store = JsonOrchestrationRepository::from_path(path.clone());
    let mut tx = store.begin().unwrap();
    for (index, id) in ids.iter().enumerate() {
        tx.sessions().push(OrchestrationSession::new(
            *id,
            format!("/repo/{index}"),
            format!("seed-{index}"),
            "2026-09-27T00:00:00Z",
        ));
    }
    tx.commit().unwrap();
    path
}

fn bump(store: &JsonOrchestrationRepository, id: &str) {
    let mut tx = store.begin().unwrap();
    tx.sessions()
        .iter_mut()
        .find(|s| s.id == id)
        .unwrap()
        .revision += 1;
    tx.commit().unwrap();
}

fn revision_of(store: &JsonOrchestrationRepository, id: &str) -> u64 {
    store
        .snapshot()
        .unwrap()
        .iter()
        .find(|s| s.id == id)
        .unwrap()
        .revision
}

#[test]
fn concurrent_updates_to_different_workspaces_are_all_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded(&dir, &["w1", "w2", "w3"]);
    let handles: Vec<_> = (0..THREADS)
        .map(|thread_index| {
            let path = path.clone();
            thread::spawn(move || {
                // 스레드마다 별도 인스턴스 — 과도기 AW처럼 같은 파일을 여러 인스턴스가 연다.
                let store = JsonOrchestrationRepository::from_path(path);
                let id = ["w1", "w2", "w3"][thread_index % 3];
                for _ in 0..PER_THREAD {
                    bump(&store, id);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let store = JsonOrchestrationRepository::from_path(path);
    let expected =
        |slot: usize| ((0..THREADS).filter(|t| t % 3 == slot).count() * PER_THREAD) as u64;
    assert_eq!(revision_of(&store, "w1"), expected(0));
    assert_eq!(revision_of(&store, "w2"), expected(1));
    assert_eq!(revision_of(&store, "w3"), expected(2));
    assert!(expected(0) + expected(1) + expected(2) >= 100);
}

#[test]
fn concurrent_updates_to_one_workspace_are_all_kept() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded(&dir, &["w1"]);
    let store = Arc::new(JsonOrchestrationRepository::from_path(path));
    let handles: Vec<_> = (0..THREADS)
        .map(|_| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                for _ in 0..PER_THREAD {
                    bump(&store, "w1");
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    assert_eq!(revision_of(&store, "w1"), (THREADS * PER_THREAD) as u64);
}

/// 회귀 검출: 경계 없이 같은 흐름(읽기 → 수정 → 쓰기)을 돌리면 변경을 잃는다. 위 두 테스트가 lock 덕분에
/// 통과한다는 것을 보인다.
#[test]
fn without_the_store_boundary_updates_are_lost() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded(&dir, &["w1", "w2"]);
    let handles: Vec<_> = (0..THREADS)
        .map(|thread_index| {
            let path = path.clone();
            thread::spawn(move || {
                let store = JsonOrchestrationRepository::from_path(path);
                let id = if thread_index % 2 == 0 { "w1" } else { "w2" };
                for _ in 0..PER_THREAD {
                    // 경계 없이 같은 파일에 동시에 쓰면 임시 파일이 부딪혀 쓰기 자체가 실패하기도 한다 — 그것도 손실이다.
                    let _ = store.update_unlocked_for_test(|sessions| {
                        let session = sessions.iter_mut().find(|s| s.id == id).unwrap();
                        session.revision += 1;
                        thread::yield_now();
                        Ok(())
                    });
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    // 경계가 없으면 파일이 깨져 읽기 자체가 실패할 수도 있다 — 그 경우도 변경을 잃은 것이다.
    let total = JsonOrchestrationRepository::from_path(path)
        .snapshot()
        .map(|sessions| sessions.iter().map(|s| s.revision).sum::<u64>())
        .unwrap_or(0);
    assert!(
        total < (THREADS * PER_THREAD) as u64,
        "expected lost updates without the boundary, got all {total}"
    );
}
