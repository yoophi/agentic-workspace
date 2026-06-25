use std::process::Command;

use crate::{
    application::ports::{
        GitBranchReader, GitHistoryReader, GitRepositoryValidator, GitWorktreeReader,
    },
    domain::{
        branch::GitBranch,
        commit::{GitCommitDetail, GitCommitFileChange, GitCommitSummary, GitFileDiff},
        worktree::GitWorktree,
    },
};

const MAX_DIFF_BYTES: usize = 200_000;

pub struct GitCliRepositoryValidator;

impl GitRepositoryValidator for GitCliRepositoryValidator {
    fn validate_repository(&self, path: &str) -> Result<String, String> {
        let output = Command::new("git")
            .args(["-C", path, "rev-parse", "--show-toplevel"])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !output.status.success() {
            return Err(git_error_message(
                &output.stderr,
                "Path is not a Git repository.",
            ));
        }

        let git_root = String::from_utf8(output.stdout)
            .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?
            .trim()
            .to_string();

        if git_root.is_empty() {
            return Err("Git repository root could not be resolved.".to_string());
        }

        Ok(git_root)
    }
}

impl GitWorktreeReader for GitCliRepositoryValidator {
    fn list_worktrees(&self, repository_path: &str) -> Result<Vec<GitWorktree>, String> {
        let output = Command::new("git")
            .args(["-C", repository_path, "worktree", "list", "--porcelain"])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !output.status.success() {
            return Err(git_error_message(
                &output.stderr,
                "Failed to list Git worktrees.",
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;

        parse_worktree_porcelain(&stdout)
    }
}

impl GitBranchReader for GitCliRepositoryValidator {
    fn list_branches(&self, repository_path: &str) -> Result<Vec<GitBranch>, String> {
        let output = Command::new("git")
            .args([
                "-C",
                repository_path,
                "branch",
                "--all",
                "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(worktreepath)",
            ])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !output.status.success() {
            return Err(git_error_message(
                &output.stderr,
                "Failed to list Git branches.",
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;

        parse_branch_format(&stdout)
    }
}

impl GitHistoryReader for GitCliRepositoryValidator {
    fn list_history(&self, repository_path: &str) -> Result<Vec<GitCommitSummary>, String> {
        let output = Command::new("git")
            .args([
                "-C",
                repository_path,
                "log",
                "--max-count=100",
                "--pretty=format:%H%x00%s%x00%an%x00%cI%x1e",
            ])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !output.status.success() {
            return Err(git_error_message(
                &output.stderr,
                "Failed to list Git history.",
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;

        parse_commit_history(&stdout)
    }

    fn get_commit_detail(
        &self,
        repository_path: &str,
        commit_hash: &str,
    ) -> Result<GitCommitDetail, String> {
        let metadata_output = Command::new("git")
            .args([
                "-C",
                repository_path,
                "show",
                "-s",
                "--format=%H%x00%s%x00%an%x00%cI",
                commit_hash,
            ])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !metadata_output.status.success() {
            return Err(git_error_message(
                &metadata_output.stderr,
                "Failed to read Git commit detail.",
            ));
        }

        let metadata = String::from_utf8(metadata_output.stdout)
            .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;
        let file_output = Command::new("git")
            .args([
                "-C",
                repository_path,
                "diff-tree",
                "--root",
                "--no-commit-id",
                "--name-status",
                "-r",
                commit_hash,
            ])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !file_output.status.success() {
            return Err(git_error_message(
                &file_output.stderr,
                "Failed to read Git commit files.",
            ));
        }

        let files = String::from_utf8(file_output.stdout)
            .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;

        parse_commit_detail(&metadata, &files)
    }

    fn get_file_diff(
        &self,
        repository_path: &str,
        commit_hash: &str,
        file_path: &str,
    ) -> Result<GitFileDiff, String> {
        let output = Command::new("git")
            .args([
                "-C",
                repository_path,
                "show",
                "--format=",
                "--find-renames",
                commit_hash,
                "--",
                file_path,
            ])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !output.status.success() {
            return Err(git_error_message(
                &output.stderr,
                "Failed to read Git file diff.",
            ));
        }

        Ok(file_diff_from_output(
            commit_hash,
            file_path,
            &output.stdout,
        ))
    }
}

fn parse_branch_format(output: &str) -> Result<Vec<GitBranch>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts = line.split('\0').collect::<Vec<_>>();

            if parts.len() < 4 {
                return Some(Err(format!("Git branch output is invalid: {line}")));
            }

            let full_name = parts[0];

            if full_name.starts_with("refs/remotes/") && full_name.ends_with("/HEAD") {
                return None;
            }

            let worktree_path = if parts[3].is_empty() {
                None
            } else {
                Some(parts[3].to_string())
            };

            Some(Ok(GitBranch::new(
                parts[1].to_string(),
                full_name.to_string(),
                full_name.starts_with("refs/remotes/"),
                parts[2] == "*",
                worktree_path,
            )))
        })
        .collect()
}

fn parse_commit_history(output: &str) -> Result<Vec<GitCommitSummary>, String> {
    output
        .split('\x1e')
        .filter(|record| !record.trim().is_empty())
        .map(|record| {
            let fields = record
                .trim_start_matches('\n')
                .split('\0')
                .collect::<Vec<_>>();

            if fields.len() != 4 {
                return Err(format!("Git history output is invalid: {record}"));
            }

            Ok(GitCommitSummary::new(
                fields[0].to_string(),
                fields[1].to_string(),
                fields[2].to_string(),
                fields[3].to_string(),
            ))
        })
        .collect()
}

fn parse_commit_detail(metadata: &str, files: &str) -> Result<GitCommitDetail, String> {
    let fields = metadata.trim_end().split('\0').collect::<Vec<_>>();

    if fields.len() != 4 {
        return Err(format!("Git commit detail output is invalid: {metadata}"));
    }

    Ok(GitCommitDetail::new(
        fields[0].to_string(),
        fields[1].to_string(),
        fields[2].to_string(),
        fields[3].to_string(),
        parse_commit_files(files)?,
    ))
}

fn parse_commit_files(output: &str) -> Result<Vec<GitCommitFileChange>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields = line.splitn(2, '\t').collect::<Vec<_>>();

            if fields.len() != 2 {
                return Err(format!("Git commit file output is invalid: {line}"));
            }

            Ok(GitCommitFileChange::new(
                fields[1].to_string(),
                fields[0].to_string(),
            ))
        })
        .collect()
}

fn file_diff_from_output(commit_hash: &str, file_path: &str, output: &[u8]) -> GitFileDiff {
    let is_truncated = output.len() > MAX_DIFF_BYTES;
    let bytes = if is_truncated {
        &output[..MAX_DIFF_BYTES]
    } else {
        output
    };
    let mut content = String::from_utf8_lossy(bytes).to_string();
    let is_binary = content.contains("Binary files") || content.contains("GIT binary patch");

    if is_truncated {
        content.push_str("\n\n[diff truncated]\n");
    }

    GitFileDiff::new(
        commit_hash.to_string(),
        file_path.to_string(),
        content,
        is_binary,
        is_truncated,
    )
}

fn parse_worktree_porcelain(output: &str) -> Result<Vec<GitWorktree>, String> {
    let mut worktrees = Vec::new();
    let mut path: Option<String> = None;
    let mut branch: Option<String> = None;
    let mut commit: Option<String> = None;
    let mut is_bare = false;

    for line in output.lines().chain(std::iter::once("")) {
        if line.is_empty() {
            if let Some(current_path) = path.take() {
                let current_commit = commit
                    .take()
                    .ok_or_else(|| format!("Git worktree has no HEAD: {current_path}"))?;
                let is_main = worktrees.is_empty();

                worktrees.push(GitWorktree::new(
                    current_path,
                    branch.take(),
                    current_commit,
                    is_bare,
                    is_main,
                ));
                is_bare = false;
            }

            continue;
        }

        if let Some(value) = line.strip_prefix("worktree ") {
            path = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("HEAD ") {
            commit = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("branch ") {
            branch = Some(
                value
                    .strip_prefix("refs/heads/")
                    .unwrap_or(value)
                    .to_string(),
            );
        } else if line == "bare" {
            is_bare = true;
        }
    }

    Ok(worktrees)
}

fn git_error_message(stderr: &[u8], fallback: &str) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();

    if stderr.is_empty() {
        fallback.to_string()
    } else {
        stderr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_main_and_linked_worktrees() {
        let output = "\
worktree /repo
HEAD abc123
branch refs/heads/main

worktree /repo-feature
HEAD def456
branch refs/heads/feature/worktree-list
";

        let worktrees = parse_worktree_porcelain(output).expect("worktrees should parse");

        assert_eq!(worktrees.len(), 2);
        assert_eq!(worktrees[0].path, "/repo");
        assert_eq!(worktrees[0].branch.as_deref(), Some("main"));
        assert!(worktrees[0].is_main);
        assert_eq!(worktrees[1].path, "/repo-feature");
        assert_eq!(
            worktrees[1].branch.as_deref(),
            Some("feature/worktree-list")
        );
        assert!(!worktrees[1].is_main);
    }

    #[test]
    fn parses_bare_and_detached_worktrees() {
        let output = "\
worktree /repo
HEAD abc123
bare

worktree /repo-detached
HEAD def456
detached
";

        let worktrees = parse_worktree_porcelain(output).expect("worktrees should parse");

        assert!(worktrees[0].is_bare);
        assert_eq!(worktrees[0].branch, None);
        assert_eq!(worktrees[1].branch, None);
    }

    #[test]
    fn parses_local_and_remote_branches() {
        let output = "\
refs/heads/main\0main\0*\0/repo
refs/heads/feature/foo\0feature/foo\0 \0/repo-feature
refs/remotes/origin/main\0origin/main\0 \0
refs/remotes/origin/HEAD\0origin/HEAD\0 \0
";

        let branches = parse_branch_format(output).expect("branches should parse");

        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0].name, "main");
        assert!(branches[0].is_current);
        assert_eq!(branches[0].worktree_path.as_deref(), Some("/repo"));
        assert_eq!(branches[1].name, "feature/foo");
        assert!(!branches[1].is_remote);
        assert_eq!(branches[2].name, "origin/main");
        assert!(branches[2].is_remote);
    }

    #[test]
    fn parses_commit_history() {
        let output = "\
abc123\0Initial commit\0A Developer\02026-06-25T00:00:00+09:00\x1e
def456\0Add feature\0B Developer\02026-06-25T01:00:00+09:00\x1e";

        let commits = parse_commit_history(output).expect("history should parse");

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].message, "Initial commit");
        assert_eq!(commits[1].author, "B Developer");
    }

    #[test]
    fn parses_commit_detail() {
        let metadata = "abc123\0Initial commit\0A Developer\02026-06-25T00:00:00+09:00\n";
        let files = "A\tREADME.md\nM\tapps/desktop/src/main.tsx\n";

        let detail = parse_commit_detail(metadata, files).expect("detail should parse");

        assert_eq!(detail.hash, "abc123");
        assert_eq!(detail.files.len(), 2);
        assert_eq!(detail.files[0].status, "A");
        assert_eq!(detail.files[1].path, "apps/desktop/src/main.tsx");
    }

    #[test]
    fn marks_large_file_diff_as_truncated() {
        let output = vec![b'a'; MAX_DIFF_BYTES + 10];

        let diff = file_diff_from_output("abc123", "README.md", &output);

        assert!(diff.is_truncated);
        assert!(diff.content.ends_with("[diff truncated]\n"));
    }

    #[test]
    fn marks_binary_file_diff() {
        let output = b"Binary files a/image.png and b/image.png differ";

        let diff = file_diff_from_output("abc123", "image.png", output);

        assert!(diff.is_binary);
    }
}
