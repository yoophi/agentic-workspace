import { invoke } from "@tauri-apps/api/core";

import type { WorktreeChange } from "@/entities/worktree-change/model";

export async function listWorktreeChanges(workingDirectory: string) {
  return invoke<WorktreeChange[]>("list_worktree_changes", { workingDirectory });
}
