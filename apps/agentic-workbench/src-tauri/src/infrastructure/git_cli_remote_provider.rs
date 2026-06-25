use std::{collections::BTreeMap, process::Command};

use crate::domain::{git_remote::GitRemote, git_remote_provider::GitRemoteProvider};

pub struct GitCliRemoteProvider;

impl GitRemoteProvider for GitCliRemoteProvider {
    fn list_remotes(&self, working_directory: &str) -> Result<Vec<GitRemote>, String> {
        let output = Command::new("git")
            .args(["-C", working_directory, "remote", "-v"])
            .output()
            .map_err(|error| format!("Failed to run git: {error}"))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Failed to read git output: {error}"))?;

        Ok(parse_git_remote_output(&stdout))
    }
}

fn parse_git_remote_output(output: &str) -> Vec<GitRemote> {
    let mut remotes = BTreeMap::<String, GitRemote>::new();

    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(url) = parts.next() else {
            continue;
        };
        let kind = parts.next().unwrap_or_default();
        let remote = remotes.entry(name.to_owned()).or_insert_with(|| GitRemote {
            name: name.to_owned(),
            fetch_url: None,
            push_url: None,
        });

        match kind {
            "(fetch)" => remote.fetch_url = Some(url.to_owned()),
            "(push)" => remote.push_url = Some(url.to_owned()),
            _ => {}
        }
    }

    remotes.into_values().collect()
}
