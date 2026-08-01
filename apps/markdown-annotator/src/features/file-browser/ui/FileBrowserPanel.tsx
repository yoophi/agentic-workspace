import { useMemo, useState } from "react";
import { FileBrowserTree } from "@yoophi/file-browser-react";
import type { FileBrowserRow, FileBrowserSortField } from "@yoophi/file-browser-core";

import { Input } from "@/components/ui/input";
import { MaFileBrowserRow } from "@/shared/ui/file-browser-components";
import { useFileBrowser } from "../model/use-file-browser";

export function FileBrowserPanel({ rootPath, exclusions, selectedPath, onSelect }: { rootPath: string; exclusions: string[]; selectedPath: string | null; onSelect: (path: string) => void }) {
  const [expandedPaths, setExpandedPaths] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState("");
  const [sortBy, setSortBy] = useState<FileBrowserSortField>("name");
  const options = useMemo(() => ({ expandedPaths, searchQuery, sort: { by: sortBy, direction: "asc" as const } }), [expandedPaths, searchQuery, sortBy]);
  const browser = useFileBrowser(rootPath, exclusions, options);
  const toggle = (row: FileBrowserRow) => setExpandedPaths((current) => {
    const next = new Set(current);
    if (next.has(row.path)) next.delete(row.path); else next.add(row.path);
    return next;
  });
  return <FileBrowserPanelView {...browser} searchQuery={searchQuery} sortBy={sortBy} selectedPath={selectedPath} onSearchChange={setSearchQuery} onSortChange={setSortBy} onToggle={toggle} onSelect={(row) => onSelect(row.path)} />;
}

export function FileBrowserPanelView({ rows, scanning, visitedEntries, matchedDocuments, warnings, searchQuery, sortBy, selectedPath, onSearchChange, onSortChange, onToggle, onSelect }: {
  rows: readonly FileBrowserRow[]; scanning: boolean; visitedEntries: number; matchedDocuments: number;
  warnings: readonly { relativePath: string; code: string }[]; searchQuery: string; sortBy: FileBrowserSortField;
  selectedPath: string | null; onSearchChange: (value: string) => void; onSortChange: (value: FileBrowserSortField) => void;
  onToggle: (row: FileBrowserRow) => void; onSelect: (row: FileBrowserRow) => void;
}) {
  return (
    <section aria-label="Markdown 파일 탐색기" className="flex h-full min-h-0 flex-col border-r">
      <header className="space-y-2 border-b p-2">
        <Input aria-label="파일명 또는 경로 검색" value={searchQuery} placeholder="문서 검색" onChange={(event) => onSearchChange(event.target.value)} />
        <label className="flex items-center gap-2 text-xs"><span>정렬</span><select aria-label="문서 정렬" value={sortBy} onChange={(event) => onSortChange(event.target.value as FileBrowserSortField)}><option value="name">이름</option><option value="path">경로</option><option value="modifiedAt">수정 시각</option></select></label>
        <p aria-live="polite" className="text-xs text-muted-foreground">{scanning ? `검색 중 · ${visitedEntries}개 확인 · 문서 ${matchedDocuments}개` : `문서 ${matchedDocuments}개`}</p>
      </header>
      {warnings.length > 0 ? <p role="status" className="p-2 text-xs text-amber-700">읽지 못한 폴더 {warnings.length}개를 건너뛰었습니다.</p> : null}
      {rows.length === 0 && !scanning ? <p className="p-4 text-sm text-muted-foreground">표시할 Markdown 문서가 없습니다.</p> : (
        <FileBrowserTree rows={rows} selectedPath={selectedPath} activePath={selectedPath} ariaLabel="Markdown 문서 트리" height={480} onToggle={onToggle} onSelect={onSelect} renderRow={(row, state) => <MaFileBrowserRow row={row} selected={state.selected} />} />
      )}
    </section>
  );
}
