use std::process::Command;

use crate::domain::{
    GitChangedFile, GitChangedFileGroup, GitCommitDetail, GitCommitFileChange, GitCommitGraph,
    GitCommitHistory, GitCommitPage, GitCommitSummary, GitFileDiff, GitGraphCommit,
    GitGraphLayoutHints, GitGraphPage, GitGraphRef, GitGraphRefKind, GitWorktreeChanges,
    GitWorktreeFileDiff,
};
use crate::ports::{GitHistoryReader, GitWorktreeStatusReader};

const MAX_DIFF_BYTES: usize = 200_000;

/// git CLI(`std::process::Command`) 기반 GitHistoryReader 구현. git2 crate 미사용.
/// git-explorer·agentic-workbench가 working_directory(path)를 넘겨 공유한다.
pub struct GitCliHistoryReader;

impl GitHistoryReader for GitCliHistoryReader {
    fn list_history(
        &self,
        repository_path: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
        included_refs: &[String],
        excluded_refs: &[String],
    ) -> Result<GitCommitHistory, String> {
        let revisions = git_history_revisions(included_refs, excluded_refs, false);

        if let Some(cursor) = cursor {
            if !commit_exists(repository_path, cursor)? {
                return Ok(GitCommitHistory::new(
                    Vec::new(),
                    GitCommitPage::invalidated(offset, limit),
                ));
            }
        }

        // count는 첫 페이지에서만 계산한다. 후속 페이지는 limit+1 조회로
        // has_more만 판정해 전체 이력 순회를 반복하지 않는다.
        let is_first_page = offset == 0 && cursor.is_none();
        let total_count = if is_first_page {
            Some(git_revision_count(repository_path, &revisions)?)
        } else {
            None
        };

        if total_count == Some(0) {
            return Ok(GitCommitHistory::new(
                Vec::new(),
                GitCommitPage::new(offset, limit, 0, 0),
            ));
        }

        let fetch_limit = if total_count.is_some() { limit } else { limit + 1 };
        let output = Command::new("git")
            .args([
                "-C",
                repository_path,
                "log",
                "--pretty=format:%H%x00%s%x00%an%x00%cI%x1e",
                &format!("--skip={offset}"),
                "-n",
                &fetch_limit.to_string(),
            ])
            .args(&revisions)
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
        let mut commits = parse_commit_history(&stdout)?;
        let page = finalize_page(&mut commits, total_count, offset, limit);

        Ok(GitCommitHistory::new(commits, page))
    }

    fn get_commit_graph(
        &self,
        repository_path: &str,
        limit: usize,
        offset: usize,
        cursor: Option<&str>,
        included_refs: &[String],
        excluded_refs: &[String],
    ) -> Result<GitCommitGraph, String> {
        let revisions = git_history_revisions(included_refs, excluded_refs, true);

        if let Some(cursor) = cursor {
            if !commit_exists(repository_path, cursor)? {
                return Ok(GitCommitGraph::new(
                    Vec::new(),
                    Vec::new(),
                    GitGraphPage::invalidated(offset, limit),
                    GitGraphLayoutHints::default_row_layout(),
                ));
            }
        }

        let is_first_page = offset == 0 && cursor.is_none();
        let fetch_limit = if is_first_page { limit } else { limit + 1 };
        let run_log = || -> Result<String, String> {
            let log_output = Command::new("git")
                .args([
                    "-C",
                    repository_path,
                    "log",
                    "--exclude=refs/stash",
                    "--topo-order",
                    "--date=iso-strict",
                    "--pretty=format:%H%x00%h%x00%P%x00%s%x00%an%x00%cI%x1e",
                    &format!("--skip={offset}"),
                    "-n",
                    &fetch_limit.to_string(),
                ])
                .args(&revisions)
                .output()
                .map_err(|error| format!("Failed to run git: {error}"))?;

            if !log_output.status.success() {
                return Err(git_error_message(
                    &log_output.stderr,
                    "Failed to read Git commit graph.",
                ));
            }

            String::from_utf8(log_output.stdout)
                .map_err(|error| format!("Git returned invalid UTF-8: {error}"))
        };

        // 첫 페이지: 서로 독립인 count/head/log/refs 조회를 병렬 실행한다
        // (AW specs/007 research R9). 후속 페이지: log 1회만 실행한다.
        let (total_count, head_hash, log_stdout, refs) = if is_first_page {
            let (count_result, head_result, log_result, refs_result) = std::thread::scope(|scope| {
                let count = scope.spawn(|| git_revision_count(repository_path, &revisions));
                let head = scope.spawn(|| git_head_hash(repository_path));
                let log = scope.spawn(run_log);
                let refs = scope.spawn(|| read_graph_refs(repository_path));

                (
                    count.join(),
                    head.join(),
                    log.join(),
                    refs.join(),
                )
            });
            let join_error = |_| "Git graph worker thread panicked.".to_string();
            let total_count = count_result.map_err(join_error)??;
            let head_hash = head_result.map_err(join_error)??;
            let log_stdout = log_result.map_err(join_error)??;
            let refs = refs_result.map_err(join_error)??;

            if total_count == 0 {
                return Ok(GitCommitGraph::new(
                    Vec::new(),
                    Vec::new(),
                    GitGraphPage::new(offset, limit, 0, 0),
                    GitGraphLayoutHints::default_row_layout(),
                ));
            }

            (Some(total_count), head_hash, log_stdout, refs)
        } else {
            let head_hash = git_head_hash(repository_path)?;
            (None, head_hash, run_log()?, Vec::new())
        };

        let mut commits = parse_commit_graph_history(&log_stdout, &head_hash)?;
        let page = finalize_page(&mut commits, total_count, offset, limit);

        Ok(GitCommitGraph::new(
            commits,
            refs,
            page,
            GitGraphLayoutHints::default_row_layout(),
        ))
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

        Ok(file_diff_from_output(commit_hash, file_path, &output.stdout))
    }
}

/// 페이지 메타 계산을 history/graph가 공유한다. total이 없으면(limit+1 조회)
/// 초과분으로 has_more를 판정하고 commits를 limit로 자른다.
fn finalize_page<T>(
    commits: &mut Vec<T>,
    total_count: Option<usize>,
    offset: usize,
    limit: usize,
) -> GitCommitPage {
    match total_count {
        Some(total_count) => GitCommitPage::new(offset, limit, total_count, commits.len()),
        None => {
            let has_more = commits.len() > limit;
            commits.truncate(limit);
            GitCommitPage::without_total(offset, limit, has_more)
        }
    }
}

fn git_revision_count(repository_path: &str, revisions: &[String]) -> Result<usize, String> {
    let output = Command::new("git")
        .args([
            "-C",
            repository_path,
            "rev-list",
            "--exclude=refs/stash",
            "--count",
        ])
        .args(revisions)
        .output()
        .map_err(|error| format!("Failed to run git: {error}"))?;

    if !output.status.success() {
        return Err(git_error_message(
            &output.stderr,
            "Failed to count Git commits.",
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;

    stdout
        .trim()
        .parse::<usize>()
        .map_err(|error| format!("Git commit count is invalid: {error}"))
}

fn git_history_revisions(
    included_refs: &[String],
    excluded_refs: &[String],
    default_all: bool,
) -> Vec<String> {
    if !included_refs.is_empty() {
        return included_refs.to_vec();
    }

    if !excluded_refs.is_empty() {
        let mut revisions = vec!["--all".to_string(), "--not".to_string()];
        revisions.extend(excluded_refs.iter().cloned());
        return revisions;
    }

    if default_all {
        return vec!["--all".to_string()];
    }

    vec!["HEAD".to_string()]
}

/// cursor(commit hash)가 현재 저장소에 존재하는지 확인한다.
fn commit_exists(repository_path: &str, commit_hash: &str) -> Result<bool, String> {
    let output = Command::new("git")
        .args([
            "-C",
            repository_path,
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{commit_hash}^{{commit}}"),
        ])
        .output()
        .map_err(|error| format!("Failed to run git: {error}"))?;

    Ok(output.status.success())
}

fn read_graph_refs(repository_path: &str) -> Result<Vec<GitGraphRef>, String> {
    let refs_output = Command::new("git")
        .args([
            "-C",
            repository_path,
            "for-each-ref",
            "--format=%(objectname)%00%(*objectname)%00%(refname)%00%(refname:short)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ])
        .output()
        .map_err(|error| format!("Failed to run git: {error}"))?;

    if !refs_output.status.success() {
        return Err(git_error_message(
            &refs_output.stderr,
            "Failed to read Git refs.",
        ));
    }

    let refs_stdout = String::from_utf8(refs_output.stdout)
        .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;
    parse_graph_refs(&refs_stdout)
}

fn git_head_hash(repository_path: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", repository_path, "rev-parse", "HEAD"])
        .output()
        .map_err(|error| format!("Failed to run git: {error}"))?;

    if !output.status.success() {
        return Err(git_error_message(&output.stderr, "Failed to read Git HEAD."));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("Git returned invalid UTF-8: {error}"))?;

    Ok(stdout.trim().to_string())
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

fn parse_commit_graph_history(output: &str, head_hash: &str) -> Result<Vec<GitGraphCommit>, String> {
    output
        .split('\x1e')
        .filter(|record| !record.trim().is_empty())
        .map(|record| {
            let fields = record
                .trim_start_matches('\n')
                .split('\0')
                .collect::<Vec<_>>();

            if fields.len() != 6 {
                return Err(format!("Git graph output is invalid: {record}"));
            }

            let parents = fields[2]
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let hash = fields[0].to_string();
            let is_head = hash == head_hash;

            Ok(GitGraphCommit::new(
                hash,
                fields[1].to_string(),
                parents,
                fields[3].to_string(),
                fields[4].to_string(),
                fields[5].to_string(),
                is_head,
            ))
        })
        .collect()
}

fn parse_graph_refs(output: &str) -> Result<Vec<GitGraphRef>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let fields = line.split('\0').collect::<Vec<_>>();

            if fields.len() != 4 {
                return Some(Err(format!("Git ref output is invalid: {line}")));
            }

            let object_hash = fields[0];
            let peeled_hash = fields[1];
            let full_name = fields[2];
            let short_name = fields[3];

            if full_name.starts_with("refs/remotes/") && full_name.ends_with("/HEAD") {
                return None;
            }

            let kind = if full_name.starts_with("refs/heads/") {
                GitGraphRefKind::LocalBranch
            } else if full_name.starts_with("refs/remotes/") {
                GitGraphRefKind::RemoteBranch
            } else if full_name.starts_with("refs/tags/") {
                GitGraphRefKind::Tag
            } else {
                return None;
            };
            let target = if peeled_hash.is_empty() {
                object_hash
            } else {
                peeled_hash
            };

            Some(Ok(GitGraphRef::new(
                short_name.to_string(),
                target.to_string(),
                kind,
            )))
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

/// git stderr를 사용자 메시지로 변환. history 외 worktree/branch reader(git-explorer)도
/// 공유하므로 pub로 노출한다.
pub fn git_error_message(stderr: &[u8], fallback: &str) -> String {
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();

    if stderr.is_empty() {
        fallback.to_string()
    } else {
        stderr
    }
}

const MAX_WORKTREE_DIFF_BYTES: usize = 120_000;

/// git CLI 기반 미커밋(working-tree) status/diff 조회 구현.
pub struct GitCliWorktreeStatusReader;

impl GitWorktreeStatusReader for GitCliWorktreeStatusReader {
    fn status(&self, repository_path: &str) -> Result<GitWorktreeChanges, String> {
        // --no-optional-locks: status가 .git/index를 다시 쓰지 않게 해 파일
        // watcher 기반 소비 앱의 되먹임을 차단한다(AW specs/007 research R2).
        let output = Command::new("git")
            .args([
                "--no-optional-locks",
                "-C",
                repository_path,
                "status",
                "--porcelain=v1",
                "-uall",
            ])
            .output()
            .map_err(|error| format!("Failed to run git status: {error}"))?;

        if !output.status.success() {
            return Err(worktree_git_error(
                "Failed to read git status",
                &output.stderr,
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Failed to parse git status output: {error}"))?;
        let files = parse_status_output(&stdout);

        Ok(summarize_changes(repository_path, files))
    }

    fn diff(&self, repository_path: &str, file_path: &str) -> Result<GitWorktreeFileDiff, String> {
        let mut diff = run_worktree_diff(repository_path, file_path, false)?;
        if diff.is_empty() {
            diff = run_worktree_diff(repository_path, file_path, true)?;
        }
        if diff.is_empty() {
            diff = format!(
                "No textual git diff is available for `{file_path}`. The file may be untracked or unchanged relative to the selected diff scope."
            );
        }

        let is_binary = diff.contains("Binary files ") || diff.contains("GIT binary patch");
        let is_truncated = diff.len() > MAX_WORKTREE_DIFF_BYTES;
        if is_truncated {
            diff.truncate(MAX_WORKTREE_DIFF_BYTES);
            diff.push_str("\n\n[diff truncated]");
        }

        Ok(GitWorktreeFileDiff {
            path: file_path.to_string(),
            content: diff,
            is_binary,
            is_truncated,
        })
    }
}

fn run_worktree_diff(
    repository_path: &str,
    file_path: &str,
    cached: bool,
) -> Result<String, String> {
    let mut command = Command::new("git");
    command.args(["-C", repository_path, "diff", "--no-ext-diff"]);
    if cached {
        command.arg("--cached");
    }
    command.args(["--", file_path]);

    let output = command
        .output()
        .map_err(|error| format!("Failed to run git diff: {error}"))?;

    if !output.status.success() {
        return Err(worktree_git_error("Failed to read git diff", &output.stderr));
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

fn summarize_changes(repository_path: &str, files: Vec<GitChangedFile>) -> GitWorktreeChanges {
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
        working_directory: repository_path.to_string(),
        files,
        staged_count,
        unstaged_count,
        untracked_count,
        conflicted_count,
    }
}

fn worktree_git_error(context: &str, stderr: &[u8]) -> String {
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
    fn parses_commit_graph_history() {
        let output = "\
abc123\0abc123\0def456 feed00\0Merge branch\0A Developer\02026-06-25T00:00:00+09:00\x1e
def456\0def456\0\0Initial commit\0A Developer\02026-06-24T00:00:00+09:00\x1e";

        let commits =
            parse_commit_graph_history(output, "abc123").expect("graph history should parse");

        assert_eq!(commits.len(), 2);
        assert!(commits[0].is_head);
        assert!(commits[0].is_merge);
        assert_eq!(commits[0].parents, vec!["def456", "feed00"]);
        assert!(!commits[1].is_merge);
    }

    #[test]
    fn parses_graph_refs() {
        let output = "\
abc123\0\0refs/heads/main\0main
def456\0\0refs/remotes/origin/main\0origin/main
ignore\0\0refs/remotes/origin/HEAD\0origin/HEAD
tagobj\0feed00\0refs/tags/v1.0.0\0v1.0.0
";

        let refs = parse_graph_refs(output).expect("refs should parse");

        assert_eq!(refs.len(), 3);
        assert_eq!(refs[0].kind, GitGraphRefKind::LocalBranch);
        assert_eq!(refs[1].kind, GitGraphRefKind::RemoteBranch);
        assert_eq!(refs[2].kind, GitGraphRefKind::Tag);
        assert_eq!(refs[2].target, "feed00");
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

    #[test]
    fn parses_worktree_status_groups_and_renames() {
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

/// cursor 페이지네이션과 count/refs 첫 페이지 한정을 실제 git 저장소 fixture로
/// 검증한다(AW specs/007 research R8).
#[cfg(test)]
mod pagination_tests {
    use std::{fs, path::Path, process::Command};

    use super::GitCliHistoryReader;
    use crate::ports::GitHistoryReader;

    struct FixtureRepo {
        dir: tempfile::TempDir,
    }

    impl FixtureRepo {
        fn path(&self) -> &str {
            self.dir.path().to_str().expect("fixture path should be utf-8")
        }
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    }

    fn init_fixture_repo(commit_count: usize) -> FixtureRepo {
        let dir = tempfile::tempdir().expect("fixture dir should be created");
        let path = dir.path();
        run_git(path, &["init", "-b", "main"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test"]);

        for index in 0..commit_count {
            fs::write(path.join("file.txt"), format!("content {index}"))
                .expect("fixture file should be written");
            run_git(path, &["add", "."]);
            run_git(path, &["commit", "-m", &format!("commit {index}")]);
        }

        FixtureRepo { dir }
    }

    #[test]
    fn graph_first_page_has_total_and_refs_but_later_pages_skip_them() {
        let repo = init_fixture_repo(3);

        let first_page = GitCliHistoryReader
            .get_commit_graph(repo.path(), 2, 0, None, &[], &[])
            .expect("first graph page should load");

        assert_eq!(first_page.page.total_count, Some(3));
        assert!(first_page.page.has_more);
        assert!(!first_page.refs.is_empty(), "first page should include refs");
        assert_eq!(first_page.commits.len(), 2);

        let cursor = first_page.commits.last().expect("page has commits").hash.clone();
        let second_page = GitCliHistoryReader
            .get_commit_graph(repo.path(), 2, 2, Some(&cursor), &[], &[])
            .expect("second graph page should load");

        assert_eq!(second_page.page.total_count, None, "count must be skipped");
        assert!(second_page.refs.is_empty(), "refs must be skipped");
        assert_eq!(second_page.commits.len(), 1);
        assert!(!second_page.page.has_more);
        assert_ne!(second_page.commits[0].hash, first_page.commits[1].hash);
    }

    #[test]
    fn history_pages_continue_after_cursor_without_recounting() {
        let repo = init_fixture_repo(3);

        let first_page = GitCliHistoryReader
            .list_history(repo.path(), 2, 0, None, &[], &[])
            .expect("first history page should load");

        assert_eq!(first_page.page.total_count, Some(3));
        assert!(first_page.page.has_more);

        let cursor = first_page.commits.last().expect("page has commits").hash.clone();
        let second_page = GitCliHistoryReader
            .list_history(repo.path(), 2, 2, Some(&cursor), &[], &[])
            .expect("second history page should load");

        assert_eq!(second_page.page.total_count, None);
        assert_eq!(second_page.commits.len(), 1);
        assert!(!second_page.page.has_more);
    }

    #[test]
    fn invalid_cursor_is_flagged_for_frontend_reset() {
        let repo = init_fixture_repo(2);

        let page = GitCliHistoryReader
            .get_commit_graph(
                repo.path(),
                2,
                2,
                Some("deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
                &[],
                &[],
            )
            .expect("invalid cursor should not error");

        assert_eq!(page.page.cursor_invalidated, Some(true));
        assert!(page.commits.is_empty());
        assert!(!page.page.has_more);
    }
}
