import { invoke } from "@tauri-apps/api/core";

import type {
  WorktreeFileEntry,
  WorktreeTextFile,
} from "@/entities/worktree-file/model/types";

export type WorktreeFileListScope = {
  /** markdown이면 .md/.markdown/.mdx 파일과 조상 디렉터리만 반환한다. */
  kind?: "all" | "markdown";
  /** 조회 시작 상대 경로(경로 탈출 방지 검증 적용). */
  dir?: string;
  /** 1이면 해당 디렉터리 직계만(폴더 펼침용 lazy loading). */
  depth?: number;
};

export async function listWorktreeFiles(
  workingDirectory: string,
  scope?: WorktreeFileListScope,
) {
  return invoke<WorktreeFileEntry[]>("list_worktree_files", {
    workingDirectory,
    scope,
  });
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
