import { invoke } from "@tauri-apps/api/core";
import type { WorktreeWorkspaceLayout } from "@/entities/worktree-workspace-layout/model/types";

export function getWorktreeWorkspaceLayout(workingDirectory: string) {
  return invoke<WorktreeWorkspaceLayout | null>("get_worktree_workspace_layout", { workingDirectory });
}

export function saveWorktreeWorkspaceLayout(layout: WorktreeWorkspaceLayout) {
  return invoke<WorktreeWorkspaceLayout>("save_worktree_workspace_layout", { layout });
}
