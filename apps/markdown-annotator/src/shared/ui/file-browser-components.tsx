import type { FileBrowserRow } from "@yoophi/file-browser-core";
import { ChevronDown, ChevronRight, FileText, Folder, FolderOpen } from "lucide-react";

export function MaFileBrowserRow({ row, selected }: { row: FileBrowserRow; selected: boolean }) {
  const Icon = row.kind === "directory" ? (row.expanded ? FolderOpen : Folder) : FileText;
  return (
    <div className="flex h-8 min-w-0 items-center gap-1.5 rounded px-2 text-sm hover:bg-muted data-[selected=true]:bg-muted" data-selected={selected} style={{ paddingLeft: 8 + row.depth * 16 }}>
      {row.kind === "directory" ? (row.expanded ? <ChevronDown className="size-3.5" /> : <ChevronRight className="size-3.5" />) : <span className="w-3.5" />}
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1 truncate">{row.label}</span>
    </div>
  );
}
