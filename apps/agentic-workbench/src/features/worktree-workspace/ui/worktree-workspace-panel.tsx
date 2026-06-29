import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  AlertCircleIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  FileIcon,
  FileTextIcon,
  FolderIcon,
  FolderOpenIcon,
  GitBranchIcon,
  GitCommitIcon,
  GitPullRequestIcon,
  ListTreeIcon,
  Loader2Icon,
  RefreshCwIcon,
} from "lucide-react";
import {
  Group as ResizablePanelGroup,
  Panel as ResizablePanel,
  Separator as ResizableHandle,
} from "react-resizable-panels";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import { worktreeFileQueryKeys } from "@/entities/worktree-file/api/query-keys";
import {
  listWorktreeFiles,
  readWorktreeTextFile,
} from "@/entities/worktree-file/api/worktree-file-repository";
import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type WorktreeWorkspacePanelProps = {
  worktree: GitWorktree;
};

type WorkspaceTabId = "git" | "files" | "markdown";
type GitHistoryView = "graph" | "list";

type FileTreeRow = WorktreeFileEntry & {
  depth: number;
  isExpanded: boolean;
};

const workspaceTabs: Array<{
  id: WorkspaceTabId;
  label: string;
  icon: typeof GitBranchIcon;
}> = [
  { id: "git", label: "Git", icon: GitBranchIcon },
  { id: "files", label: "Files", icon: FileIcon },
  { id: "markdown", label: "Markdown", icon: FileTextIcon },
];

export function WorktreeWorkspacePanel({ worktree }: WorktreeWorkspacePanelProps) {
  const [selectedTab, setSelectedTab] = useState<WorkspaceTabId>("git");
  const [gitHistoryView, setGitHistoryView] = useState<GitHistoryView>("graph");

  return (
    <section className="flex h-full min-h-0 flex-col overflow-hidden border-l bg-background">
      <header className="flex shrink-0 items-center justify-between gap-3 border-b px-4 py-2">
        <div className="flex min-w-0 items-center gap-2">
          <GitPullRequestIcon className="size-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0">
            <div className="flex min-w-0 items-center gap-2">
              <span className="truncate text-sm font-medium">Workspace</span>
              <Badge variant={worktree.status === "dirty" ? "destructive" : "secondary"} className="shrink-0">
                {worktree.status}
              </Badge>
            </div>
            <EllipsisPopoverText
              value={worktree.path}
              className="min-w-0 font-mono text-xs text-muted-foreground"
              contentClassName="font-mono text-xs"
            />
          </div>
        </div>
        <div className="flex shrink-0 rounded-md border p-0.5" role="tablist" aria-label="Worktree workspace">
          {workspaceTabs.map((tab) => {
            const Icon = tab.icon;
            const isSelected = selectedTab === tab.id;

            return (
              <Button
                key={tab.id}
                type="button"
                size="sm"
                variant={isSelected ? "secondary" : "ghost"}
                role="tab"
                aria-selected={isSelected}
                onClick={() => setSelectedTab(tab.id)}
              >
                <Icon data-icon="inline-start" />
                {tab.label}
              </Button>
            );
          })}
        </div>
      </header>

      <div className="min-h-0 flex-1" role="tabpanel">
        {selectedTab === "git" ? (
          <GitWorkspaceTab
            worktree={worktree}
            historyView={gitHistoryView}
            onHistoryViewChange={setGitHistoryView}
          />
        ) : selectedTab === "files" ? (
          <FileWorkspaceTab worktree={worktree} />
        ) : (
          <PlaceholderWorkspaceTab
            icon={FileTextIcon}
            title="Markdown files"
            description="Markdown 파일만 필터링해 표시할 영역입니다."
            detailTitle="Markdown preview"
            detailDescription="Markdown preview와 annotation UI를 이 영역에 통합합니다."
          />
        )}
      </div>
    </section>
  );
}

function GitWorkspaceTab({
  worktree,
  historyView,
  onHistoryViewChange,
}: {
  worktree: GitWorktree;
  historyView: GitHistoryView;
  onHistoryViewChange: (view: GitHistoryView) => void;
}) {
  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full min-h-0">
      <ResizablePanel id="git-workspace-nav" defaultSize="42%" minSize="280px">
        <div className="flex h-full min-h-0 flex-col border-r">
          <section className="shrink-0 border-b p-4">
            <div className="flex items-center justify-between gap-2">
              <div className="min-w-0">
                <h2 className="truncate text-sm font-medium">Git status</h2>
                <p className="mt-1 truncate text-xs text-muted-foreground">
                  {worktree.branch || "detached"} · {worktree.status}
                </p>
              </div>
              <Badge variant="outline" className="shrink-0 font-mono">
                {worktree.branch || "detached"}
              </Badge>
            </div>
          </section>

          <section className="flex min-h-0 flex-1 flex-col">
            <header className="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-3">
              <div className="flex min-w-0 items-center gap-2">
                <GitCommitIcon className="size-4 shrink-0 text-muted-foreground" />
                <h2 className="truncate text-sm font-medium">Commit log</h2>
              </div>
              <div className="flex shrink-0 rounded-md border p-0.5">
                <Button
                  type="button"
                  size="sm"
                  variant={historyView === "graph" ? "secondary" : "ghost"}
                  onClick={() => onHistoryViewChange("graph")}
                >
                  Graph
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant={historyView === "list" ? "secondary" : "ghost"}
                  onClick={() => onHistoryViewChange("list")}
                >
                  List
                </Button>
              </div>
            </header>
            <div className="min-h-0 flex-1 overflow-auto p-4">
              <EmptyPanel
                title={historyView === "graph" ? "Git graph 연결 예정" : "Commit list 연결 예정"}
                description="git-explorer의 history graph/list 컴포넌트를 분리한 뒤 이 영역에 통합합니다."
              />
            </div>
          </section>
        </div>
      </ResizablePanel>

      <ResizableHandle
        aria-label="Git workspace detail 영역 크기 조정"
        className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        <div className="relative z-10 h-12 w-1 rounded-full bg-border transition-colors" />
      </ResizableHandle>

      <ResizablePanel id="git-workspace-detail" minSize="320px">
        <div className="flex h-full min-h-0 flex-col">
          <header className="shrink-0 border-b px-4 py-3">
            <div className="flex min-w-0 items-center gap-2">
              <GitCommitIcon className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <h2 className="truncate text-sm font-medium">Commit detail</h2>
                <p className="truncate text-xs text-muted-foreground">
                  commit 선택 시 변경 파일과 diff를 표시합니다.
                </p>
              </div>
            </div>
          </header>
          <div className="min-h-0 flex-1 overflow-auto p-4">
            <EmptyPanel
              title="선택된 commit 없음"
              description="왼쪽 graph 또는 commit list에서 commit을 선택하면 상세 정보를 표시합니다."
            />
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function FileWorkspaceTab({ worktree }: { worktree: GitWorktree }) {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set());
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const filesQuery = useQuery({
    queryKey: worktreeFileQueryKeys.list(worktree.path),
    queryFn: () => listWorktreeFiles(worktree.path),
  });
  const previewQuery = useQuery({
    enabled: selectedFilePath !== null,
    queryKey: selectedFilePath
      ? worktreeFileQueryKeys.textFile(worktree.path, selectedFilePath)
      : worktreeFileQueryKeys.textFile(worktree.path, ""),
    queryFn: () => readWorktreeTextFile(worktree.path, selectedFilePath ?? ""),
  });
  const rows = useMemo(
    () => buildFileTreeRows(filesQuery.data ?? [], expandedFolders),
    [filesQuery.data, expandedFolders],
  );
  const selectedFile = filesQuery.data?.find(
    (entry) => !entry.isDir && entry.relativePath === selectedFilePath,
  );

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

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full min-h-0">
      <ResizablePanel id="file-workspace-tree" defaultSize="42%" minSize="280px">
        <div className="flex h-full min-h-0 flex-col border-r">
          <header className="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-3">
            <div className="flex min-w-0 items-center gap-2">
              <ListTreeIcon className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <h2 className="truncate text-sm font-medium">File tree</h2>
                <p className="truncate text-xs text-muted-foreground">
                  {filesQuery.data?.length ?? 0} visible items
                </p>
              </div>
            </div>
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="File tree 새로고침"
              disabled={filesQuery.isFetching}
              onClick={() => void filesQuery.refetch()}
            >
              <RefreshCwIcon className={cn(filesQuery.isFetching && "animate-spin")} />
            </Button>
          </header>

          <div className="min-h-0 flex-1 overflow-auto p-2">
            {filesQuery.isLoading ? (
              <InlineState icon={Loader2Icon} title="파일 목록을 불러오는 중입니다." spinning />
            ) : filesQuery.isError ? (
              <InlineState
                icon={AlertCircleIcon}
                title="파일 목록을 불러오지 못했습니다."
                description={String(filesQuery.error)}
                variant="destructive"
              />
            ) : rows.length === 0 ? (
              <EmptyPanel
                title="표시할 파일 없음"
                description="숨김 파일과 빌드 산출물을 제외한 파일이 없습니다."
                className="min-h-56"
              />
            ) : (
              <div className="flex flex-col text-sm">
                {rows.map((row) => (
                  <FileTreeRowButton
                    key={row.relativePath}
                    row={row}
                    selected={row.relativePath === selectedFilePath}
                    onToggleFolder={toggleFolder}
                    onSelectFile={setSelectedFilePath}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </ResizablePanel>

      <ResizableHandle
        aria-label="File detail 영역 크기 조정"
        className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        <div className="relative z-10 h-12 w-1 rounded-full bg-border transition-colors" />
      </ResizableHandle>

      <ResizablePanel id="file-workspace-preview" minSize="320px">
        <div className="flex h-full min-h-0 flex-col">
          <header className="shrink-0 border-b px-4 py-3">
            <div className="flex min-w-0 items-center gap-2">
              <FileTextIcon className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <h2 className="truncate text-sm font-medium">File preview</h2>
                <p className="truncate font-mono text-xs text-muted-foreground">
                  {selectedFile?.relativePath ?? "No file selected"}
                </p>
              </div>
            </div>
          </header>
          <div className="min-h-0 flex-1 overflow-auto p-4">
            {selectedFilePath === null ? (
              <EmptyPanel
                title="파일을 선택하세요"
                description="왼쪽 file tree에서 텍스트 파일을 선택하면 내용을 미리보기합니다."
              />
            ) : previewQuery.isLoading ? (
              <InlineState icon={Loader2Icon} title="파일을 읽는 중입니다." spinning />
            ) : previewQuery.isError ? (
              <InlineState
                icon={AlertCircleIcon}
                title="미리보기를 표시할 수 없습니다."
                description={String(previewQuery.error)}
                variant="destructive"
              />
            ) : previewQuery.data ? (
              <div className="flex h-full min-h-0 flex-col rounded-md border bg-muted/20">
                <div className="flex shrink-0 items-center justify-between gap-2 border-b px-3 py-2 text-xs text-muted-foreground">
                  <span className="min-w-0 truncate font-mono">{previewQuery.data.relativePath}</span>
                  <span className="shrink-0">
                    {formatBytes(previewQuery.data.size)}
                    {previewQuery.data.truncated ? " · truncated" : ""}
                  </span>
                </div>
                <pre className="min-h-0 flex-1 overflow-auto p-3 text-xs leading-5">
                  <code>{previewQuery.data.content}</code>
                </pre>
              </div>
            ) : null}
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function FileTreeRowButton({
  row,
  selected,
  onToggleFolder,
  onSelectFile,
}: {
  row: FileTreeRow;
  selected: boolean;
  onToggleFolder: (path: string) => void;
  onSelectFile: (path: string) => void;
}) {
  const Icon = row.isDir ? (row.isExpanded ? FolderOpenIcon : FolderIcon) : FileIcon;

  return (
    <button
      type="button"
      className="flex h-8 w-full min-w-0 items-center gap-1.5 rounded-sm px-2 text-left hover:bg-muted data-[selected=true]:bg-muted"
      data-selected={selected}
      style={{ paddingLeft: `${8 + row.depth * 16}px` }}
      onClick={() => {
        if (row.isDir) {
          onToggleFolder(row.relativePath);
        } else {
          onSelectFile(row.relativePath);
        }
      }}
    >
      {row.isDir ? (
        row.isExpanded ? (
          <ChevronDownIcon className="size-3.5 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRightIcon className="size-3.5 shrink-0 text-muted-foreground" />
        )
      ) : (
        <span className="w-3.5 shrink-0" />
      )}
      <Icon className="size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1 truncate">{row.name}</span>
      {!row.isDir ? (
        <span className="shrink-0 text-xs text-muted-foreground">{formatBytes(row.size)}</span>
      ) : null}
    </button>
  );
}

function PlaceholderWorkspaceTab({
  icon: Icon,
  title,
  description,
  detailTitle,
  detailDescription,
}: {
  icon: typeof FileIcon;
  title: string;
  description: string;
  detailTitle: string;
  detailDescription: string;
}) {
  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full min-h-0">
      <ResizablePanel id={`${title}-nav`} defaultSize="42%" minSize="260px">
        <div className="flex h-full min-h-0 flex-col border-r">
          <header className="flex shrink-0 items-center gap-2 border-b px-4 py-3">
            <Icon className="size-4 shrink-0 text-muted-foreground" />
            <h2 className="truncate text-sm font-medium">{title}</h2>
          </header>
          <div className="min-h-0 flex-1 overflow-auto p-4">
            <EmptyPanel title={title} description={description} />
          </div>
        </div>
      </ResizablePanel>

      <ResizableHandle
        aria-label={`${title} detail 영역 크기 조정`}
        className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        <div className="relative z-10 h-12 w-1 rounded-full bg-border transition-colors" />
      </ResizableHandle>

      <ResizablePanel id={`${title}-detail`} minSize="300px">
        <div className="flex h-full min-h-0 flex-col">
          <header className="shrink-0 border-b px-4 py-3">
            <h2 className="truncate text-sm font-medium">{detailTitle}</h2>
          </header>
          <div className="min-h-0 flex-1 overflow-auto p-4">
            <EmptyPanel title={detailTitle} description={detailDescription} />
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function InlineState({
  icon: Icon,
  title,
  description,
  spinning = false,
  variant = "muted",
}: {
  icon: typeof AlertCircleIcon;
  title: string;
  description?: string;
  spinning?: boolean;
  variant?: "muted" | "destructive";
}) {
  return (
    <div
      className={cn(
        "flex min-h-40 items-center justify-center rounded-md border border-dashed p-4 text-center",
        variant === "destructive"
          ? "border-destructive/30 bg-destructive/5 text-destructive"
          : "bg-muted/20",
      )}
    >
      <div className="max-w-sm">
        <Icon
          className={cn(
            "mx-auto size-5",
            spinning && "animate-spin",
            variant === "muted" && "text-muted-foreground",
          )}
        />
        <p className="mt-2 text-sm font-medium">{title}</p>
        {description ? (
          <p className="mt-1 break-words text-sm leading-6 text-muted-foreground">
            {description}
          </p>
        ) : null}
      </div>
    </div>
  );
}

function EmptyPanel({
  title,
  description,
  className,
}: {
  title: string;
  description: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex min-h-40 items-center justify-center rounded-md border border-dashed bg-muted/20 p-4 text-center",
        className,
      )}
    >
      <div className="max-w-sm">
        <p className="text-sm font-medium">{title}</p>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
    </div>
  );
}

function buildFileTreeRows(
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

function isEntryVisible(
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

function pathDepth(path: string) {
  return Math.max(path.split("/").filter(Boolean).length - 1, 0);
}

function formatBytes(bytes: number) {
  if (bytes === 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB"];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** exponent;
  return `${value >= 10 || exponent === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[exponent]}`;
}
