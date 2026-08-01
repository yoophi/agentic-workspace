import { createFileBrowserRows } from "@yoophi/file-browser-core";

import type { WorktreeFileEntry } from "../model/types";

export type WorktreeFileTreeRow = WorktreeFileEntry & {
  depth: number;
  isExpanded: boolean;
};

export function mergeWorktreeFileEntries(
  entryGroups: Array<readonly WorktreeFileEntry[] | null | undefined>,
) {
  const seen = new Set<string>();
  return entryGroups
    .flatMap((group) => group ?? [])
    .filter((entry) => {
      if (seen.has(entry.relativePath)) return false;
      seen.add(entry.relativePath);
      return true;
    })
    .sort((left, right) =>
      left.relativePath.toLowerCase().localeCompare(right.relativePath.toLowerCase()),
    );
}

export function buildWorktreeFileTreeRows(
  entries: readonly WorktreeFileEntry[],
  expandedFolders: ReadonlySet<string>,
): WorktreeFileTreeRow[] {
  const entriesByPath = new Map(entries.map((entry) => [entry.relativePath, entry]));
  return createFileBrowserRows(
    entries.map((entry) => ({
      path: entry.relativePath,
      kind: entry.isDir ? "directory" : "file",
      modifiedAt: entry.modifiedMs == null ? undefined : new Date(entry.modifiedMs).toISOString(),
      size: entry.size,
    })),
    {
      expandedPaths: expandedFolders,
      compressSingleDirectoryChains: false,
      directoriesFirst: false,
      sort: { by: "path", direction: "asc" },
    },
  ).flatMap((row) => {
    const entry = entriesByPath.get(row.path);
    return entry ? [{ ...entry, depth: row.depth, isExpanded: row.expanded }] : [];
  });
}
