import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";
import {
  buildWorktreeFileTreeRows,
  mergeWorktreeFileEntries,
  type WorktreeFileTreeRow,
} from "@/entities/worktree-file/lib/file-browser-adapter";

export type FileTreeRow = WorktreeFileTreeRow;

export { mergeWorktreeFileEntries };

export function buildFileTreeRows(
  entries: WorktreeFileEntry[],
  expandedFolders: ReadonlySet<string>,
): FileTreeRow[] {
  return buildWorktreeFileTreeRows(entries, expandedFolders);
}

export function isEntryVisible(
  entry: WorktreeFileEntry,
  expandedFolders: ReadonlySet<string>,
) {
  const segments = entry.relativePath.split("/").filter(Boolean);
  let folderPath = "";

  for (const segment of segments.slice(0, -1)) {
    folderPath = folderPath ? `${folderPath}/${segment}` : segment;
    if (!expandedFolders.has(folderPath)) {
      return false;
    }
  }

  return true;
}

export function isParentDirectoryLoaded(
  selectedPath: string,
  loadedDirs: readonly string[],
) {
  const parentDir = selectedPath.split("/").slice(0, -1).join("/");
  return parentDir === "" || loadedDirs.includes(parentDir);
}

export function pathDepth(path: string) {
  return Math.max(path.split("/").filter(Boolean).length - 1, 0);
}
