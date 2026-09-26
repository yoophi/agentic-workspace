//! SC-001 보조 측정. CI에서는 `#[ignore]`이며 quickstart §1에서 수동 실행한다:
//! `cargo test -p workbench-core --test list_latency -- --ignored --nocapture`

mod support;

use std::time::{Duration, Instant};

use serde_json::json;
use support::{fixtures::Seed, TestRuntime};
use workbench_protocol::{AuthenticatedPrincipal, CallRequest, OperationId, Workbench};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "latency measurement; run manually with --ignored --nocapture"]
async fn project_list_p95_under_5ms_with_50_projects() {
    let runtime = TestRuntime::new();
    let seed = Seed {
        projects: (0..50)
            .map(|index| {
                json!({
                    "id": format!("project-{index}"),
                    "name": format!("Project {index}"),
                    "workingDirectory": format!("/tmp/project-{index}"),
                    "description": null
                })
            })
            .collect(),
        ..Default::default()
    };
    support::fixtures::apply_seed(&runtime.paths, &seed);

    let mut samples = Vec::with_capacity(1000);
    for _ in 0..1000 {
        let request = CallRequest::query(OperationId::ProjectList, json!({}));
        let started = Instant::now();
        let reply = runtime
            .runtime
            .call(AuthenticatedPrincipal::desktop(), request)
            .await
            .unwrap();
        samples.push(started.elapsed());
        assert_eq!(reply.output().unwrap().as_array().unwrap().len(), 50);
    }
    samples.sort();
    let p50 = samples[samples.len() / 2];
    let p95 = samples[(samples.len() as f64 * 0.95) as usize - 1];
    let max = *samples.last().unwrap();
    println!("project.list in-memory latency: p50={p50:?} p95={p95:?} max={max:?}");
    assert!(p95 < Duration::from_millis(5), "p95 {p95:?} exceeds 5ms");
}

// ---- 038 SC-001: Workbench 경유 지연 vs 직접 호출(이전 AW 경로에 해당) ----
// 이전 command는 서비스·어댑터를 blocking pool에서 직접 불렀다. 같은 서비스를 직접 부른 값을 기준선으로 두고,
// Workbench 경유(authorize·역직렬화·lock·ledger) 값과의 p95 차이가 50ms 이내인지 본다.

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[(samples.len() as f64 * 0.95) as usize - 1]
}

fn report(label: &str, via: Duration, direct: Duration) {
    let delta = via.saturating_sub(direct);
    println!("{label}: workbench p95={via:?} direct p95={direct:?} delta={delta:?}");
    assert!(
        delta < Duration::from_millis(50),
        "{label}: delta {delta:?} exceeds 50ms"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "latency measurement; run manually with --ignored --nocapture"]
async fn saved_prompt_list_overhead_under_50ms() {
    use workbench_core::{
        application::saved_prompt_service,
        infrastructure::json_saved_prompt_repository::JsonSavedPromptRepository,
    };

    let runtime = TestRuntime::new();
    let seed = Seed {
        saved_prompts: (0..50)
            .map(|index| json!({ "id": format!("saved-prompt-{index}"), "label": format!("L{index}"), "prompt": "p" }))
            .collect(),
        ..Default::default()
    };
    support::fixtures::apply_seed(&runtime.paths, &seed);

    let mut via = Vec::with_capacity(100);
    for _ in 0..100 {
        let started = Instant::now();
        let reply = runtime
            .call(CallRequest::query(OperationId::SavedPromptList, json!({})))
            .await
            .unwrap();
        via.push(started.elapsed());
        assert_eq!(reply.output().unwrap().as_array().unwrap().len(), 50);
    }
    let repository = JsonSavedPromptRepository::new(&runtime.paths);
    let mut direct = Vec::with_capacity(100);
    for _ in 0..100 {
        let started = Instant::now();
        let prompts = saved_prompt_service::list_saved_prompts(&repository).unwrap();
        direct.push(started.elapsed());
        assert_eq!(prompts.len(), 50);
    }
    report("savedPrompt.list", p95(via), p95(direct));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "latency measurement; run manually with --ignored --nocapture"]
async fn goal_record_progress_overhead_under_50ms() {
    use workbench_core::{
        application::goal_service,
        domain::goal::{GoalDraft, GoalProgressUpdate},
        infrastructure::json_goal_repository::JsonGoalRepository,
    };

    let runtime = TestRuntime::new();
    runtime
        .call(support::command_request(
            OperationId::GoalCreate,
            "create",
            json!({ "workingDirectory": "/repo/wt", "objective": "Ship" }),
        ))
        .await
        .unwrap();
    let mut via = Vec::with_capacity(100);
    for index in 0..100 {
        let request = support::command_request(
            OperationId::GoalRecordProgress,
            &format!("p{index}"),
            json!({ "workingDirectory": "/repo/wt", "tokensUsed": index, "timeUsedSeconds": 1 }),
        );
        let started = Instant::now();
        runtime.call(request).await.unwrap();
        via.push(started.elapsed());
    }

    // 기준선: 별도 데이터 디렉터리에서 같은 서비스를 직접(ledger 없이) 부른다.
    let baseline_dir = tempfile::tempdir().unwrap();
    let baseline_paths =
        workbench_core::infrastructure::data_paths::DataPaths::new(baseline_dir.path());
    baseline_paths.ensure_dirs().unwrap();
    let repository = JsonGoalRepository::new(&baseline_paths);
    goal_service::create_goal(
        &repository,
        GoalDraft {
            working_directory: "/repo/wt".into(),
            objective: "Ship".into(),
            token_budget: None,
        },
    )
    .unwrap();
    let mut direct = Vec::with_capacity(100);
    for index in 0..100 {
        let started = Instant::now();
        goal_service::record_goal_progress(
            &repository,
            "/repo/wt".into(),
            GoalProgressUpdate {
                tokens_used: index,
                time_used_seconds: 1,
            },
        )
        .unwrap();
        direct.push(started.elapsed());
    }
    report("goal.recordProgress", p95(via), p95(direct));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "latency measurement; run manually with --ignored --nocapture"]
async fn worktree_list_files_overhead_under_50ms() {
    use workbench_core::{
        domain::worktree_file::WorktreeFileListScope,
        infrastructure::fs::worktree_file_provider::FsWorktreeFileProvider,
        ports::worktree_file_provider::WorktreeFileProvider,
    };

    let runtime = TestRuntime::new();
    let seed: support::git_repo::GitRepoSeed = serde_json::from_value(json!({
        "commits": [{
            "message": "init",
            "files": (0..200)
                .map(|index| (format!("src/dir{}/file{index}.rs", index % 10), json!("fn f() {}\n")))
                .collect::<serde_json::Map<_, _>>()
        }]
    }))
    .unwrap();
    let parent = runtime.dir.path().join("repos");
    std::fs::create_dir_all(&parent).unwrap();
    let repo = support::git_repo::build(&seed, &parent);
    let root = repo.root.to_str().unwrap().to_owned();

    let mut via = Vec::with_capacity(100);
    for _ in 0..100 {
        let started = Instant::now();
        let reply = runtime
            .call(CallRequest::query(
                OperationId::WorktreeListFiles,
                json!({ "workingDirectory": root }),
            ))
            .await
            .unwrap();
        via.push(started.elapsed());
        assert_eq!(reply.output().unwrap().as_array().unwrap().len(), 211);
    }
    let mut direct = Vec::with_capacity(100);
    for _ in 0..100 {
        let started = Instant::now();
        let entries = FsWorktreeFileProvider
            .list_files(&root, &WorktreeFileListScope::default())
            .unwrap();
        direct.push(started.elapsed());
        assert_eq!(entries.len(), 211);
    }
    report("worktree.listFiles (200 files)", p95(via), p95(direct));
}
