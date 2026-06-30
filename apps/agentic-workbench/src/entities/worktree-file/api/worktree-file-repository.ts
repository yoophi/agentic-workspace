import { invoke } from "@tauri-apps/api/core";

import type {
  WorktreeFileEntry,
  WorktreeTextFile,
} from "@/entities/worktree-file/model/types";

export async function listWorktreeFiles(workingDirectory: string) {
  return invoke<WorktreeFileEntry[]>("list_worktree_files", { workingDirectory });
}

export async function readWorktreeTextFile(
  workingDirectory: string,
  path: string,
) {
  return invoke<WorktreeTextFile>("read_worktree_text_file", {
    workingDirectory,
    path,
  });
}

export function startWorktreeWatcher(workingDirectory: string) {
  return invoke<void>("start_worktree_watcher", { workingDirectory });
}

export function stopWorktreeWatcher() {
  return invoke<void>("stop_worktree_watcher");
}
