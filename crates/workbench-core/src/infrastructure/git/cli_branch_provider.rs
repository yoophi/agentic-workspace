use std::process::Command;

use crate::{
    domain::{errors::GitError, git_branch::GitBranch},
    ports::git_providers::GitBranchProvider,
};

pub struct GitCliBranchProvider;

impl GitBranchProvider for GitCliBranchProvider {
    fn list_branches(&self, working_directory: &str) -> Result<Vec<GitBranch>, GitError> {
        let output = Command::new("git")
            .args([
                "-C",
                working_directory,
                "branch",
                "--all",
                "--format=%(refname:short)|%(HEAD)",
            ])
            .output()
            .map_err(|error| GitError::spawn("Failed to run git branch", &error))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8(output.stdout).map_err(|error| {
            GitError::CommandFailed(format!("Failed to read git branch output: {error}"))
        })?;

        Ok(parse_branch_output(&stdout))
    }
}

fn parse_branch_output(output: &str) -> Vec<GitBranch> {
    output
        .lines()
        .filter_map(|line| {
            let (name, head) = line.split_once('|')?;
            let name = name.trim();

            if name.is_empty() || name == "HEAD" || name.ends_with("/HEAD") {
                return None;
            }

            Some(GitBranch {
                name: name.to_owned(),
                is_current: head.trim() == "*",
                is_remote: name.starts_with("remotes/"),
            })
        })
        .collect()
}
