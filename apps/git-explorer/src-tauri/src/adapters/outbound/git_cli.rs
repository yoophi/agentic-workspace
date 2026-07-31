use std::process::Command;

use git_core::git_error_message;

use crate::{
    application::ports::{GitBranchReader, GitRepositoryValidator, GitWorktreeReader},
    domain::{branch::GitBranch, worktree::GitWorktree},
};

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
}
