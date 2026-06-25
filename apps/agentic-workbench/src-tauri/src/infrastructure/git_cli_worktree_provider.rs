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
    fn list_worktrees(&self, working_directory: &str) -> Result<Vec<GitWorktree>, String> {
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
            .map(|(index, record)| self.to_worktree(record, index == 0))
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
        let worktree = self
            .list_worktrees(working_directory)?
            .into_iter()
            .find(|worktree| worktree.path == path)
            .ok_or_else(|| "Git worktree not found.".to_owned())?;

        match worktree.status {
            GitWorktreeStatus::Dirty => {
                Err("Worktree has changes and cannot be deleted.".to_owned())
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
    ) -> Result<GitWorktree, String> {
        let status = if record.prune_reason.is_some() {
            GitWorktreeStatus::Prunable
        } else if has_changes(&record.path)? {
            GitWorktreeStatus::Dirty
        } else {
            GitWorktreeStatus::Clean
        };
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
    let output = Command::new("git")
        .args(["-C", path, "status", "--porcelain"])
        .output()
        .map_err(|error| format!("Failed to run git status: {error}"))?;

    if !output.status.success() {
        return Ok(true);
    }

    Ok(!output.stdout.is_empty())
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
