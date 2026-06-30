import type { ReactNode } from "react";
import { useEffect, useState } from "react";

import {
  AlertCircle,
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  FolderOpen,
  Loader2,
} from "lucide-react";

import type { GitCommitDetail, GitCommitFileChange, GitFileDiff } from "@yoophi/git-graph";

import { cn } from "../lib/cn";
import { fileStatusClassName } from "../lib/styling";
import { buildFileTreeRows, getFileFolderPaths } from "../model/file-tree";
import { DiffViewer } from "./diff-viewer";

type FileChangeView = "tree" | "list";

const NEUTRAL_STATUS_CLASS = "border-border bg-background text-muted-foreground";

export type CommitDetailViewProps = {
  /** 커밋 메타(해시/메시지/작성자·날짜). 없으면 헤더를 생략한다. */
  commit?: GitCommitDetail;
  files: GitCommitFileChange[];
  selectedFilePath?: string;
  onSelectFile: (path: string) => void;
  diff?: GitFileDiff;
  diffLoading?: boolean;
  diffError?: string | null;
  /** Tree/List 토글 제공 여부(기본 true). false면 List 고정. */
  enableTreeView?: boolean;
  /** 파일 상태별 색상 배지 사용 여부(기본 true). */
  showFileStatusColor?: boolean;
  /** Diff 라인 번호 표시 여부(기본 true). */
  showDiffLineNumbers?: boolean;
  /** Diff 뷰어 컨테이너 클래스 오버라이드(예: 최대 높이). */
  diffClassName?: string;
};

/** 커밋 상세 = 메타 헤더 + 변경 파일 목록(tree/list, 상태 색상) + 선택 파일 diff. */
export function CommitDetailView({
  commit,
  files,
  selectedFilePath,
  onSelectFile,
  diff,
  diffLoading = false,
  diffError,
  enableTreeView = true,
  showFileStatusColor = true,
  showDiffLineNumbers = true,
  diffClassName,
}: CommitDetailViewProps) {
  const [fileChangeView, setFileChangeView] = useState<FileChangeView>(
    enableTreeView ? "tree" : "list",
  );
  const [expandedFolders, setExpandedFolders] = useState<ReadonlySet<string>>(() => new Set());

  useEffect(() => {
    setExpandedFolders(getFileFolderPaths(files));
  }, [files]);

  function toggleFolder(path: string) {
    setExpandedFolders((current) => {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }

  const effectiveView: FileChangeView = enableTreeView ? fileChangeView : "list";
  const fileRows = buildFileTreeRows(files, expandedFolders);
  const statusClass = (status: string) =>
    showFileStatusColor ? fileStatusClassName(status) : NEUTRAL_STATUS_CLASS;

  return (
    <div className="grid gap-3">
      {commit ? (
        <div className="grid gap-1">
          <p className="font-mono text-xs text-muted-foreground">{commit.hash}</p>
          <h3 className="break-words text-sm font-medium">{commit.message}</h3>
          <p className="text-sm text-muted-foreground">
            {commit.author} · {commit.date}
          </p>
        </div>
      ) : null}

      <div className="grid gap-3">
        <div className="flex items-center justify-between gap-2">
          <h3 className="text-sm font-medium">Changed files</h3>
          {enableTreeView ? (
            <div className="flex rounded-md border p-0.5">
              <ViewToggle active={effectiveView === "tree"} onClick={() => setFileChangeView("tree")}>
                Tree
              </ViewToggle>
              <ViewToggle active={effectiveView === "list"} onClick={() => setFileChangeView("list")}>
                List
              </ViewToggle>
            </div>
          ) : null}
        </div>

        {files.length === 0 ? (
          <p className="rounded-md border p-3 text-sm text-muted-foreground">No changed files.</p>
        ) : effectiveView === "tree" ? (
          <div className="overflow-hidden rounded-md border text-sm">
            {fileRows.map((row) =>
              row.type === "folder" ? (
                <button
                  aria-expanded={row.isExpanded}
                  className="flex h-8 w-full items-center gap-1 border-b px-2 text-left last:border-b-0 hover:bg-muted/50"
                  key={row.id}
                  onClick={() => toggleFolder(row.path)}
                  style={{ paddingLeft: `${8 + row.depth * 18}px` }}
                  type="button"
                >
                  {row.isExpanded ? (
                    <ChevronDown className="size-3.5 shrink-0 text-muted-foreground" />
                  ) : (
                    <ChevronRight className="size-3.5 shrink-0 text-muted-foreground" />
                  )}
                  {row.isExpanded ? (
                    <FolderOpen className="size-4 shrink-0 text-muted-foreground" />
                  ) : (
                    <Folder className="size-4 shrink-0 text-muted-foreground" />
                  )}
                  <span className="min-w-0 truncate text-muted-foreground">{row.name}</span>
                </button>
              ) : (
                <button
                  className="flex h-8 w-full items-center gap-2 border-b px-2 text-left last:border-b-0 hover:bg-muted/50 data-[selected=true]:bg-muted"
                  data-selected={row.file.path === selectedFilePath}
                  key={row.id}
                  onClick={() => onSelectFile(row.file.path)}
                  style={{ paddingLeft: `${28 + row.depth * 18}px` }}
                  title={row.file.path}
                  type="button"
                >
                  <FileText className="size-4 shrink-0 text-muted-foreground" />
                  <span className="min-w-0 flex-1 truncate font-mono text-xs">{row.name}</span>
                  <span
                    className={cn(
                      "ml-auto shrink-0 rounded-sm border px-1.5 py-0.5 font-mono text-[10px] leading-none",
                      statusClass(row.file.status),
                    )}
                  >
                    {row.file.status}
                  </span>
                </button>
              ),
            )}
          </div>
        ) : (
          <div className="overflow-hidden rounded-md border text-sm">
            <table className="w-full border-collapse">
              <thead>
                <tr className="border-b bg-muted/40 text-xs font-medium text-muted-foreground">
                  <th className="w-20 px-3 py-2 text-left font-medium">Status</th>
                  <th className="px-3 py-2 text-left font-medium">File</th>
                </tr>
              </thead>
              <tbody>
                {files.map((file) => (
                  <tr
                    className="cursor-pointer border-b last:border-b-0 hover:bg-muted/50 data-[selected=true]:bg-muted"
                    data-selected={file.path === selectedFilePath}
                    key={`${file.status}:${file.path}`}
                    onClick={() => onSelectFile(file.path)}
                  >
                    <td className="px-3 py-2 font-mono text-xs">
                      <span
                        className={cn(
                          "rounded-sm border px-1.5 py-0.5 text-[10px] leading-none",
                          statusClass(file.status),
                        )}
                      >
                        {file.status}
                      </span>
                    </td>
                    <td className="max-w-0 truncate px-3 py-2 font-mono text-xs">{file.path}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {!selectedFilePath ? (
          <p className="text-sm text-muted-foreground">Select a changed file to inspect its diff.</p>
        ) : diffLoading ? (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading diff
          </p>
        ) : diffError ? (
          <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <span>{diffError}</span>
          </p>
        ) : diff?.isBinary ? (
          <p className="text-sm text-muted-foreground">
            This file is binary and cannot be displayed as text diff.
          </p>
        ) : diff ? (
          <div className="grid gap-2">
            {diff.isTruncated ? (
              <p className="text-xs text-muted-foreground">Large diff truncated for display.</p>
            ) : null}
            <DiffViewer
              className={diffClassName}
              content={diff.content}
              showLineNumbers={showDiffLineNumbers}
            />
          </div>
        ) : null}
      </div>
    </div>
  );
}

function ViewToggle({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      className={cn(
        "rounded-sm px-2.5 py-1 text-xs font-medium transition-colors",
        active ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground",
      )}
      onClick={onClick}
      type="button"
    >
      {children}
    </button>
  );
}
