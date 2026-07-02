use std::{fs, path::Path, process::Command};

use crate::domain::{
    git_worktree::{GitWorktree, GitWorktreeCreateDraft, GitWorktreeStatus},
    git_worktree_provider::GitWorktreeProvider,
};

pub struct GitCliWorktreeProvider;

#[derive(Default)]
struct WorktreeRecord {
    path: String,
    head: Option<String>,
    branch: Option<String>,
    prune_reason: Option<String>,
}

impl GitWorktreeProvider for GitCliWorktreeProvider {
    fn list_worktrees(
        &self,
        working_directory: &str,
        include_status: bool,
    ) -> Result<Vec<GitWorktree>, String> {
        let output = Command::new("git")
            .args(["-C", working_directory, "worktree", "list", "--porcelain"])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Failed to read git output: {error}"))?;

        parse_worktree_records(&stdout)
            .into_iter()
            .enumerate()
            .map(|(index, record)| self.to_worktree(record, index == 0, include_status))
            .collect()
    }

    fn create_worktree(
        &self,
        working_directory: &str,
        draft: GitWorktreeCreateDraft,
    ) -> Result<(), String> {
        if let Some(parent) = Path::new(&draft.path).parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Failed to create worktree parent directory: {error}"))?;
        }

        let mut command = Command::new("git");
        command.args(["-C", working_directory, "worktree", "add"]);

        if let Some(branch) = &draft.branch {
            command.args(["-b", branch]);
        }

        command.arg(&draft.path);

        if let Some(reference) = &draft.reference {
            command.arg(reference);
        }

        run_git_command(command, "Failed to create git worktree")
    }

    fn delete_worktree(&self, working_directory: &str, path: &str) -> Result<(), String> {
        // 삭제 안전성 판단에는 dirty 여부가 필요하므로 status를 포함해 조회한다.
        let worktree = self
            .list_worktrees(working_directory, true)?
            .into_iter()
            .find(|worktree| worktree.path == path)
            .ok_or_else(|| "Git worktree not found.".to_owned())?;

        match worktree.status {
            GitWorktreeStatus::Dirty => {
                Err("Worktree has changes and cannot be deleted.".to_owned())
            }
            GitWorktreeStatus::Unknown => {
                Err("Worktree status is not resolved yet and cannot be deleted.".to_owned())
            }
            GitWorktreeStatus::Prunable => {
                let mut command = Command::new("git");
                command.args(["-C", working_directory, "worktree", "prune"]);
                run_git_command(command, "Failed to prune git worktree")
            }
            GitWorktreeStatus::Clean => {
                let mut command = Command::new("git");
                command.args(["-C", working_directory, "worktree", "remove", path]);
                run_git_command(command, "Failed to remove git worktree")
            }
        }
    }
}

impl GitCliWorktreeProvider {
    fn to_worktree(
        &self,
        record: WorktreeRecord,
        is_main_worktree: bool,
        include_status: bool,
    ) -> Result<GitWorktree, String> {
        // prunable 판정은 porcelain 출력만으로 가능하므로 include_status와 무관하게 유지한다.
        let status = if record.prune_reason.is_some() {
            GitWorktreeStatus::Prunable
        } else if !include_status {
            GitWorktreeStatus::Unknown
        } else if has_changes(&record.path)? {
            GitWorktreeStatus::Dirty
        } else {
            GitWorktreeStatus::Clean
        };
        // Unknown은 clean 여부가 미확정이므로 삭제를 허용하지 않는다.
        let can_delete = status == GitWorktreeStatus::Prunable
            || (status == GitWorktreeStatus::Clean && !is_main_worktree);

        Ok(GitWorktree {
            path: record.path,
            head: record.head,
            branch: record.branch,
            prune_reason: record.prune_reason,
            status,
            can_delete,
        })
    }
}

fn parse_worktree_records(output: &str) -> Vec<WorktreeRecord> {
    let mut records = Vec::new();
    let mut current = WorktreeRecord::default();

    for line in output.lines() {
        if line.is_empty() {
            push_record(&mut records, &mut current);
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            push_record(&mut records, &mut current);
            current.path = path.to_owned();
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current.head = Some(head.to_owned());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            current.branch = Some(branch.trim_start_matches("refs/heads/").to_owned());
        } else if let Some(reason) = line.strip_prefix("prunable ") {
            current.prune_reason = Some(reason.to_owned());
        }
    }

    push_record(&mut records, &mut current);
    records
}

fn push_record(records: &mut Vec<WorktreeRecord>, current: &mut WorktreeRecord) {
    if current.path.is_empty() {
        return;
    }

    records.push(std::mem::take(current));
}

fn has_changes(path: &str) -> Result<bool, String> {
    // --no-optional-locks: status가 .git/index를 다시 쓰지 않게 해 worktree
    // watcher와의 되먹임을 차단한다(specs/007 research R2).
    let started_at = std::time::Instant::now();
    let output = Command::new("git")
        .args(["--no-optional-locks", "-C", path, "status", "--porcelain"])
        .output()
        .map_err(|error| format!("Failed to run git status: {error}"))?;
    crate::infrastructure::perf_log::log_git("worktree-status", started_at.elapsed());

    if !output.status.success() {
        return Ok(true);
    }

    Ok(!output.stdout.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(path: &str, prune_reason: Option<&str>) -> WorktreeRecord {
        WorktreeRecord {
            path: path.to_owned(),
            head: Some("abc123".to_owned()),
            branch: Some("main".to_owned()),
            prune_reason: prune_reason.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn include_status_false_returns_unknown_without_running_status() {
        // git 저장소가 아닌(존재하지도 않는) 경로: status를 실제로 실행했다면
        // Dirty로 계산됐을 것이므로 Unknown은 status 생략을 증명한다.
        let worktree = GitCliWorktreeProvider
            .to_worktree(record("/nonexistent/worktree", None), false, false)
            .expect("worktree should build without status");

        assert_eq!(worktree.status, GitWorktreeStatus::Unknown);
        assert!(!worktree.can_delete);
    }

    #[test]
    fn include_status_false_keeps_prunable_judgement() {
        let worktree = GitCliWorktreeProvider
            .to_worktree(
                record("/nonexistent/worktree", Some("gitdir file points to non-existent location")),
                false,
                false,
            )
            .expect("worktree should build");

        assert_eq!(worktree.status, GitWorktreeStatus::Prunable);
        assert!(worktree.can_delete);
    }
}

fn run_git_command(mut command: Command, context: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{context}: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = stderr.trim();

    if message.is_empty() {
        Err(context.to_owned())
    } else {
        Err(format!("{context}: {message}"))
    }
}
