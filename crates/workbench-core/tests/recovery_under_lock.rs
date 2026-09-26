//! Codex 리뷰 반영(research R14): 손상된 primary의 `.bak` 복구는 aggregate lock 안에서만 일어나므로,
//! 동시에 들어온 create의 결과를 늦은 복구가 덮어쓰지 않는다. 읽기 경로는 파일을 쓰지 않는다.

mod support;

use std::{collections::BTreeSet, fs, sync::Arc, time::Duration};

use serde_json::json;
use support::{create_request, list_request, TestRuntime};
use workbench_core::ports::operation_ledger::LedgerState;
use workbench_protocol::FaultCode;

fn seed_two_with_backup(rt: &TestRuntime) -> Vec<String> {
    let seed = json!([
        { "id": "project-seed-1", "name": "Seed One", "workingDirectory": "/tmp/s1", "description": null },
        { "id": "project-seed-2", "name": "Seed Two", "workingDirectory": "/tmp/s2", "description": null }
    ]);
    let bytes = serde_json::to_vec_pretty(&seed).unwrap();
    fs::write(rt.paths.projects_file(), &bytes).unwrap();
    fs::write(
        rt.paths.projects_file().with_file_name("projects.json.bak"),
        &bytes,
    )
    .unwrap();
    // primary 손상
    fs::write(rt.paths.projects_file(), b"{ this is not json").unwrap();
    vec!["project-seed-1".into(), "project-seed-2".into()]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn recovery_never_overwrites_concurrent_creates() {
    let rt = Arc::new(TestRuntime::new());
    let seeds = seed_two_with_backup(&rt);
    rt.runtime
        .coordinator()
        .set_recovery_delay(Some(Duration::from_millis(200)));

    let mut tasks = Vec::new();
    {
        let rt = Arc::clone(&rt);
        tasks.push(tokio::spawn(async move {
            rt.call(list_request()).await.map(|_| ())
        }));
    }
    for index in 0..5 {
        let rt = Arc::clone(&rt);
        tasks.push(tokio::spawn(async move {
            rt.call(create_request(
                &format!("c{index}"),
                &format!("C{index}"),
                "/tmp/c",
            ))
            .await
            .map(|_| ())
        }));
    }
    for task in tasks {
        task.await
            .unwrap()
            .expect("read and creates all succeed after recovery");
    }

    let projects = rt.projects();
    assert_eq!(projects.len(), 7, "복구된 2개 + 생성 5개");
    let ids: BTreeSet<String> = projects
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_owned())
        .collect();
    for seed in &seeds {
        assert!(
            ids.contains(seed),
            "복구된 프로젝트 {seed}가 남아 있어야 한다"
        );
    }
    let applied = rt
        .runtime
        .ledger()
        .applied_resource_ids("projects")
        .unwrap();
    assert_eq!(applied.len(), 5);
    for id in applied {
        assert!(
            ids.contains(&id),
            "applied {id}가 파일에 없다 — 복구가 변경을 덮어썼다"
        );
    }
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Applied)
            .unwrap(),
        5
    );
    assert_eq!(rt.runtime.coordinator().revision(), 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovery_works_without_delay_hook_too() {
    let rt = TestRuntime::new();
    seed_two_with_backup(&rt);
    let reply = rt.call(list_request()).await.unwrap();
    assert_eq!(reply.output().unwrap().as_array().unwrap().len(), 2);
    let reply = rt.call(create_request("c1", "C1", "/tmp/c")).await.unwrap();
    assert_eq!(reply.revision(), Some(1));
    assert_eq!(rt.projects().len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_path_never_writes_when_no_backup_exists() {
    let rt = TestRuntime::new();
    fs::write(rt.paths.projects_file(), b"{ corrupt").unwrap();
    let before = fs::read(rt.paths.projects_file()).unwrap();

    let fault = rt.call(list_request()).await.unwrap_err();
    assert_eq!(fault.code, FaultCode::Unavailable, "{fault}");
    assert!(fault.message.contains("backup recovery failed"), "{fault}");
    assert_eq!(
        fs::read(rt.paths.projects_file()).unwrap(),
        before,
        "읽기가 파일을 바꾸지 않았다"
    );

    let fault = rt
        .call(create_request("c1", "C1", "/tmp/c"))
        .await
        .unwrap_err();
    assert_eq!(fault.code, FaultCode::Unavailable);
    assert_eq!(
        rt.runtime
            .ledger()
            .count_by_state(LedgerState::Failed)
            .unwrap(),
        1
    );
    assert_eq!(rt.runtime.coordinator().revision(), 0);
}
