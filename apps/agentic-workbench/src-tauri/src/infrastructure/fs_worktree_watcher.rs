use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

use crate::infrastructure::{WORKSPACE_EXCLUDED_DIRS, perf_log};

const WORKTREE_EVENT_DEBOUNCE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeChangedEvent {
    pub working_directory: String,
    pub changed_path: String,
    pub kind: WorktreeChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeChangeKind {
    File,
    Git,
}

/// 원시 이벤트 분류 결과. Ignored는 발행하지 않는다(specs/007 research R3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventClass {
    Ignored,
    File,
    Git,
}

pub struct WorktreeWatchHandle {
    _watcher: RecommendedWatcher,
}

pub fn watch_worktree(
    working_directory: String,
    notify: impl Fn(WorktreeChangedEvent) + Send + Sync + 'static,
) -> Result<WorktreeWatchHandle, String> {
    let worktree_root = PathBuf::from(&working_directory);
    if !worktree_root.exists() {
        return Err(format!(
            "Cannot watch missing worktree path: {}",
            worktree_root.display()
        ));
    }

    let watched_paths = watched_paths_for_worktree(&worktree_root);
    if watched_paths.is_empty() {
        return Err(format!(
            "No existing watch paths for worktree: {}",
            worktree_root.display()
        ));
    }

    let git_paths = git_metadata_paths(&worktree_root);
    let worktree_for_event = working_directory.clone();
    let (sender, receiver) = mpsc::channel::<(WorktreeChangeKind, String)>();

    // trailing debounce thread: 마지막 원시 이벤트 후 debounce 경과 시 1회 발행해
    // 이벤트 폭주를 줄이고 창 내 마지막 변경 유실을 막는다. watcher(=sender)가
    // drop되면 channel disconnect로 thread도 종료된다.
    thread::spawn(move || {
        run_debounce_loop(receiver, WORKTREE_EVENT_DEBOUNCE, move |change| {
            perf_log::log_watcher("emit", &format!("kind={:?}", change.kind));
            notify(WorktreeChangedEvent {
                working_directory: worktree_for_event.clone(),
                changed_path: change.changed_path,
                kind: change.kind,
            });
        });
    });

    let fallback_path = working_directory.clone();
    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| match result {
            Ok(event) => {
                let (class, representative) = classify_event(&event.paths, &git_paths);
                let kind = match class {
                    EventClass::Ignored => return,
                    EventClass::File => WorktreeChangeKind::File,
                    EventClass::Git => WorktreeChangeKind::Git,
                };
                let changed_path = representative
                    .and_then(|path| path.to_str())
                    .unwrap_or(&fallback_path)
                    .to_string();

                let _ = sender.send((kind, changed_path));
            }
            Err(error) => {
                eprintln!("Worktree watcher event failed: {error}");
            }
        },
        Config::default(),
    )
    .map_err(|error| format!("Failed to start worktree watcher: {error}"))?;

    for path in watched_paths {
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .map_err(|error| {
                format!("Failed to watch worktree path {}: {error}", path.display())
            })?;
    }

    Ok(WorktreeWatchHandle { _watcher: watcher })
}

/// debounce된 발행 대상. working_directory는 watch_worktree가 채운다.
pub(crate) struct DebouncedChange {
    pub kind: WorktreeChangeKind,
    pub changed_path: String,
}

pub(crate) fn run_debounce_loop(
    receiver: Receiver<(WorktreeChangeKind, String)>,
    debounce: Duration,
    emit: impl Fn(DebouncedChange),
) {
    while let Ok((kind, changed_path)) = receiver.recv() {
        let mut pending = DebouncedChange { kind, changed_path };

        loop {
            match receiver.recv_timeout(debounce) {
                Ok((kind, changed_path)) => {
                    // git 이벤트가 하나라도 있으면 병합 결과는 git이다.
                    pending = DebouncedChange {
                        kind: if pending.kind == WorktreeChangeKind::Git {
                            WorktreeChangeKind::Git
                        } else {
                            kind
                        },
                        changed_path,
                    };
                }
                Err(RecvTimeoutError::Timeout) => {
                    emit(pending);
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    emit(pending);
                    return;
                }
            }
        }
    }
}

/// 이벤트 분류와 대표 경로(분류를 결정한 첫 경로)를 한 번의 순회로 계산한다.
pub(crate) fn classify_event<'a>(
    paths: &'a [PathBuf],
    git_paths: &[PathBuf],
) -> (EventClass, Option<&'a PathBuf>) {
    let mut class = EventClass::Ignored;
    let mut representative: Option<&PathBuf> = None;

    for path in paths {
        if is_git_metadata_path(path, git_paths) {
            if is_significant_git_change(path) {
                return (EventClass::Git, Some(path));
            }
            continue;
        }

        if class == EventClass::Ignored && !is_excluded_workspace_path(path) {
            class = EventClass::File;
            representative = Some(path);
        }
    }

    (class, representative.or_else(|| paths.first()))
}

fn is_git_metadata_path(path: &Path, git_paths: &[PathBuf]) -> bool {
    git_paths
        .iter()
        .any(|git_path| path == git_path || path.starts_with(git_path))
}

/// `.git` 내부 변화 중 commit 이력/브랜치에 의미 있는 것만 git 이벤트로 취급한다.
/// index·*.lock·FETCH_HEAD·logs는 status 실행이나 fetch만으로도 바뀌므로 무시해
/// 주기적 status와의 되먹임을 차단한다(specs/007 research R3).
fn is_significant_git_change(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    if name.ends_with(".lock") {
        return false;
    }

    // 단일 순회, 할당 없음: watcher 콜백은 이벤트 폭주 구간의 hot loop다.
    let mut has_refs = false;
    for component in path.components() {
        let component = component.as_os_str();
        if component == "logs" {
            return false;
        }
        if component == "refs" {
            has_refs = true;
        }
    }
    if has_refs {
        return true;
    }

    matches!(name, "HEAD" | "MERGE_HEAD" | "packed-refs")
}

fn is_excluded_workspace_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|name| WORKSPACE_EXCLUDED_DIRS.contains(&name))
    })
}

fn watched_paths_for_worktree(worktree_root: &Path) -> Vec<PathBuf> {
    let mut paths = vec![worktree_root.to_path_buf()];

    for git_path in git_metadata_paths(worktree_root) {
        if git_path.exists() && !paths.contains(&git_path) {
            paths.push(git_path);
        }
    }

    paths
}

fn git_metadata_paths(worktree_root: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    let dot_git = worktree_root.join(".git");
    if dot_git.exists() {
        paths.push(dot_git);
    }

    for argument in ["--git-dir", "--git-common-dir"] {
        if let Some(path) = git_rev_parse_path(worktree_root, argument)
            && !paths.contains(&path)
        {
            paths.push(path);
        }
    }

    paths
}

fn git_rev_parse_path(worktree_root: &Path, argument: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree_root)
        .args(["rev-parse", argument])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }

    let path = PathBuf::from(value);
    Some(if path.is_absolute() {
        path
    } else {
        worktree_root.join(path)
    })
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread, time::Duration};

    use super::{EventClass, WorktreeChangeKind, classify_event, run_debounce_loop};

    fn git_paths() -> Vec<std::path::PathBuf> {
        vec!["/repo/.git".into()]
    }

    // (a) 파일 목록 화면과 동일한 제외 목록(EXCLUDED_DIRS) 전체가 무시된다.
    #[test]
    fn ignores_all_excluded_directories() {
        for dir in [
            "node_modules",
            "target",
            "dist",
            ".next",
            "build",
            "coverage",
            ".turbo",
        ] {
            let (class, _) =
                classify_event(&[format!("/repo/{dir}/output.js").into()], &git_paths());
            assert_eq!(class, EventClass::Ignored, "{dir} should be ignored");
        }

        assert_eq!(
            classify_event(&["/repo/src/main.ts".into()], &git_paths()).0,
            EventClass::File,
        );
        // 무시 대상과 유효 변경이 섞이면 유효 변경을 따르고, 대표 경로는 유효 변경이다.
        let mixed_paths: Vec<std::path::PathBuf> =
            vec!["/repo/dist/output.js".into(), "/repo/src/main.ts".into()];
        let (class, representative) = classify_event(&mixed_paths, &git_paths());
        assert_eq!(class, EventClass::File);
        assert_eq!(
            representative.map(|p| p.to_str().unwrap()),
            Some("/repo/src/main.ts")
        );
    }

    // (b) .git 내부 이벤트 세분화: index/*.lock/FETCH_HEAD 단독 변화는 미발행,
    // HEAD/refs/packed-refs 변화는 git 이벤트.
    #[test]
    fn classifies_git_metadata_events() {
        for insignificant in [
            "/repo/.git/index",
            "/repo/.git/index.lock",
            "/repo/.git/HEAD.lock",
            "/repo/.git/FETCH_HEAD",
        ] {
            assert_eq!(
                classify_event(&[insignificant.into()], &git_paths()).0,
                EventClass::Ignored,
                "{insignificant} alone should not emit",
            );
        }

        for significant in [
            "/repo/.git/HEAD",
            "/repo/.git/refs/heads/main",
            "/repo/.git/packed-refs",
            "/repo/.git/MERGE_HEAD",
        ] {
            assert_eq!(
                classify_event(&[significant.into()], &git_paths()).0,
                EventClass::Git,
                "{significant} should emit a git event",
            );
        }
    }

    // (c) trailing debounce: 창 안의 마지막 이벤트가 유실되지 않고 1회로 병합 발행된다.
    #[test]
    fn debounce_emits_last_event_after_quiet_period() {
        let (sender, receiver) = mpsc::channel();
        let (emitted_sender, emitted_receiver) = mpsc::channel();
        let handle = thread::spawn(move || {
            run_debounce_loop(receiver, Duration::from_millis(50), move |event| {
                emitted_sender
                    .send(event)
                    .expect("test channel should accept event");
            });
        });

        sender
            .send((WorktreeChangeKind::File, "/repo/src/a.ts".to_owned()))
            .expect("send should succeed");
        thread::sleep(Duration::from_millis(5));
        sender
            .send((WorktreeChangeKind::Git, "/repo/.git/HEAD".to_owned()))
            .expect("send should succeed");

        let event = emitted_receiver
            .recv_timeout(Duration::from_millis(500))
            .expect("debounced event should be emitted");
        assert!(
            matches!(event.kind, WorktreeChangeKind::Git),
            "git kind should win the merge"
        );
        assert_eq!(event.changed_path, "/repo/.git/HEAD");
        assert!(
            emitted_receiver
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "only one merged event should be emitted",
        );

        drop(sender);
        handle.join().expect("debounce thread should exit");
    }
}
