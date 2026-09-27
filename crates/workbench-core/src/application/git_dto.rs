//! Git·worktree 도메인/git-core 타입 ↔ protocol DTO 변환(038 US2, research R1). wire 동일성은
//! `dto::assert_wire_parity`가 고정한다: 원본 타입을 serde로 직렬화한 JSON == DTO JSON.

use git_core::domain as gitc;
use workbench_protocol::operations::{git as wire_git, worktree as wire_wt};

use crate::domain::{
    git_branch::GitBranch,
    git_remote::GitRemote,
    git_worktree::{GitWorktree, GitWorktreeStatus},
    worktree_change::{WorktreeChange, WorktreeChangeType},
    worktree_file::{
        WorktreeFileEntry, WorktreeFileListKind, WorktreeFileListScope, WorktreeTextFile,
    },
};

pub fn git_remote_dto(remote: &GitRemote) -> wire_git::GitRemoteDto {
    wire_git::GitRemoteDto {
        name: remote.name.clone(),
        fetch_url: remote.fetch_url.clone(),
        push_url: remote.push_url.clone(),
    }
}

pub fn git_branch_dto(branch: &GitBranch) -> wire_git::GitBranchDto {
    wire_git::GitBranchDto {
        name: branch.name.clone(),
        is_current: branch.is_current,
        is_remote: branch.is_remote,
    }
}

fn git_worktree_status_dto(status: &GitWorktreeStatus) -> wire_git::GitWorktreeStatus {
    match status {
        GitWorktreeStatus::Clean => wire_git::GitWorktreeStatus::Clean,
        GitWorktreeStatus::Prunable => wire_git::GitWorktreeStatus::Prunable,
        GitWorktreeStatus::Dirty => wire_git::GitWorktreeStatus::Dirty,
        GitWorktreeStatus::Unknown => wire_git::GitWorktreeStatus::Unknown,
    }
}

pub fn git_worktree_dto(worktree: &GitWorktree) -> wire_git::GitWorktreeDto {
    wire_git::GitWorktreeDto {
        path: worktree.path.clone(),
        head: worktree.head.clone(),
        branch: worktree.branch.clone(),
        status: git_worktree_status_dto(&worktree.status),
        prune_reason: worktree.prune_reason.clone(),
        can_delete: worktree.can_delete,
    }
}

fn worktree_change_type_dto(kind: WorktreeChangeType) -> wire_wt::WorktreeChangeType {
    match kind {
        WorktreeChangeType::Added => wire_wt::WorktreeChangeType::Added,
        WorktreeChangeType::Modified => wire_wt::WorktreeChangeType::Modified,
        WorktreeChangeType::Deleted => wire_wt::WorktreeChangeType::Deleted,
        WorktreeChangeType::Renamed => wire_wt::WorktreeChangeType::Renamed,
        WorktreeChangeType::Untracked => wire_wt::WorktreeChangeType::Untracked,
    }
}

pub fn worktree_change_dto(change: &WorktreeChange) -> wire_wt::WorktreeChangeDto {
    wire_wt::WorktreeChangeDto {
        path: change.path.clone(),
        old_path: change.old_path.clone(),
        change_type: worktree_change_type_dto(change.change_type),
        binary: change.binary,
        diff: change.diff.clone(),
        content: change.content.clone(),
        truncated: change.truncated,
    }
}

fn changed_file_group_dto(group: gitc::GitChangedFileGroup) -> wire_wt::GitChangedFileGroup {
    match group {
        gitc::GitChangedFileGroup::Staged => wire_wt::GitChangedFileGroup::Staged,
        gitc::GitChangedFileGroup::Unstaged => wire_wt::GitChangedFileGroup::Unstaged,
        gitc::GitChangedFileGroup::Untracked => wire_wt::GitChangedFileGroup::Untracked,
        gitc::GitChangedFileGroup::Conflicted => wire_wt::GitChangedFileGroup::Conflicted,
    }
}

pub fn git_worktree_changes_dto(
    changes: &gitc::GitWorktreeChanges,
) -> wire_wt::GitWorktreeChangesDto {
    wire_wt::GitWorktreeChangesDto {
        working_directory: changes.working_directory.clone(),
        files: changes
            .files
            .iter()
            .map(|file| wire_wt::GitChangedFileDto {
                path: file.path.clone(),
                old_path: file.old_path.clone(),
                staged_status: file.staged_status.clone(),
                unstaged_status: file.unstaged_status.clone(),
                group: changed_file_group_dto(file.group),
            })
            .collect(),
        staged_count: changes.staged_count as u64,
        unstaged_count: changes.unstaged_count as u64,
        untracked_count: changes.untracked_count as u64,
        conflicted_count: changes.conflicted_count as u64,
    }
}

pub fn git_worktree_file_diff_dto(
    diff: &gitc::GitWorktreeFileDiff,
) -> wire_wt::GitWorktreeFileDiffDto {
    wire_wt::GitWorktreeFileDiffDto {
        path: diff.path.clone(),
        content: diff.content.clone(),
        is_binary: diff.is_binary,
        is_truncated: diff.is_truncated,
    }
}

pub fn worktree_file_entry_dto(entry: &WorktreeFileEntry) -> wire_wt::WorktreeFileEntryDto {
    wire_wt::WorktreeFileEntryDto {
        name: entry.name.clone(),
        path: entry.path.clone(),
        relative_path: entry.relative_path.clone(),
        is_dir: entry.is_dir,
        size: entry.size,
        modified_ms: entry.modified_ms,
    }
}

pub fn worktree_text_file_dto(file: &WorktreeTextFile) -> wire_wt::WorktreeTextFileDto {
    wire_wt::WorktreeTextFileDto {
        path: file.path.clone(),
        relative_path: file.relative_path.clone(),
        content: file.content.clone(),
        size: file.size,
        truncated: file.truncated,
    }
}

pub fn worktree_file_list_scope_domain(
    scope: wire_wt::WorktreeFileListScopeDto,
) -> WorktreeFileListScope {
    WorktreeFileListScope {
        kind: match scope.kind {
            wire_wt::WorktreeFileListKind::All => WorktreeFileListKind::All,
            wire_wt::WorktreeFileListKind::Markdown => WorktreeFileListKind::Markdown,
        },
        dir: scope.dir,
        depth: scope.depth.map(saturating_usize),
    }
}

/// wire의 u64 → 서비스의 usize. 32비트 대상에서도 넘치지 않게 포화시킨다(값은 어차피 clamp된다).
pub fn saturating_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

fn commit_page_dto(page: &gitc::GitCommitPage) -> wire_wt::GitCommitPageDto {
    wire_wt::GitCommitPageDto {
        offset: page.offset as u64,
        limit: page.limit as u64,
        total_count: page.total_count.map(|count| count as u64),
        has_more: page.has_more,
        cursor_invalidated: page.cursor_invalidated,
    }
}

pub fn git_commit_history_dto(history: &gitc::GitCommitHistory) -> wire_wt::GitCommitHistoryDto {
    wire_wt::GitCommitHistoryDto {
        commits: history
            .commits
            .iter()
            .map(|commit| wire_wt::GitCommitSummaryDto {
                hash: commit.hash.clone(),
                message: commit.message.clone(),
                author: commit.author.clone(),
                date: commit.date.clone(),
            })
            .collect(),
        page: commit_page_dto(&history.page),
    }
}

fn graph_ref_kind_dto(kind: &gitc::GitGraphRefKind) -> wire_wt::GitGraphRefKind {
    match kind {
        gitc::GitGraphRefKind::LocalBranch => wire_wt::GitGraphRefKind::LocalBranch,
        gitc::GitGraphRefKind::RemoteBranch => wire_wt::GitGraphRefKind::RemoteBranch,
        gitc::GitGraphRefKind::Tag => wire_wt::GitGraphRefKind::Tag,
    }
}

pub fn git_commit_graph_dto(graph: &gitc::GitCommitGraph) -> wire_wt::GitCommitGraphDto {
    wire_wt::GitCommitGraphDto {
        commits: graph
            .commits
            .iter()
            .map(|commit| wire_wt::GitGraphCommitDto {
                hash: commit.hash.clone(),
                short_hash: commit.short_hash.clone(),
                parents: commit.parents.clone(),
                message: commit.message.clone(),
                author: commit.author.clone(),
                date: commit.date.clone(),
                is_head: commit.is_head,
                is_merge: commit.is_merge,
            })
            .collect(),
        refs: graph
            .refs
            .iter()
            .map(|reference| wire_wt::GitGraphRefDto {
                name: reference.name.clone(),
                target: reference.target.clone(),
                kind: graph_ref_kind_dto(&reference.kind),
            })
            .collect(),
        page: commit_page_dto(&graph.page),
        layout_hints: wire_wt::GitGraphLayoutHintsDto {
            row_height: graph.layout_hints.row_height,
            max_initial_lanes: graph.layout_hints.max_initial_lanes,
        },
    }
}

pub fn git_commit_detail_dto(detail: &gitc::GitCommitDetail) -> wire_wt::GitCommitDetailDto {
    wire_wt::GitCommitDetailDto {
        hash: detail.hash.clone(),
        message: detail.message.clone(),
        author: detail.author.clone(),
        date: detail.date.clone(),
        files: detail
            .files
            .iter()
            .map(|file| wire_wt::GitCommitFileChangeDto {
                path: file.path.clone(),
                status: file.status.clone(),
            })
            .collect(),
    }
}

pub fn git_file_diff_dto(diff: &gitc::GitFileDiff) -> wire_wt::GitFileDiffDto {
    wire_wt::GitFileDiffDto {
        commit_hash: diff.commit_hash.clone(),
        path: diff.path.clone(),
        content: diff.content.clone(),
        is_binary: diff.is_binary,
        is_truncated: diff.is_truncated,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use workbench_protocol::operations::worktree::WorktreeListFilesInput;

    use super::*;
    use crate::application::dto::{assert_input_roundtrip, assert_wire_parity};

    #[test]
    fn git_remote_wire_parity() {
        for (fetch, push) in [
            (None, None),
            (Some("https://a".to_owned()), Some("ssh://b".to_owned())),
        ] {
            let remote = GitRemote {
                name: "origin".into(),
                fetch_url: fetch,
                push_url: push,
            };
            assert_wire_parity("remote", &remote, &git_remote_dto(&remote));
        }
    }

    #[test]
    fn git_branch_wire_parity() {
        let branch = GitBranch {
            name: "remotes/origin/main".into(),
            is_current: false,
            is_remote: true,
        };
        assert_wire_parity("branch", &branch, &git_branch_dto(&branch));
    }

    #[test]
    fn git_worktree_wire_parity() {
        for status in [
            GitWorktreeStatus::Clean,
            GitWorktreeStatus::Prunable,
            GitWorktreeStatus::Dirty,
            GitWorktreeStatus::Unknown,
        ] {
            for prune_reason in [None, Some("gitdir missing".to_owned())] {
                let worktree = GitWorktree {
                    path: "/r/wt".into(),
                    head: Some("abc".into()),
                    branch: None,
                    status: status.clone(),
                    prune_reason,
                    can_delete: true,
                };
                assert_wire_parity("worktree", &worktree, &git_worktree_dto(&worktree));
            }
        }
    }

    #[test]
    fn worktree_change_wire_parity() {
        for (kind, binary, diff, content) in [
            (
                WorktreeChangeType::Modified,
                false,
                Some("@@".to_owned()),
                None,
            ),
            (
                WorktreeChangeType::Untracked,
                false,
                None,
                Some("new".to_owned()),
            ),
            (WorktreeChangeType::Renamed, true, None, None),
            (WorktreeChangeType::Added, false, None, None),
            (
                WorktreeChangeType::Deleted,
                false,
                Some("-".to_owned()),
                None,
            ),
        ] {
            let change = WorktreeChange {
                path: "a".into(),
                old_path: (kind == WorktreeChangeType::Renamed).then(|| "b".to_owned()),
                change_type: kind,
                binary,
                diff,
                content,
                truncated: false,
            };
            assert_wire_parity("change", &change, &worktree_change_dto(&change));
        }
    }

    #[test]
    fn git_worktree_changes_wire_parity() {
        let changes = gitc::GitWorktreeChanges {
            working_directory: "/r".into(),
            files: vec![
                gitc::GitChangedFile {
                    path: "a".into(),
                    old_path: Some("b".into()),
                    staged_status: Some("R".into()),
                    unstaged_status: None,
                    group: gitc::GitChangedFileGroup::Staged,
                },
                gitc::GitChangedFile {
                    path: "c".into(),
                    old_path: None,
                    staged_status: None,
                    unstaged_status: Some("?".into()),
                    group: gitc::GitChangedFileGroup::Untracked,
                },
            ],
            staged_count: 1,
            unstaged_count: 0,
            untracked_count: 1,
            conflicted_count: 0,
        };
        assert_wire_parity("changes", &changes, &git_worktree_changes_dto(&changes));
    }

    #[test]
    fn git_worktree_file_diff_wire_parity() {
        let diff = gitc::GitWorktreeFileDiff {
            path: "a".into(),
            content: "diff".into(),
            is_binary: false,
            is_truncated: true,
        };
        assert_wire_parity("wt diff", &diff, &git_worktree_file_diff_dto(&diff));
    }

    #[test]
    fn worktree_file_entry_wire_parity() {
        for modified_ms in [None, Some(1_700_000_000_000)] {
            let entry = WorktreeFileEntry {
                name: "a.md".into(),
                path: "/r/a.md".into(),
                relative_path: "a.md".into(),
                is_dir: false,
                size: 3,
                modified_ms,
            };
            assert_wire_parity("entry", &entry, &worktree_file_entry_dto(&entry));
        }
    }

    #[test]
    fn worktree_text_file_wire_parity() {
        let file = WorktreeTextFile {
            path: "/r/a".into(),
            relative_path: "a".into(),
            content: "x".into(),
            size: 1,
            truncated: false,
        };
        assert_wire_parity("text file", &file, &worktree_text_file_dto(&file));
    }

    #[test]
    fn worktree_file_list_scope_input_roundtrip() {
        let input: WorktreeListFilesInput = assert_input_roundtrip(
            "list files",
            json!({"workingDirectory": "/r", "scope": {"kind": "markdown", "dir": "docs", "depth": 1}}),
        );
        let scope = worktree_file_list_scope_domain(input.scope.unwrap());
        assert_eq!(scope.kind, WorktreeFileListKind::Markdown);
        assert_eq!(scope.dir.as_deref(), Some("docs"));
        assert_eq!(scope.depth, Some(1));
        // kind 생략 → all (AW 역직렬화와 같음)
        let dto: workbench_protocol::operations::worktree::WorktreeFileListScopeDto =
            serde_json::from_value(json!({})).unwrap();
        let domain: WorktreeFileListScope = serde_json::from_value(json!({})).unwrap();
        assert_eq!(worktree_file_list_scope_domain(dto), domain);
    }

    fn page(total: Option<usize>, invalidated: Option<bool>) -> gitc::GitCommitPage {
        gitc::GitCommitPage {
            offset: 2,
            limit: 10,
            total_count: total,
            has_more: true,
            cursor_invalidated: invalidated,
        }
    }

    #[test]
    fn git_commit_history_wire_parity() {
        for (total, invalidated) in [(Some(3), None), (None, Some(true))] {
            let history = gitc::GitCommitHistory {
                commits: vec![gitc::GitCommitSummary::new(
                    "h".into(),
                    "m".into(),
                    "a".into(),
                    "2026-01-01T00:00:00+00:00".into(),
                )],
                page: page(total, invalidated),
            };
            assert_wire_parity("history", &history, &git_commit_history_dto(&history));
        }
    }

    #[test]
    fn git_commit_graph_wire_parity() {
        let graph = gitc::GitCommitGraph {
            commits: vec![gitc::GitGraphCommit::new(
                "h".into(),
                "h".into(),
                vec!["p1".into(), "p2".into()],
                "merge".into(),
                "a".into(),
                "d".into(),
                true,
            )],
            refs: vec![
                gitc::GitGraphRef::new(
                    "main".into(),
                    "h".into(),
                    gitc::GitGraphRefKind::LocalBranch,
                ),
                gitc::GitGraphRef::new(
                    "origin/main".into(),
                    "h".into(),
                    gitc::GitGraphRefKind::RemoteBranch,
                ),
                gitc::GitGraphRef::new("v1".into(), "h".into(), gitc::GitGraphRefKind::Tag),
            ],
            page: page(None, None),
            layout_hints: gitc::GitGraphLayoutHints::default_row_layout(),
        };
        assert_wire_parity("graph", &graph, &git_commit_graph_dto(&graph));
    }

    #[test]
    fn git_commit_detail_wire_parity() {
        let detail = gitc::GitCommitDetail::new(
            "h".into(),
            "m".into(),
            "a".into(),
            "d".into(),
            vec![gitc::GitCommitFileChange::new("a.rs".into(), "A".into())],
        );
        assert_wire_parity("detail", &detail, &git_commit_detail_dto(&detail));
    }

    #[test]
    fn git_file_diff_wire_parity() {
        let diff = gitc::GitFileDiff::new("h".into(), "a".into(), "c".into(), true, false);
        assert_wire_parity("file diff", &diff, &git_file_diff_dto(&diff));
    }
}
