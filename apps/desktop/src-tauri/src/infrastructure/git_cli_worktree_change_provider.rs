use std::{fs, path::PathBuf, process::Command};

use crate::{
    domain::worktree_change::{WorktreeChange, WorktreeChangeType},
    ports::worktree_change_provider::WorktreeChangeProvider,
};

const CONTENT_LIMIT_BYTES: usize = 120_000;

pub struct GitCliWorktreeChangeProvider;

impl WorktreeChangeProvider for GitCliWorktreeChangeProvider {
    fn list_changes(&self, working_directory: String) -> Result<Vec<WorktreeChange>, String> {
        let output = git_output(
            &working_directory,
            ["status", "--short", "--renames"].as_slice(),
        )?;

        output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| change_from_status_line(&working_directory, line))
            .collect()
    }
}

fn change_from_status_line(working_directory: &str, line: &str) -> Result<WorktreeChange, String> {
    let status = line
        .get(..2)
        .ok_or_else(|| format!("Unexpected git status line: {line}"))?;
    let path_text = line.get(3..).unwrap_or("").trim();

    let (path, old_path) = if matches!(status.chars().next(), Some('R' | 'C'))
        || matches!(status.chars().nth(1), Some('R' | 'C'))
    {
        split_rename_path(path_text)
    } else {
        (path_text.to_string(), None)
    };

    let change_type = change_type_from_status(status);
    let mut change = WorktreeChange {
        summary: summary_for_change(&change_type, &path, old_path.as_deref()),
        path,
        old_path,
        change_type,
        diff: None,
        preview: None,
        binary: false,
        truncated: false,
        message: None,
    };

    match change.change_type {
        WorktreeChangeType::Added if status == "??" => {
            apply_untracked_preview(working_directory, &mut change)?;
        }
        _ => {
            apply_git_diff(working_directory, &mut change)?;
        }
    }

    Ok(change)
}

fn change_type_from_status(status: &str) -> WorktreeChangeType {
    if status == "??" {
        return WorktreeChangeType::Added;
    }
    if status.contains('U') {
        return WorktreeChangeType::Unmerged;
    }
    if status.contains('R') {
        return WorktreeChangeType::Renamed;
    }
    if status.contains('C') {
        return WorktreeChangeType::Copied;
    }
    if status.contains('D') {
        return WorktreeChangeType::Deleted;
    }
    if status.contains('A') {
        return WorktreeChangeType::Added;
    }
    if status.contains('M') || status.contains('T') {
        return WorktreeChangeType::Modified;
    }
    WorktreeChangeType::Unknown
}

fn split_rename_path(path_text: &str) -> (String, Option<String>) {
    if let Some((old_path, new_path)) = path_text.split_once(" -> ") {
        (new_path.to_string(), Some(old_path.to_string()))
    } else {
        (path_text.to_string(), None)
    }
}

fn summary_for_change(
    change_type: &WorktreeChangeType,
    path: &str,
    old_path: Option<&str>,
) -> String {
    match change_type {
        WorktreeChangeType::Added => format!("Added {path}"),
        WorktreeChangeType::Modified => format!("Modified {path}"),
        WorktreeChangeType::Deleted => format!("Deleted {path}"),
        WorktreeChangeType::Renamed => {
            format!("Renamed {} to {path}", old_path.unwrap_or("unknown path"))
        }
        WorktreeChangeType::Copied => format!("Copied {path}"),
        WorktreeChangeType::Unmerged => format!("Unmerged changes in {path}"),
        WorktreeChangeType::Unknown => format!("Changed {path}"),
    }
}

fn apply_git_diff(working_directory: &str, change: &mut WorktreeChange) -> Result<(), String> {
    let output = git_output(
        working_directory,
        [
            "diff",
            "--no-ext-diff",
            "--find-renames",
            "HEAD",
            "--",
            change.path.as_str(),
        ]
        .as_slice(),
    )?;

    if output.trim().is_empty() {
        change.message = Some("No unified diff is available for this file.".to_string());
        return Ok(());
    }

    if output.contains("Binary files") || output.contains("GIT binary patch") {
        change.binary = true;
        change.message = Some("Binary file diff is not available.".to_string());
        return Ok(());
    }

    let (diff, truncated) = truncate_text(&output, CONTENT_LIMIT_BYTES);
    change.diff = Some(diff);
    change.truncated = truncated;
    Ok(())
}

fn apply_untracked_preview(
    working_directory: &str,
    change: &mut WorktreeChange,
) -> Result<(), String> {
    let path = safe_worktree_path(working_directory, &change.path)?;
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("Failed to inspect {}: {error}", change.path))?;
    if !metadata.is_file() {
        change.message = Some("Preview is available only for regular files.".to_string());
        return Ok(());
    }

    let mut bytes =
        fs::read(&path).map_err(|error| format!("Failed to read {}: {error}", change.path))?;
    let truncated = bytes.len() > CONTENT_LIMIT_BYTES;
    if truncated {
        bytes.truncate(CONTENT_LIMIT_BYTES);
    }

    if bytes.contains(&0) {
        change.binary = true;
        change.truncated = truncated;
        change.message = Some("Binary file preview is not available.".to_string());
        return Ok(());
    }

    match String::from_utf8(bytes) {
        Ok(contents) => {
            change.preview = Some(contents);
            change.truncated = truncated;
        }
        Err(_) => {
            change.binary = true;
            change.truncated = truncated;
            change.message = Some("File preview is not valid UTF-8 text.".to_string());
        }
    }

    Ok(())
}

fn safe_worktree_path(working_directory: &str, relative_path: &str) -> Result<PathBuf, String> {
    let base = fs::canonicalize(working_directory)
        .map_err(|error| format!("Failed to resolve working directory: {error}"))?;
    let candidate = base.join(relative_path);
    let resolved = candidate
        .canonicalize()
        .map_err(|error| format!("Failed to resolve {relative_path}: {error}"))?;

    if !resolved.starts_with(&base) {
        return Err(format!("{relative_path} is outside the working directory."));
    }

    Ok(resolved)
}

fn git_output(working_directory: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(working_directory)
        .args(args)
        .output()
        .map_err(|error| format!("Failed to run git: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn truncate_text(text: &str, max_bytes: usize) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text.to_string(), false);
    }

    let mut end = 0;
    for (index, _) in text.char_indices() {
        if index > max_bytes {
            break;
        }
        end = index;
    }

    (text[..end].to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_status_lines() {
        let modified = change_type_from_status(" M");
        let added = change_type_from_status("??");
        let deleted = change_type_from_status(" D");
        let renamed = change_type_from_status("R ");

        assert_eq!(modified, WorktreeChangeType::Modified);
        assert_eq!(added, WorktreeChangeType::Added);
        assert_eq!(deleted, WorktreeChangeType::Deleted);
        assert_eq!(renamed, WorktreeChangeType::Renamed);
        assert_eq!(
            split_rename_path("old name.ts -> new name.ts"),
            ("new name.ts".to_string(), Some("old name.ts".to_string()))
        );
    }

    #[test]
    fn truncates_on_utf8_boundary() {
        let (text, truncated) = truncate_text("abc가나다", 5);

        assert_eq!(text, "abc");
        assert!(truncated);
    }
}
