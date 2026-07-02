import { invoke } from "@tauri-apps/api/core";

import type {
  GitWorktree,
  GitWorktreeCreateInput,
} from "@/entities/project/model/git-worktree";
import type {
  GitWorktreeChanges,
  GitWorktreeFileDiff,
} from "@/entities/project/model/git-worktree-changes";

export type ListGitWorktreesOptions = {
  /** false면 worktree별 clean/dirty 계산을 건너뛰고 status가 "unknown"으로 온다. */
  includeStatus?: boolean;
};

export async function listGitWorktrees(
  workingDirectory: string,
  options?: ListGitWorktreesOptions,
) {
  return invoke<GitWorktree[]>("list_git_worktrees", {
    workingDirectory,
    includeStatus: options?.includeStatus,
  });
}

export async function createGitWorktree(
  workingDirectory: string,
  input: GitWorktreeCreateInput,
) {
  return invoke("create_git_worktree", { workingDirectory, input });
}

export async function deleteGitWorktree(
  workingDirectory: string,
  path: string,
) {
  return invoke("delete_git_worktree", { workingDirectory, path });
}

export async function getWorktreeChanges(workingDirectory: string) {
  return invoke<GitWorktreeChanges>("get_worktree_changes", { workingDirectory });
}

export async function getWorktreeFileDiff(workingDirectory: string, path: string) {
  return invoke<GitWorktreeFileDiff>("get_worktree_file_diff", { workingDirectory, path });
}

export type OpenWorktreeWindowMode = "window" | "tab";

export async function openWorktreeWindow(
  projectId: string,
  projectName: string,
  worktreePath: string,
  mode: OpenWorktreeWindowMode,
) {
  return invoke<void>("open_worktree_window", {
    projectId,
    projectName,
    worktreePath,
    mode,
  });
}
