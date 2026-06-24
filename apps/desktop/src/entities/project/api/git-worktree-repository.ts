import { invoke } from "@tauri-apps/api/core";

import type {
  GitWorktree,
  GitWorktreeCreateInput,
} from "@/entities/project/model/git-worktree";

export async function listGitWorktrees(workingDirectory: string) {
  return invoke<GitWorktree[]>("list_git_worktrees", { workingDirectory });
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
