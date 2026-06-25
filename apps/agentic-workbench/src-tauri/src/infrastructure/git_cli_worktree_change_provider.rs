use std::{path::Path, process::Command};

use crate::domain::{
    worktree_change::{WorktreeChange, WorktreeChangeType},
    worktree_change_provider::WorktreeChangeProvider,
};

/// diff/내용 표시 한도(문자 수). 이보다 길면 잘라내고 truncated 플래그를 세운다.
const MAX_TEXT_LEN: usize = 200_000;
/// 한 번에 처리할 최대 변경 파일 수. 비정상적으로 큰 변경에서 UI/메모리를 보호한다.
const MAX_FILES: usize = 500;
/// binary 판정을 위해 검사할 선두 바이트 수.
const BINARY_SNIFF_LEN: usize = 8000;

pub struct GitCliWorktreeChangeProvider;

impl WorktreeChangeProvider for GitCliWorktreeChangeProvider {
    fn list_changes(&self, working_directory: &str) -> Result<Vec<WorktreeChange>, String> {
        let mut changes = Vec::new();

        for entry in self.tracked_changes(working_directory)? {
            if changes.len() >= MAX_FILES {
                return Ok(changes);
            }
            changes.push(self.build_tracked_change(working_directory, entry));
        }

        for path in self.untracked_paths(working_directory)? {
            if changes.len() >= MAX_FILES {
                return Ok(changes);
            }
            changes.push(self.build_untracked_change(working_directory, path));
        }

        Ok(changes)
    }
}

struct TrackedEntry {
    change_type: WorktreeChangeType,
    path: String,
    old_path: Option<String>,
}

impl GitCliWorktreeChangeProvider {
    /// HEAD 대비 추적 파일의 변경 목록(rename 감지 포함)을 가져온다.
    fn tracked_changes(&self, working_directory: &str) -> Result<Vec<TrackedEntry>, String> {
        let output = Command::new("git")
            .args([
                "-C",
                working_directory,
                "diff",
                "--name-status",
                "-M",
                "-z",
                "HEAD",
            ])
            .output()
            .map_err(|error| format!("Failed to run git diff: {error}"))?;

        // HEAD가 없는(커밋 0개) 저장소 등에서는 빈 목록으로 처리한다.
        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(parse_name_status_z(&stdout))
    }

    fn build_tracked_change(&self, working_directory: &str, entry: TrackedEntry) -> WorktreeChange {
        let raw_diff = self
            .diff_for_path(working_directory, &entry)
            .unwrap_or_default();

        if raw_diff.is_empty() {
            return WorktreeChange {
                path: entry.path,
                old_path: entry.old_path,
                change_type: entry.change_type,
                binary: false,
                diff: None,
                content: None,
                truncated: false,
            };
        }

        if diff_is_binary(&raw_diff) {
            return WorktreeChange {
                path: entry.path,
                old_path: entry.old_path,
                change_type: entry.change_type,
                binary: true,
                diff: None,
                content: None,
                truncated: false,
            };
        }

        let (diff, truncated) = truncate_text(raw_diff, MAX_TEXT_LEN);
        WorktreeChange {
            path: entry.path,
            old_path: entry.old_path,
            change_type: entry.change_type,
            binary: false,
            diff: Some(diff),
            content: None,
            truncated,
        }
    }

    fn diff_for_path(
        &self,
        working_directory: &str,
        entry: &TrackedEntry,
    ) -> Result<String, String> {
        let mut command = Command::new("git");
        command.args(["-C", working_directory, "diff", "-M", "HEAD", "--"]);
        if let Some(old_path) = &entry.old_path {
            command.arg(old_path);
        }
        command.arg(&entry.path);

        let output = command
            .output()
            .map_err(|error| format!("Failed to run git diff: {error}"))?;

        if !output.status.success() {
            return Ok(String::new());
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// `.gitignore`를 존중하면서 untracked 파일 목록을 가져온다.
    fn untracked_paths(&self, working_directory: &str) -> Result<Vec<String>, String> {
        let output = Command::new("git")
            .args([
                "-C",
                working_directory,
                "ls-files",
                "--others",
                "--exclude-standard",
                "-z",
            ])
            .output()
            .map_err(|error| format!("Failed to run git ls-files: {error}"))?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(|path| path.to_owned())
            .collect())
    }

    fn build_untracked_change(&self, working_directory: &str, path: String) -> WorktreeChange {
        let absolute = Path::new(working_directory).join(&path);
        match std::fs::read(&absolute) {
            Ok(bytes) if looks_binary(&bytes) => WorktreeChange {
                path,
                old_path: None,
                change_type: WorktreeChangeType::Untracked,
                binary: true,
                diff: None,
                content: None,
                truncated: false,
            },
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let (content, truncated) = truncate_text(text, MAX_TEXT_LEN);
                WorktreeChange {
                    path,
                    old_path: None,
                    change_type: WorktreeChangeType::Untracked,
                    binary: false,
                    diff: None,
                    content: Some(content),
                    truncated,
                }
            }
            Err(_) => WorktreeChange {
                path,
                old_path: None,
                change_type: WorktreeChangeType::Untracked,
                binary: false,
                diff: None,
                content: None,
                truncated: false,
            },
        }
    }
}

/// `git diff --name-status -M -z HEAD` 출력을 파싱한다.
/// 모든 필드가 NUL로 구분되며, rename/copy는 상태 뒤에 (old, new) 두 경로가 온다.
fn parse_name_status_z(output: &str) -> Vec<TrackedEntry> {
    let mut tokens = output.split('\0').filter(|token| !token.is_empty());
    let mut entries = Vec::new();

    while let Some(status) = tokens.next() {
        let code = status.chars().next().unwrap_or(' ');
        match code {
            'R' | 'C' => {
                let Some(old_path) = tokens.next() else {
                    break;
                };
                let Some(new_path) = tokens.next() else {
                    break;
                };
                entries.push(TrackedEntry {
                    change_type: if code == 'R' {
                        WorktreeChangeType::Renamed
                    } else {
                        WorktreeChangeType::Added
                    },
                    path: new_path.to_owned(),
                    old_path: Some(old_path.to_owned()),
                });
            }
            other => {
                let Some(path) = tokens.next() else {
                    break;
                };
                entries.push(TrackedEntry {
                    change_type: change_type_from_code(other),
                    path: path.to_owned(),
                    old_path: None,
                });
            }
        }
    }

    entries
}

fn change_type_from_code(code: char) -> WorktreeChangeType {
    match code {
        'A' => WorktreeChangeType::Added,
        'D' => WorktreeChangeType::Deleted,
        // 'M'(수정), 'T'(타입 변경) 등은 모두 수정으로 취급한다.
        _ => WorktreeChangeType::Modified,
    }
}

/// git diff 출력이 binary 파일 변경을 나타내는지 확인한다.
fn diff_is_binary(diff: &str) -> bool {
    diff.lines()
        .any(|line| line.starts_with("Binary files ") || line == "GIT binary patch")
}

/// 선두 바이트에 NUL이 있으면 binary로 간주한다.
fn looks_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_SNIFF_LEN).any(|byte| *byte == 0)
}

/// 문자열을 char 경계 기준 max 길이로 자르고, 잘렸는지 여부를 반환한다.
fn truncate_text(text: String, max: usize) -> (String, bool) {
    if text.chars().count() <= max {
        return (text, false);
    }

    let truncated: String = text.chars().take(max).collect();
    (truncated, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modified_added_and_deleted_entries() {
        let output = "M\0src/lib.rs\0A\0src/new.rs\0D\0src/old.rs\0";
        let entries = parse_name_status_z(output);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].change_type, WorktreeChangeType::Modified);
        assert_eq!(entries[0].path, "src/lib.rs");
        assert_eq!(entries[1].change_type, WorktreeChangeType::Added);
        assert_eq!(entries[2].change_type, WorktreeChangeType::Deleted);
        assert_eq!(entries[2].path, "src/old.rs");
    }

    #[test]
    fn parses_rename_with_old_and_new_paths() {
        let output = "R100\0src/old_name.rs\0src/new_name.rs\0";
        let entries = parse_name_status_z(output);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].change_type, WorktreeChangeType::Renamed);
        assert_eq!(entries[0].old_path.as_deref(), Some("src/old_name.rs"));
        assert_eq!(entries[0].path, "src/new_name.rs");
    }

    #[test]
    fn detects_binary_diff_output() {
        assert!(diff_is_binary(
            "diff --git a/logo.png b/logo.png\nBinary files a/logo.png and b/logo.png differ\n"
        ));
        assert!(!diff_is_binary(
            "diff --git a/x.txt b/x.txt\n@@ -1 +1 @@\n-a\n+b\n"
        ));
    }

    #[test]
    fn detects_binary_bytes() {
        assert!(looks_binary(&[0x89, 0x50, 0x00, 0x01]));
        assert!(!looks_binary(b"plain text content"));
    }

    #[test]
    fn truncates_only_when_over_limit() {
        let (text, truncated) = truncate_text("hello".to_owned(), 10);
        assert_eq!(text, "hello");
        assert!(!truncated);

        let (text, truncated) = truncate_text("hello world".to_owned(), 5);
        assert_eq!(text, "hello");
        assert!(truncated);
    }
}
