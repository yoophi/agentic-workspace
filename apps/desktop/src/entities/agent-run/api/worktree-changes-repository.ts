import { invoke } from "@tauri-apps/api/core";

import type { WorktreeChange } from "@/entities/agent-run/model/types";

/** worktree에서 HEAD 대비 변경된 파일 목록(diff/내용 포함)을 조회한다. */
export async function listWorktreeChanges(workingDirectory: string) {
  return invoke<WorktreeChange[]>("list_worktree_changes", { workingDirectory });
}
