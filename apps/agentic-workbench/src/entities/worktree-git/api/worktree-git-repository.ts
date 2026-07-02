import { invoke } from "@tauri-apps/api/core";

import type {
  GitCommitDetail,
  GitCommitGraph,
  GitCommitHistory,
  GitFileDiff,
} from "@/entities/worktree-git/model/types";

export type WorktreeGitPageOptions = {
  maxCount?: number;
  offset?: number;
  /**
   * 마지막으로 받은 commit hash. 이력 재작성(rebase 등)을 감지하고, cursor가
   * 있는 페이지는 백엔드가 count/refs 재계산을 생략한다(specs/007 R8).
   */
  cursor?: string;
};

export async function listWorktreeGitHistory(
  workingDirectory: string,
  options: WorktreeGitPageOptions = {},
) {
  return invoke<GitCommitHistory>("list_worktree_git_history", {
    workingDirectory,
    maxCount: options.maxCount ?? 100,
    offset: options.offset ?? 0,
    cursor: options.cursor,
  });
}

export async function getWorktreeGitGraph(
  workingDirectory: string,
  options: WorktreeGitPageOptions = {},
) {
  return invoke<GitCommitGraph>("get_worktree_git_graph", {
    workingDirectory,
    maxCount: options.maxCount ?? 300,
    offset: options.offset ?? 0,
    cursor: options.cursor,
  });
}

export async function getWorktreeCommitDetail(
  workingDirectory: string,
  commitHash: string,
) {
  return invoke<GitCommitDetail>("get_worktree_commit_detail", {
    workingDirectory,
    commitHash,
  });
}

export async function getWorktreeCommitFileDiff(
  workingDirectory: string,
  commitHash: string,
  path: string,
) {
  return invoke<GitFileDiff>("get_worktree_commit_file_diff", {
    workingDirectory,
    commitHash,
    path,
  });
}
