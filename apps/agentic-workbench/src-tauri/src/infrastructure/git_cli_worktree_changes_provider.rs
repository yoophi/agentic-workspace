use std::process::Command;

use crate::domain::{
    git_worktree_changes::{GitChangedFile, GitChangedFileGroup, GitFileDiff, GitWorktreeChanges},
    git_worktree_changes_provider::GitWorktreeChangesProvider,
};

const MAX_DIFF_BYTES: usize = 120_000;

pub struct GitCliWorktreeChangesProvider;

impl GitWorktreeChangesProvider for GitCliWorktreeChangesProvider {
    fn status(&self, working_directory: &str) -> Result<GitWorktreeChanges, String> {
        let output = Command::new("git")
            .args(["-C", working_directory, "status", "--porcelain=v1", "-uall"])
            .output()
            .map_err(|error| format!("Failed to run git status: {error}"))?;

        if !output.status.success() {
            return Err(git_error("Failed to read git status", &output.stderr));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Failed to parse git status output: {error}"))?;
        let files = parse_status_output(&stdout);

        Ok(summarize_changes(working_directory, files))
    }

    fn diff(&self, working_directory: &str, path: &str) -> Result<GitFileDiff, String> {
        let mut diff = run_diff(working_directory, path, false)?;
        if diff.is_empty() {
            diff = run_diff(working_directory, path, true)?;
        }
        if diff.is_empty() {
            diff = format!(
                "No textual git diff is available for `{path}`. The file may be untracked or unchanged relative to the selected diff scope."
            );
        }

        let binary = diff.contains("Binary files ") || diff.contains("GIT binary patch");
        let truncated = diff.len() > MAX_DIFF_BYTES;
        if truncated {
            diff.truncate(MAX_DIFF_BYTES);
            diff.push_str("\n\n[diff truncated]");
        }

        Ok(GitFileDiff {
            path: path.to_string(),
            diff,
            truncated,
            binary,
        })
    }
}

fn run_diff(working_directory: &str, path: &str, cached: bool) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(["-C", working_directory, "diff", "--no-ext-diff"]);
    if cached {
        command.arg("--cached");
    }
    command.args(["--", path]);

    let output = command
        .output()
        .map_err(|error| format!("Failed to run git diff: {error}"))?;

    if !output.status.success() {
        return Err(git_error("Failed to read git diff", &output.stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_status_output(output: &str) -> Vec<GitChangedFile> {
    output.lines().filter_map(parse_status_line).collect()
}

fn parse_status_line(line: &str) -> Option<GitChangedFile> {
    if line.len() < 4 {
        return None;
    }

    let staged = line.chars().next()?;
    let unstaged = line.chars().nth(1)?;
    let raw_path = line.get(3..)?.trim();
    let (old_path, path) = parse_status_path(raw_path);
    let group = status_group(staged, unstaged);

    Some(GitChangedFile {
        path,
        old_path,
        staged_status: status_label(staged),
        unstaged_status: status_label(unstaged),
        group,
    })
}

fn parse_status_path(raw_path: &str) -> (Option<String>, String) {
    let unquoted = raw_path.trim_matches('"');
    if let Some((old_path, path)) = unquoted.split_once(" -> ") {
        return (Some(old_path.to_string()), path.to_string());
    }
    (None, unquoted.to_string())
}

fn status_group(staged: char, unstaged: char) -> GitChangedFileGroup {
    if staged == '?' && unstaged == '?' {
        return GitChangedFileGroup::Untracked;
    }
    if matches!(staged, 'U' | 'A' | 'D') && matches!(unstaged, 'U' | 'A' | 'D') {
        return GitChangedFileGroup::Conflicted;
    }
    if staged != ' ' {
        return GitChangedFileGroup::Staged;
    }
    GitChangedFileGroup::Unstaged
}

fn status_label(status: char) -> Option<String> {
    (status != ' ').then(|| status.to_string())
}

fn summarize_changes(working_directory: &str, files: Vec<GitChangedFile>) -> GitWorktreeChanges {
    let staged_count = files
        .iter()
        .filter(|file| file.group == GitChangedFileGroup::Staged)
        .count();
    let unstaged_count = files
        .iter()
        .filter(|file| file.group == GitChangedFileGroup::Unstaged)
        .count();
    let untracked_count = files
        .iter()
        .filter(|file| file.group == GitChangedFileGroup::Untracked)
        .count();
    let conflicted_count = files
        .iter()
        .filter(|file| file.group == GitChangedFileGroup::Conflicted)
        .count();

    GitWorktreeChanges {
        working_directory: working_directory.to_string(),
        files,
        staged_count,
        unstaged_count,
        untracked_count,
        conflicted_count,
    }
}

fn git_error(context: &str, stderr: &[u8]) -> String {
    let message = String::from_utf8_lossy(stderr).trim().to_string();
    if message.is_empty() {
        context.to_string()
    } else {
        format!("{context}: {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_groups_and_renames() {
        let changes = summarize_changes(
            "/repo",
            parse_status_output(
                "M  staged.ts\n M unstaged.ts\n?? new.ts\nUU conflict.ts\nR  old.ts -> renamed.ts\n",
            ),
        );

        assert_eq!(changes.staged_count, 2);
        assert_eq!(changes.unstaged_count, 1);
        assert_eq!(changes.untracked_count, 1);
        assert_eq!(changes.conflicted_count, 1);
        assert_eq!(changes.files[4].old_path.as_deref(), Some("old.ts"));
        assert_eq!(changes.files[4].path, "renamed.ts");
    }
}
