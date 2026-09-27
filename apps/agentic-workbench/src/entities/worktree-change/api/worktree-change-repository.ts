import { invoke } from "@/shared/api/transport";

import type { WorktreeChange } from "@/entities/worktree-change/model";

export async function listWorktreeChanges(workingDirectory: string) {
  return invoke<WorktreeChange[]>("list_worktree_changes", { workingDirectory });
}
