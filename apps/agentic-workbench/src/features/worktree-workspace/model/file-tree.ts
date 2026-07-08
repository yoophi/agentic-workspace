import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";

export type FileTreeRow = WorktreeFileEntry & {
  depth: number;
  isExpanded: boolean;
};

export function mergeWorktreeFileEntries(
  entryGroups: Array<readonly WorktreeFileEntry[] | null | undefined>,
) {
  const entries = entryGroups.flatMap((group) => group ?? []);
  const seen = new Set<string>();

  return entries
    .filter((entry) => {
      if (seen.has(entry.relativePath)) {
        return false;
      }
      seen.add(entry.relativePath);
      return true;
    })
    .sort((left, right) =>
      left.relativePath
        .toLowerCase()
        .localeCompare(right.relativePath.toLowerCase()),
    );
}

export function buildFileTreeRows(
  entries: WorktreeFileEntry[],
  expandedFolders: ReadonlySet<string>,
): FileTreeRow[] {
  return entries
    .filter((entry) => isEntryVisible(entry, expandedFolders))
    .map((entry) => ({
      ...entry,
      depth: pathDepth(entry.relativePath),
      isExpanded: entry.isDir && expandedFolders.has(entry.relativePath),
    }));
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
