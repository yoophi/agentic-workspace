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
};

export async function listWorktreeGitHistory(
  workingDirectory: string,
  options: WorktreeGitPageOptions = {},
) {
  return invoke<GitCommitHistory>("list_worktree_git_history", {
    workingDirectory,
    maxCount: options.maxCount ?? 100,
    offset: options.offset ?? 0,
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
