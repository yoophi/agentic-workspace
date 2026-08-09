import { FileBrowserTree } from "@yoophi/file-browser-react";
import { ChevronDownIcon, ChevronRightIcon, FileIcon, FolderIcon, FolderOpenIcon } from "lucide-react";

import type { FileTreeRow } from "@/features/worktree-workspace/model/file-tree";

export function WorktreeFileBrowserTree({
  rows,
  selectedPath,
  onToggleFolder,
  onSelectFile,
}: {
  rows: readonly FileTreeRow[];
  selectedPath: string | null;
  onToggleFolder: (path: string) => void;
  onSelectFile: (path: string) => void;
}) {
  const sourceByPath = new Map(rows.map((row) => [row.relativePath, row]));
  const browserRows = rows.map((row) => ({
    id: `${row.isDir ? "directory" : "file"}:${row.relativePath}`,
    path: row.relativePath,
    label: row.name,
    kind: row.isDir ? "directory" as const : "file" as const,
    depth: row.depth,
    expanded: row.isExpanded,
    hasChildren: row.isDir,
    size: row.size,
    matchRanges: [],
    chainPaths: [row.relativePath],
  }));

  return (
    <FileBrowserTree
      rows={browserRows}
      selectedPath={selectedPath}
      activePath={selectedPath}
      ariaLabel="파일 트리"
      height={Math.max(32, Math.min(480, browserRows.length * 32))}
      onToggle={(row) => onToggleFolder(row.path)}
      onSelect={(row) => onSelectFile(row.path)}
      renderRow={(row, state) => {
        const source = sourceByPath.get(row.path);
        const Icon = row.kind === "directory" ? (row.expanded ? FolderOpenIcon : FolderIcon) : FileIcon;
        return (
          <div
            className="flex h-8 w-full min-w-0 items-center gap-1.5 rounded-sm px-2 text-left hover:bg-muted data-[selected=true]:bg-muted"
            data-selected={state.selected}
            style={{ paddingLeft: `${8 + row.depth * 16}px` }}
          >
            {row.kind === "directory" ? (
              row.expanded ? <ChevronDownIcon className="size-3.5 shrink-0 text-muted-foreground" /> : <ChevronRightIcon className="size-3.5 shrink-0 text-muted-foreground" />
            ) : <span className="w-3.5 shrink-0" />}
            <Icon className="size-4 shrink-0 text-muted-foreground" />
            <span className="min-w-0 flex-1 truncate">{row.label}</span>
            {row.kind === "file" && source ? <span className="shrink-0 text-xs text-muted-foreground">{formatBytes(source.size)}</span> : null}
          </div>
        );
      }}
    />
  );
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
