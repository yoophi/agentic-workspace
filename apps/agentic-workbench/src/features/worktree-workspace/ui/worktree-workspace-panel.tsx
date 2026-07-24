import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useInfiniteQuery, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertCircleIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  FileDiffIcon,
  FileIcon,
  FileTextIcon,
  FolderIcon,
  FolderOpenIcon,
  FolderKanbanIcon,
  GitBranchIcon,
  GitCommitIcon,
  GitPullRequestIcon,
  ListTreeIcon,
  Loader2Icon,
  PencilLineIcon,
  RefreshCwIcon,
  SendIcon,
  StickyNoteIcon,
  Trash2Icon,
  XIcon,
} from "lucide-react";
import {
  Group as ResizablePanelGroup,
  Panel as ResizablePanel,
  Separator as ResizableHandle,
} from "react-resizable-panels";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import {
  getWorktreeChanges,
  getWorktreeFileDiff,
} from "@/entities/project/api/git-worktree-repository";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import { WorktreeStatusBadge } from "@/entities/project/ui/worktree-status-badge";
import { worktreeFileQueryKeys } from "@/entities/worktree-file/api/query-keys";
import {
  listWorktreeFiles,
  listSpeckitMarkdownFiles,
  readWorktreeTextFile,
  startWorktreeWatcher,
  stopWorktreeWatcher,
} from "@/entities/worktree-file/api/worktree-file-repository";
import { isMarkdownPath } from "@/entities/worktree-file/lib/is-markdown-path";
import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";
import { worktreeGitQueryKeys } from "@/entities/worktree-git/api/query-keys";
import {
  getWorktreeCommitDetail,
  getWorktreeCommitFileDiff,
  getWorktreeGitGraph,
  listWorktreeGitHistory,
} from "@/entities/worktree-git/api/worktree-git-repository";
import type {
  GitCommitGraph,
  GitCommitSummary,
} from "@/entities/worktree-git/model/types";
import {
  computeGitGraphRows,
  getMaxGraphLane,
} from "@/features/worktree-workspace/model/git-graph-layout";
import {
  CommitDetailView,
  HistoryGraphView,
  InfiniteLoadSentinel,
  WorktreeChangesView,
  combineGitCommitGraphPages,
  combineGitCommitHistoryPages,
  getNextGitPageParam,
  initialGitPageParam,
  refsByTarget,
  useVirtualRows,
} from "@yoophi/git-ui";
import {
  extractTocEntries,
  formatAnnotationsForAgent,
  isFullBlockAnnotation,
  parseMarkdownToBlocks,
} from "@yoophi/markdown-annotation-core";
import type {
  AnnotationDraft,
  AnnotationAnchor,
  AnnotationType,
  MarkdownBlock,
} from "@yoophi/markdown-annotation-core/types";
import {
  AnnotationInputDialog,
  MarkdownViewer,
  buildViewerAnnotationMaps,
  getSelectionAnchors,
  getSelectionRects,
  scrollToBlock,
  type SelectionRect,
} from "@yoophi/markdown-annotation-react";
import {
  autoRefreshQueryOptions,
  findStaleCommitSelection,
  findStaleFileSelection,
  WORKTREE_CHANGED_EVENT,
  type WorktreeChangedEvent,
  type StaleSelection,
} from "@yoophi/workspace-auto-refresh";
import { annotationDialogComponents } from "@/features/worktree-workspace/ui/annotation-dialog-components";
import {
  buildFileTreeRows,
  isParentDirectoryLoaded,
  mergeWorktreeFileEntries,
  type FileTreeRow,
} from "@/features/worktree-workspace/model/file-tree";
import {
  buildSpeckitFeatures,
  getTaskDocumentPaths,
} from "@/features/worktree-workspace/model/speckit-files";
import { measureSessionMilestone } from "@/shared/lib/session-perf";
import { cn } from "@/lib/utils";
import { markdownViewerComponents } from "@/features/worktree-workspace/ui/markdown-viewer-components";
import { MarkdownPreviewToc } from "@/features/worktree-workspace/ui/markdown-preview-toc";
import { SpeckitFilesPanel } from "@/features/worktree-workspace/ui/speckit-files-panel";
import { useMarkdownAnnotationWorkspace } from "@/features/worktree-workspace/model/use-markdown-annotation-workspace";
import { MarkdownAnnotationWorkspace } from "@/features/worktree-workspace/ui/markdown-annotation-workspace";
import type { WorkspacePanelId } from "@/features/worktree-workspace/model/workspace-layout";

type WorktreeWorkspacePanelProps = {
  worktree: GitWorktree;
  onSendAnnotationPrompt?: (prompt: string) => void;
  initialTab?: WorkspaceTabId;
  selectedPanel?: WorkspacePanelId;
};

type WorkspaceTabId = "git" | "files" | "markdown" | "speckit";
type GitHistoryView = "graph" | "list";

type AnnotationDraftTarget =
  | {
      kind: "block";
      block: MarkdownBlock;
    }
  | {
      kind: "selection";
      anchors: AnnotationAnchor[];
      text: string;
    };

// CommitListView 고정 row 높이(virtualization용). 두 줄 레이아웃 기준.
const COMMIT_LIST_ROW_HEIGHT = 56;

const workspaceTabs: Array<{
  id: WorkspaceTabId;
  label: string;
  icon: typeof GitBranchIcon;
}> = [
  { id: "git", label: "Git", icon: GitBranchIcon },
  { id: "files", label: "Files", icon: FileIcon },
  { id: "markdown", label: "Markdown", icon: FileTextIcon },
  { id: "speckit", label: "Speckit", icon: FolderKanbanIcon },
];

function fullBlockAnchor(block: MarkdownBlock): AnnotationAnchor {
  return {
    blockId: block.id,
    startLine: block.startLine,
    endLine: block.endLine,
    startOffset: 0,
    endOffset: block.content.length,
    selectedText: block.rawContent,
  };
}

function createAnnotationFromAnchor({
  anchor,
  block,
  comment,
  createdAt,
  fileName,
  groupId,
  type,
}: {
  anchor: AnnotationAnchor;
  block?: MarkdownBlock;
  comment: string;
  createdAt: string;
  fileName: string;
  groupId?: string;
  type: AnnotationType;
}): AnnotationDraft {
  return {
    id: crypto.randomUUID(),
    groupId,
    fileName,
    anchor,
    selectedText: anchor.selectedText ?? block?.rawContent ?? "",
    comment,
    type,
    createdAt,
  };
}

export function WorktreeWorkspacePanel({
  worktree,
  onSendAnnotationPrompt,
  initialTab = "git",
  selectedPanel,
}: WorktreeWorkspacePanelProps) {
  const [selectedTab, setSelectedTab] = useState<WorkspaceTabId>(selectedPanel ?? initialTab);
  const [gitHistoryView, setGitHistoryView] = useState<GitHistoryView>("graph");
  const queryClient = useQueryClient();
  // watcher 구독을 유지한 채 최신 탭을 참조하기 위한 ref. effect 의존성에 탭을
  // 넣으면 탭 전환마다 watcher가 재시작되므로 ref로 분리한다.
  const selectedTabRef = useRef(selectedTab);

  useEffect(() => {
    selectedTabRef.current = selectedTab;
  }, [selectedTab]);
  useEffect(() => { if (selectedPanel) setSelectedTab(selectedPanel); }, [selectedPanel]);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    startWorktreeWatcher(worktree.path).catch((error) => {
      console.error("Failed to start worktree watcher", error);
    });

    // 선별 invalidation(contracts §5): 활성 탭에 필요한 query만 즉시 refetch
    // 대상으로 만들고, 비활성 query는 stale 표시만 남겨 다음 mount 때 갱신한다.
    listen<WorktreeChangedEvent>(WORKTREE_CHANGED_EVENT, (event) => {
      if (disposed || event.payload.workingDirectory !== worktree.path) {
        return;
      }

      const activeTab = selectedTabRef.current;

      // 파일 목록 전체 rescan(WalkDir)은 파일 트리가 화면에 있을 때만 즉시 필요하다.
      void queryClient.invalidateQueries({
        queryKey: worktreeFileQueryKeys.list(worktree.path),
        refetchType: activeTab === "git" ? "none" : "active",
      });
      void queryClient.invalidateQueries({
        queryKey: worktreeFileQueryKeys.textFiles(worktree.path),
        refetchType: activeTab === "git" ? "none" : "active",
      });
      void queryClient.invalidateQueries({
        queryKey: worktreeFileQueryKeys.speckit(worktree.path),
        refetchType: activeTab === "speckit" ? "active" : "none",
      });
      void queryClient.invalidateQueries({ queryKey: projectQueryKeys.worktreeChanges(worktree.path) });

      if (event.payload.kind === "file") {
        return;
      }

      void queryClient.invalidateQueries({ queryKey: worktreeGitQueryKeys.history(worktree.path) });
      void queryClient.invalidateQueries({ queryKey: worktreeGitQueryKeys.graph(worktree.path) });
      void queryClient.invalidateQueries({
        queryKey: ["worktree-git", "commit-detail", worktree.path],
      });
      void queryClient.invalidateQueries({
        queryKey: ["worktree-git", "file-diff", worktree.path],
      });
    })
      .then((dispose) => {
        unlisten = dispose;
      })
      .catch((error) => {
        console.error("Failed to listen for worktree changes", error);
      });

    return () => {
      disposed = true;
      unlisten?.();
      stopWorktreeWatcher().catch((error) => {
        console.error("Failed to stop worktree watcher", error);
      });
    };
  }, [queryClient, worktree.path]);

  return (
    <section className="flex h-full min-h-0 flex-col overflow-hidden border-l bg-background">
      <header className="flex h-11 shrink-0 items-center justify-between gap-3 border-b px-4">
        <div className="flex min-w-0 items-center gap-2">
          <GitPullRequestIcon className="size-4 shrink-0 text-muted-foreground" />
          <div className="flex min-w-0 items-center gap-2">
            <span className="truncate text-sm font-medium">Workspace</span>
            <WorktreeStatusBadge status={worktree.status} />
          </div>
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
        ) : selectedTab === "markdown" ? (
          <MarkdownWorkspaceTab
            worktree={worktree}
            onSendAnnotationPrompt={onSendAnnotationPrompt}
          />
        ) : (
          <SpeckitWorkspaceTab worktree={worktree} onSendAnnotationPrompt={onSendAnnotationPrompt} />
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
  const [selectedCommitHash, setSelectedCommitHash] = useState<string | null>(null);
  const [selectedDiffPath, setSelectedDiffPath] = useState<string | null>(null);
  const [staleCommitSelection, setStaleCommitSelection] = useState<StaleSelection | null>(null);
  const [viewMode, setViewMode] = useState<"commit" | "worktree">("commit");
  const [worktreeFilePath, setWorktreeFilePath] = useState<string | null>(null);
  const gitQueryClient = useQueryClient();
  // 선택된 view의 query만 실행한다(specs/007 research R6). 반대 view는 전환
  // 시점에 로드되고, 이미 캐시가 있으면 즉시 표시된다. 후속 페이지는 마지막
  // commit hash를 cursor로 넘겨 이력 재작성을 감지한다(R8).
  const historyQuery = useInfiniteQuery({
    enabled: historyView === "list",
    queryKey: worktreeGitQueryKeys.history(worktree.path),
    queryFn: ({ pageParam }) =>
      listWorktreeGitHistory(worktree.path, {
        maxCount: 100,
        offset: pageParam.offset,
        cursor: pageParam.cursor,
      }),
    initialPageParam: initialGitPageParam,
    getNextPageParam: getNextGitPageParam,
    ...autoRefreshQueryOptions,
  });
  const graphQuery = useInfiniteQuery({
    enabled: historyView === "graph",
    queryKey: worktreeGitQueryKeys.graph(worktree.path),
    queryFn: ({ pageParam }) =>
      getWorktreeGitGraph(worktree.path, {
        maxCount: 300,
        offset: pageParam.offset,
        cursor: pageParam.cursor,
      }),
    initialPageParam: initialGitPageParam,
    getNextPageParam: getNextGitPageParam,
    ...autoRefreshQueryOptions,
  });

  // cursor가 무효(rebase 등)로 판정되면 누적 페이지를 버리고 처음부터 다시
  // 로드한다(contracts §3, data-model cursorInvalidated).
  const historyCursorInvalidated = historyQuery.data?.pages.some(
    (page) => page.page.cursorInvalidated,
  );
  const graphCursorInvalidated = graphQuery.data?.pages.some(
    (page) => page.page.cursorInvalidated,
  );

  useEffect(() => {
    const invalidatedKeys = [
      [historyCursorInvalidated, worktreeGitQueryKeys.history(worktree.path)] as const,
      [graphCursorInvalidated, worktreeGitQueryKeys.graph(worktree.path)] as const,
    ];
    for (const [invalidated, queryKey] of invalidatedKeys) {
      if (invalidated) {
        void gitQueryClient.resetQueries({ queryKey });
      }
    }
  }, [gitQueryClient, graphCursorInvalidated, historyCursorInvalidated, worktree.path]);
  const statusQuery = useQuery({
    queryKey: projectQueryKeys.worktreeChanges(worktree.path),
    queryFn: () => getWorktreeChanges(worktree.path),
    ...autoRefreshQueryOptions,
  });
  const commitDetailQuery = useQuery({
    enabled: selectedCommitHash !== null,
    queryKey: selectedCommitHash
      ? worktreeGitQueryKeys.commitDetail(worktree.path, selectedCommitHash)
      : worktreeGitQueryKeys.commitDetail(worktree.path, ""),
    queryFn: () => getWorktreeCommitDetail(worktree.path, selectedCommitHash ?? ""),
    ...autoRefreshQueryOptions,
  });
  const fileDiffQuery = useQuery({
    enabled: selectedCommitHash !== null && selectedDiffPath !== null,
    queryKey:
      selectedCommitHash && selectedDiffPath
        ? worktreeGitQueryKeys.fileDiff(worktree.path, selectedCommitHash, selectedDiffPath)
        : worktreeGitQueryKeys.fileDiff(worktree.path, "", ""),
    queryFn: () =>
      getWorktreeCommitFileDiff(
        worktree.path,
        selectedCommitHash ?? "",
        selectedDiffPath ?? "",
      ),
    ...autoRefreshQueryOptions,
  });
  const worktreeDiffQuery = useQuery({
    enabled: viewMode === "worktree" && worktreeFilePath !== null,
    queryKey: worktreeFilePath
      ? projectQueryKeys.worktreeFileDiff(worktree.path, worktreeFilePath)
      : projectQueryKeys.worktreeFileDiff(worktree.path, ""),
    queryFn: () => getWorktreeFileDiff(worktree.path, worktreeFilePath ?? ""),
  });
  const historyData = useMemo(
    () => combineGitCommitHistoryPages(historyQuery.data?.pages ?? []),
    [historyQuery.data?.pages],
  );
  const graphData = useMemo(
    () => combineGitCommitGraphPages(graphQuery.data?.pages ?? []),
    [graphQuery.data?.pages],
  );
  const graphRows = useMemo(() => computeGitGraphRows(graphData?.commits ?? []), [graphData]);
  const maxGraphLane = useMemo(() => getMaxGraphLane(graphRows), [graphRows]);
  const graphRefs = useMemo(() => refsByTarget(graphData?.refs ?? []), [graphData?.refs]);
  const graphFirstRowMeasuredRef = useRef(false);

  useEffect(() => {
    graphFirstRowMeasuredRef.current = false;
  }, [worktree.path]);

  useEffect(() => {
    if (!graphFirstRowMeasuredRef.current && (graphData?.commits.length ?? 0) > 0) {
      graphFirstRowMeasuredRef.current = true;
      measureSessionMilestone("session:graph-first-row");
    }
  }, [graphData, worktree.path]);

  function selectCommit(commitHash: string) {
    setViewMode("commit");
    setSelectedCommitHash(commitHash);
    setSelectedDiffPath(null);
    setStaleCommitSelection(null);
  }

  useEffect(() => {
    if (!selectedCommitHash || (!historyData && !graphData)) {
      setStaleCommitSelection(null);
      return;
    }

    const availableCommitHashes = new Set<string>();
    for (const commit of historyData?.commits ?? []) {
      availableCommitHashes.add(commit.hash);
    }
    for (const commit of graphData?.commits ?? []) {
      availableCommitHashes.add(commit.hash);
    }

    const staleSelection = findStaleCommitSelection({
      selectedCommitHash,
      availableCommitHashes,
      reason: "history-rewritten",
    });
    setStaleCommitSelection(staleSelection);
    if (staleSelection) {
      setSelectedDiffPath(null);
    }
  }, [graphData, historyData, selectedCommitHash]);

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full min-h-0">
      <ResizablePanel id="git-workspace-nav" defaultSize="42%" minSize="280px">
        <div className="flex h-full min-h-0 flex-col border-r">
          <section className="shrink-0 border-b p-4">
            <div className="flex items-center justify-between gap-2">
              <div className="flex min-w-0 items-center gap-1.5">
                {statusQuery.isFetching && !statusQuery.isLoading ? (
                  <Badge variant="secondary">Refreshing</Badge>
                ) : null}
                {statusQuery.isError ? <Badge variant="destructive">Error</Badge> : null}
                <Badge variant="outline" className="font-mono">
                  {worktree.branch || "detached"}
                </Badge>
              </div>
              <Button
                type="button"
                size="icon-sm"
                variant="ghost"
                aria-label="Git workspace 새로고침"
                disabled={statusQuery.isFetching || historyQuery.isFetching || graphQuery.isFetching}
                onClick={() => {
                  void statusQuery.refetch();
                  // 비활성(enabled=false) view의 refetch는 no-op이므로 선택된 view만 갱신된다.
                  if (historyView === "list") {
                    void historyQuery.refetch();
                  } else {
                    void graphQuery.refetch();
                  }
                  if (selectedCommitHash) {
                    void commitDetailQuery.refetch();
                  }
                  if (selectedCommitHash && selectedDiffPath) {
                    void fileDiffQuery.refetch();
                  }
                }}
              >
                <RefreshCwIcon
                  className={cn(
                    (statusQuery.isFetching || historyQuery.isFetching || graphQuery.isFetching) &&
                      "animate-spin",
                  )}
                />
              </Button>
            </div>
            <div className="mt-3 flex flex-wrap gap-1.5">
              {statusQuery.isLoading ? (
                <Badge variant="secondary">Loading status</Badge>
              ) : statusQuery.isError ? (
                <Badge variant="destructive">Status error</Badge>
              ) : statusQuery.data ? (
                <>
                  <Badge variant="outline">staged {statusQuery.data.stagedCount}</Badge>
                  <Badge variant="outline">unstaged {statusQuery.data.unstagedCount}</Badge>
                  <Badge variant="outline">untracked {statusQuery.data.untrackedCount}</Badge>
                  <Badge variant={statusQuery.data.conflictedCount > 0 ? "destructive" : "outline"}>
                    conflicted {statusQuery.data.conflictedCount}
                  </Badge>
                </>
              ) : null}
            </div>
          </section>

          <section className="flex min-h-0 flex-1 flex-col">
            <header className="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-1">
              <div className="flex min-w-0 items-center gap-2">
                <GitCommitIcon className="size-4 shrink-0 text-muted-foreground" />
                <h2 className="truncate text-sm font-medium">Commit log</h2>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                {(historyQuery.isFetching || graphQuery.isFetching) &&
                !historyQuery.isLoading &&
                !graphQuery.isLoading ? (
                  <Badge variant="secondary">Refreshing</Badge>
                ) : null}
                {staleCommitSelection ? <Badge variant="destructive">Stale commit</Badge> : null}
                <div className="flex rounded-md border p-0.5">
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
              </div>
            </header>
            <div className="min-h-0 flex-1 overflow-auto p-4">
              {historyView === "graph" ? (
                graphQuery.isLoading ? (
                  <InlineState icon={Loader2Icon} title="Git graph를 불러오는 중입니다." spinning />
                ) : graphQuery.isError ? (
                  <InlineState
                    icon={AlertCircleIcon}
                    title="Git graph를 불러오지 못했습니다."
                    description={String(graphQuery.error)}
                    variant="destructive"
                  />
                ) : graphData && graphData.commits.length > 0 ? (
                  <HistoryGraphView
                    graph={graphData}
                    graphRefs={graphRefs}
                    graphRows={graphRows}
                    maxGraphLane={maxGraphLane}
                    selectedCommitHash={selectedCommitHash ?? undefined}
                    onSelectCommit={selectCommit}
                    hasNextPage={graphQuery.hasNextPage}
                    isFetchingNextPage={graphQuery.isFetchingNextPage}
                    onLoadMore={() => void graphQuery.fetchNextPage()}
                  />
                ) : (
                  <EmptyPanel title="Commit 없음" description="표시할 Git commit이 없습니다." />
                )
              ) : historyQuery.isLoading ? (
                <InlineState icon={Loader2Icon} title="Commit list를 불러오는 중입니다." spinning />
              ) : historyQuery.isError ? (
                <InlineState
                  icon={AlertCircleIcon}
                  title="Commit list를 불러오지 못했습니다."
                  description={String(historyQuery.error)}
                  variant="destructive"
                />
              ) : historyData && historyData.commits.length > 0 ? (
                <CommitListView
                  commits={historyData.commits}
                  page={historyData.page}
                  selectedCommitHash={selectedCommitHash}
                  onSelectCommit={selectCommit}
                  hasNextPage={historyQuery.hasNextPage}
                  isFetchingNextPage={historyQuery.isFetchingNextPage}
                  onLoadMore={() => void historyQuery.fetchNextPage()}
                />
              ) : (
                <EmptyPanel title="Commit 없음" description="표시할 Git commit이 없습니다." />
              )}
            </div>
          </section>
        </div>
      </ResizablePanel>

      <ResizableHandle
        aria-label="Git workspace detail 영역 크기 조정"
        className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      />

      <ResizablePanel id="git-workspace-detail" minSize="320px">
        <div className="flex h-full min-h-0 flex-col">
          <header className="shrink-0 border-b">
            <div className="flex items-center justify-between gap-2 px-4 py-1">
              <div className="flex min-w-0 items-center gap-2">
                {viewMode === "commit" ? (
                  <GitCommitIcon className="size-4 shrink-0 text-muted-foreground" />
                ) : (
                  <FileDiffIcon className="size-4 shrink-0 text-muted-foreground" />
                )}
                <div className="min-w-0">
                  <h2 className="truncate text-sm font-medium">
                    {viewMode === "commit" ? "Commit detail" : "Working tree"}
                  </h2>
                </div>
              </div>
              <div className="flex shrink-0 rounded-md border p-0.5">
                <Button
                  type="button"
                  size="sm"
                  variant={viewMode === "commit" ? "secondary" : "ghost"}
                  onClick={() => setViewMode("commit")}
                >
                  Commit
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant={viewMode === "worktree" ? "secondary" : "ghost"}
                  onClick={() => setViewMode("worktree")}
                >
                  Working tree
                  {(statusQuery.data?.files.length ?? 0) > 0
                    ? ` (${statusQuery.data?.files.length})`
                    : ""}
                </Button>
              </div>
            </div>
          </header>
          <div className="min-h-0 flex-1 overflow-auto p-4">
            {viewMode === "worktree" ? (
              statusQuery.isLoading ? (
                <InlineState icon={Loader2Icon} title="변경사항을 불러오는 중입니다." spinning />
              ) : statusQuery.isError ? (
                <InlineState
                  icon={AlertCircleIcon}
                  title="변경사항을 불러오지 못했습니다."
                  description={String(statusQuery.error)}
                  variant="destructive"
                />
              ) : (
                <WorktreeChangesView
                  changes={statusQuery.data}
                  selectedFilePath={worktreeFilePath ?? undefined}
                  onSelectFile={setWorktreeFilePath}
                  diff={worktreeDiffQuery.data}
                  diffLoading={worktreeDiffQuery.isLoading}
                  diffError={
                    worktreeDiffQuery.isError ? String(worktreeDiffQuery.error) : undefined
                  }
                  diffClassName="max-h-[44svh]"
                />
              )
            ) : selectedCommitHash === null ? (
              <EmptyPanel
                title="선택된 commit 없음"
                description="왼쪽 graph 또는 commit list에서 commit을 선택하면 상세 정보를 표시합니다."
              />
            ) : staleCommitSelection ? (
              <EmptyPanel
                title="선택한 commit이 현재 history에 없습니다"
                description={`${staleCommitSelection.id.slice(0, 8)} commit이 branch 변경, rebase, reset으로 사라졌을 수 있습니다.`}
              />
            ) : commitDetailQuery.isLoading ? (
              <InlineState icon={Loader2Icon} title="Commit detail을 불러오는 중입니다." spinning />
            ) : commitDetailQuery.isError ? (
              <InlineState
                icon={AlertCircleIcon}
                title="Commit detail을 불러오지 못했습니다."
                description={String(commitDetailQuery.error)}
                variant="destructive"
              />
            ) : commitDetailQuery.data ? (
              <CommitDetailView
                commit={commitDetailQuery.data}
                files={commitDetailQuery.data.files}
                selectedFilePath={selectedDiffPath ?? undefined}
                onSelectFile={setSelectedDiffPath}
                diff={fileDiffQuery.data}
                diffLoading={fileDiffQuery.isLoading}
                diffError={fileDiffQuery.isError ? String(fileDiffQuery.error) : undefined}
                diffClassName="max-h-[44svh]"
              />
            ) : null}
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function CommitListView({
  commits,
  page,
  selectedCommitHash,
  onSelectCommit,
  hasNextPage,
  isFetchingNextPage,
  onLoadMore,
}: {
  commits: GitCommitSummary[];
  page: GitCommitGraph["page"];
  selectedCommitHash: string | null;
  onSelectCommit: (commitHash: string) => void;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onLoadMore: () => void;
}) {
  // 로드된 commit 전체를 그리지 않고 viewport 근처 row만 렌더한다(specs/007 R11).
  const { containerRef, startIndex, endExclusive, totalHeight } = useVirtualRows({
    rowCount: commits.length,
    rowHeight: COMMIT_LIST_ROW_HEIGHT,
  });
  const visibleCommits = commits.slice(startIndex, endExclusive);

  return (
    <div className="overflow-hidden rounded-md border text-sm">
      <div className="relative" ref={containerRef} style={{ height: totalHeight }}>
        {visibleCommits.map((commit, index) => (
          <button
            key={commit.hash}
            type="button"
            className="absolute inset-x-0 grid w-full grid-cols-[5rem_minmax(0,1fr)] items-center gap-2 overflow-hidden border-b px-3 text-left hover:bg-muted/50 data-[selected=true]:bg-muted"
            style={{
              height: COMMIT_LIST_ROW_HEIGHT,
              top: (startIndex + index) * COMMIT_LIST_ROW_HEIGHT,
            }}
            data-selected={commit.hash === selectedCommitHash}
            onClick={() => onSelectCommit(commit.hash)}
          >
            <span className="font-mono text-xs text-muted-foreground">{commit.hash.slice(0, 8)}</span>
            <span className="min-w-0">
              <span className="block truncate">{commit.message}</span>
              <span className="block truncate text-xs text-muted-foreground">
                {commit.author} · {formatDate(commit.date)}
              </span>
            </span>
          </button>
        ))}
      </div>
      <div className="border-t px-3 py-2 text-xs text-muted-foreground">
        <InfiniteLoadSentinel
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          onLoadMore={onLoadMore}
        />
        {commits.length} / {page.totalCount ?? commits.length} commits loaded
        {isFetchingNextPage ? " · loading older commits" : ""}
      </div>
    </div>
  );
}

function FileWorkspaceTab({ worktree }: { worktree: GitWorktree }) {
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set());
  // 한 번이라도 펼친 디렉터리의 목록 query는 유지한다. 접었다 다시 펼칠 때
  // 캐시로 즉시 표시되고, stale 파일 판정에도 계속 쓰인다(specs/007 R10).
  const [loadedDirs, setLoadedDirs] = useState<string[]>([]);
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  // 디렉터리 단위 lazy loading: 루트 직계만 먼저 읽고, 폴더 펼침 시 하위를 읽는다.
  const filesQuery = useQuery({
    queryKey: worktreeFileQueryKeys.listScope(worktree.path, { depth: 1 }),
    queryFn: () => listWorktreeFiles(worktree.path, { depth: 1 }),
    ...autoRefreshQueryOptions,
  });
  const dirQueries = useQueries({
    queries: loadedDirs.map((dir) => ({
      queryKey: worktreeFileQueryKeys.listScope(worktree.path, { dir, depth: 1 }),
      queryFn: () => listWorktreeFiles(worktree.path, { dir, depth: 1 }),
      ...autoRefreshQueryOptions,
    })),
  });
  const previewQuery = useQuery({
    enabled: selectedFilePath !== null,
    queryKey: selectedFilePath
      ? worktreeFileQueryKeys.textFile(worktree.path, selectedFilePath)
      : worktreeFileQueryKeys.textFile(worktree.path, ""),
    queryFn: () => readWorktreeTextFile(worktree.path, selectedFilePath ?? ""),
    ...autoRefreshQueryOptions,
  });
  const loadedEntries = mergeWorktreeFileEntries([
    filesQuery.data,
    ...dirQueries.map((dirQuery) => dirQuery.data),
  ]);
  const rows = useMemo(
    () => buildFileTreeRows(loadedEntries, expandedFolders),
    [loadedEntries, expandedFolders],
  );
  const selectedFile = loadedEntries.find(
    (entry) => !entry.isDir && entry.relativePath === selectedFilePath,
  );
  const staleFileSelection = useMemo(() => {
    if (!filesQuery.data || selectedFilePath === null) {
      return null;
    }
    // lazy loading에서는 로드된 디렉터리만 판단 근거가 된다. 부모 디렉터리가
    // 로드되지 않았다면 파일 존재 여부를 알 수 없으므로 stale로 보지 않는다.
    if (!isParentDirectoryLoaded(selectedFilePath, loadedDirs)) {
      return null;
    }
    return findStaleFileSelection({
      selectedPath: selectedFilePath,
      availablePaths: loadedEntries
        .filter((entry) => !entry.isDir)
        .map((entry) => entry.relativePath),
    });
  }, [filesQuery.data, loadedDirs, loadedEntries, selectedFilePath]);

  function selectFile(path: string) {
    setSelectedFilePath(path);
  }

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
    setLoadedDirs((current) => (current.includes(path) ? current : [...current, path]));
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
                  {loadedEntries.length} visible items
                </p>
                <div className="mt-1 flex items-center gap-1.5">
                  {filesQuery.isFetching && !filesQuery.isLoading ? (
                    <Badge variant="secondary">Refreshing</Badge>
                  ) : null}
                  {staleFileSelection ? <Badge variant="destructive">Stale file</Badge> : null}
                  {filesQuery.isError ? <Badge variant="destructive">Error</Badge> : null}
                </div>
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
                    onSelectFile={selectFile}
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
            ) : staleFileSelection ? (
              <EmptyPanel
                title="선택한 파일이 현재 목록에 없습니다"
                description={`${staleFileSelection.id} 파일이 삭제되었거나 이동되었을 수 있습니다.`}
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

function SpeckitWorkspaceTab({
  worktree,
  onSendAnnotationPrompt,
}: {
  worktree: GitWorktree;
  onSendAnnotationPrompt?: (prompt: string) => void;
}) {
  const previewPaneRef = useRef<HTMLDivElement | null>(null);
  const previewContentRef = useRef<HTMLDivElement | null>(null);
  const [selectedDocumentPath, setSelectedDocumentPath] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const filesQuery = useQuery({
    queryKey: worktreeFileQueryKeys.speckit(worktree.path),
    queryFn: () => listSpeckitMarkdownFiles(worktree.path),
    ...autoRefreshQueryOptions,
  });
  const featuresWithoutProgress = useMemo(
    () => buildSpeckitFeatures(filesQuery.data ?? []),
    [filesQuery.data],
  );
  const taskDocumentPaths = useMemo(
    () => getTaskDocumentPaths(featuresWithoutProgress),
    [featuresWithoutProgress],
  );
  const taskQueries = useQueries({
    queries: taskDocumentPaths.map((path) => ({
      queryKey: worktreeFileQueryKeys.textFile(worktree.path, path),
      queryFn: () => readWorktreeTextFile(worktree.path, path),
      ...autoRefreshQueryOptions,
    })),
  });
  const taskContentsByPath = useMemo(() => {
    const contents: Record<string, string | undefined> = {};
    taskDocumentPaths.forEach((path, index) => {
      contents[path] = taskQueries[index]?.data?.content;
    });
    return contents;
  }, [taskDocumentPaths, taskQueries]);
  const features = useMemo(
    () => buildSpeckitFeatures(filesQuery.data ?? [], taskContentsByPath),
    [filesQuery.data, taskContentsByPath],
  );
  const previewQuery = useQuery({
    enabled: selectedDocumentPath !== null,
    queryKey: selectedDocumentPath
      ? worktreeFileQueryKeys.textFile(worktree.path, selectedDocumentPath)
      : worktreeFileQueryKeys.textFile(worktree.path, ""),
    queryFn: () => readWorktreeTextFile(worktree.path, selectedDocumentPath ?? ""),
    ...autoRefreshQueryOptions,
  });
  const blocks = useMemo(
    () => parseMarkdownToBlocks(previewQuery.data?.content ?? ""),
    [previewQuery.data?.content],
  );
  const tocEntries = useMemo(() => extractTocEntries(blocks), [blocks]);
  const annotationWorkspace = useMarkdownAnnotationWorkspace({
    blocks,
    contentRef: previewContentRef,
    documentPath: selectedDocumentPath,
    previewRef: previewPaneRef,
  });
  const selectedDocument = features
    .flatMap((feature) => feature.documents)
    .find((document) => document.relativePath === selectedDocumentPath);
  const staleDocumentSelection = useMemo(() => {
    if (!filesQuery.data || selectedDocumentPath === null) {
      return null;
    }
    return findStaleFileSelection({
      selectedPath: selectedDocumentPath,
      availablePaths: features
        .flatMap((feature) => feature.documents)
        .map((document) => document.relativePath),
    });
  }, [features, filesQuery.data, selectedDocumentPath]);

  function refreshSpeckit() {
    void filesQuery.refetch();
    for (const taskQuery of taskQueries) {
      void taskQuery.refetch();
    }
    if (selectedDocumentPath) {
      void queryClient.invalidateQueries({
        queryKey: worktreeFileQueryKeys.textFile(worktree.path, selectedDocumentPath),
      });
    }
  }

  return (
    <ResizablePanelGroup orientation="horizontal" className="h-full min-h-0">
      <ResizablePanel id="speckit-workspace-list" defaultSize="38%" minSize="280px">
        <SpeckitFilesPanel
          errorMessage={filesQuery.isError ? String(filesQuery.error) : undefined}
          features={features}
          loading={filesQuery.isLoading}
          refreshing={filesQuery.isFetching || taskQueries.some((query) => query.isFetching)}
          selectedDocumentPath={selectedDocumentPath}
          staleDocumentPath={staleDocumentSelection?.id ?? null}
          onRefresh={refreshSpeckit}
          onSelectDocument={setSelectedDocumentPath}
        />
      </ResizablePanel>

      <ResizableHandle
        aria-label="Speckit preview 영역 크기 조정"
        className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        <div className="relative z-10 h-12 w-1 rounded-full bg-border transition-colors" />
      </ResizableHandle>

      <ResizablePanel id="speckit-workspace-preview" minSize="360px">
        <div className="flex h-full min-h-0 flex-col">
          <header className="shrink-0 border-b px-4 py-3">
            <div className="flex min-w-0 items-center gap-2">
              <FileTextIcon className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <h2 className="truncate text-sm font-medium">Speckit preview</h2>
                <p className="truncate font-mono text-xs text-muted-foreground">
                  {selectedDocument?.relativePath ?? selectedDocumentPath ?? "No Speckit document selected"}
                </p>
              </div>
            </div>
          </header>
          <div
            ref={previewPaneRef}
            className="min-h-0 flex-1 overflow-auto p-4"
            onMouseUp={() => window.setTimeout(annotationWorkspace.captureSelection, 10)}
          >
            {selectedDocumentPath === null ? (
              <EmptyPanel
                title="Speckit 문서를 선택하세요"
                description="왼쪽 Speckit 목록에서 spec, plan, tasks 문서를 선택하면 preview를 표시합니다."
              />
            ) : staleDocumentSelection ? (
              <EmptyPanel
                title="선택한 Speckit 문서가 현재 목록에 없습니다"
                description={`${staleDocumentSelection.id} 문서가 삭제되었거나 이동되었을 수 있습니다.`}
              />
            ) : previewQuery.isLoading ? (
              <InlineState icon={Loader2Icon} title="Speckit 문서를 읽는 중입니다." spinning />
            ) : previewQuery.isError ? (
              <InlineState
                icon={AlertCircleIcon}
                title="Speckit preview를 표시할 수 없습니다."
                description={String(previewQuery.error)}
                variant="destructive"
              />
            ) : previewQuery.data ? (
              <MarkdownAnnotationWorkspace
                blocks={blocks}
                contentRef={previewContentRef}
                model={annotationWorkspace}
                onSendAnnotationPrompt={onSendAnnotationPrompt}
                previewRef={previewPaneRef}
                tocEntries={tocEntries}
              />
            ) : null}
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function MarkdownWorkspaceTab({
  worktree,
  onSendAnnotationPrompt,
}: {
  worktree: GitWorktree;
  onSendAnnotationPrompt?: (prompt: string) => void;
}) {
  const previewPaneRef = useRef<HTMLDivElement | null>(null);
  const markdownColumnRef = useRef<HTMLDivElement | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set());
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [annotationsByFile, setAnnotationsByFile] = useState<Record<string, AnnotationDraft[]>>({});
  const [draftTarget, setDraftTarget] = useState<AnnotationDraftTarget | null>(null);
  const [draftType, setDraftType] = useState<AnnotationType>("note");
  const [draftComment, setDraftComment] = useState("");
  const [selectionAnchors, setSelectionAnchors] = useState<AnnotationAnchor[]>([]);
  const [selectionHighlightRects, setSelectionHighlightRects] = useState<SelectionRect[]>([]);
  const [selectionToolbarPosition, setSelectionToolbarPosition] = useState<{
    left: number;
    top: number;
  } | null>(null);
  const [editingAnnotationId, setEditingAnnotationId] = useState<string | null>(null);
  // markdown 필터는 백엔드(scope)에서 수행되어 전체 파일 목록을 받지 않는다.
  // 프론트 필터는 응답 검증 겸 안전망으로 유지한다(specs/007 R10).
  const filesQuery = useQuery({
    queryKey: worktreeFileQueryKeys.listScope(worktree.path, { kind: "markdown" }),
    queryFn: () => listWorktreeFiles(worktree.path, { kind: "markdown" }),
    ...autoRefreshQueryOptions,
  });
  const markdownEntries = useMemo(
    () => filterMarkdownTreeEntries(filesQuery.data ?? []),
    [filesQuery.data],
  );
  const rows = useMemo(
    () => buildFileTreeRows(markdownEntries, expandedFolders),
    [markdownEntries, expandedFolders],
  );
  const previewQuery = useQuery({
    enabled: selectedFilePath !== null,
    queryKey: selectedFilePath
      ? worktreeFileQueryKeys.textFile(worktree.path, selectedFilePath)
      : worktreeFileQueryKeys.textFile(worktree.path, ""),
    queryFn: () => readWorktreeTextFile(worktree.path, selectedFilePath ?? ""),
    ...autoRefreshQueryOptions,
  });
  const blocks = useMemo(
    () => parseMarkdownToBlocks(previewQuery.data?.content ?? ""),
    [previewQuery.data?.content],
  );
  const tocEntries = useMemo(() => extractTocEntries(blocks), [blocks]);
  const annotations = selectedFilePath ? (annotationsByFile[selectedFilePath] ?? []) : [];
  const annotationPrompt = useMemo(
    () =>
      selectedFilePath ? formatAnnotationsForAgent(selectedFilePath, annotations, blocks) : "",
    [selectedFilePath, annotations, blocks],
  );
  const viewerMaps = useMemo(
    () => buildViewerAnnotationMaps(annotations, blocks),
    [annotations, blocks],
  );

  function resetDraftState() {
    setDraftTarget(null);
    setDraftComment("");
    setDraftType("note");
    setEditingAnnotationId(null);
  }

  function selectMarkdownFile(path: string) {
    setSelectedFilePath(path);
    resetDraftState();
    resetSelectionState();
  }

  const staleFileSelection = useMemo(() => {
    if (!filesQuery.data || selectedFilePath === null) {
      return null;
    }
    return findStaleFileSelection({
      selectedPath: selectedFilePath,
      availablePaths: markdownEntries
        .filter((entry) => !entry.isDir)
        .map((entry) => entry.relativePath),
    });
  }, [filesQuery.data, markdownEntries, selectedFilePath]);

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

  function saveAnnotation() {
    if (!selectedFilePath || !draftTarget || (draftType !== "delete" && !draftComment.trim())) {
      return;
    }

    const comment = draftComment.trim();

    // 편집 모드: 기존 annotation(그룹 포함)의 comment/type만 갱신한다.
    if (editingAnnotationId) {
      setAnnotationsByFile((current) => {
        const fileAnnotations = current[selectedFilePath] ?? [];
        const editing = fileAnnotations.find((annotation) => annotation.id === editingAnnotationId);
        const editingGroupId = editing?.groupId;

        return {
          ...current,
          [selectedFilePath]: fileAnnotations.map((annotation) =>
            annotation.id === editingAnnotationId ||
            (editingGroupId !== undefined && annotation.groupId === editingGroupId)
              ? { ...annotation, comment, type: draftType }
              : annotation,
          ),
        };
      });
      resetDraftState();
      resetSelectionState();
      return;
    }

    const createdAt = new Date().toISOString();
    const groupId =
      draftTarget.kind === "selection" && draftTarget.anchors.length > 1
        ? crypto.randomUUID()
        : undefined;
    const nextAnnotations =
      draftTarget.kind === "block"
        ? [
            createAnnotationFromAnchor({
              anchor: fullBlockAnchor(draftTarget.block),
              block: draftTarget.block,
              comment,
              createdAt,
              fileName: selectedFilePath,
              type: draftType,
            }),
          ]
        : draftTarget.anchors.map((anchor) =>
            createAnnotationFromAnchor({
              anchor,
              comment,
              createdAt,
              fileName: selectedFilePath,
              groupId,
              type: draftType,
            }),
          );

    setAnnotationsByFile((current) => ({
      ...current,
      [selectedFilePath]: [...(current[selectedFilePath] ?? []), ...nextAnnotations],
    }));
    resetDraftState();
    resetSelectionState();
  }

  function requestBlockComment(block: MarkdownBlock) {
    setEditingAnnotationId(null);
    setDraftTarget({ kind: "block", block });
    setDraftType("note");
    setDraftComment("");
  }

  // 전체 블록 delete 토글: 이미 있으면 취소, 없으면 즉시 추가(부록 B-2.7 결함 해소).
  function toggleBlockDelete(block: MarkdownBlock) {
    if (!selectedFilePath) {
      return;
    }

    const existing = annotations.find(
      (annotation) => annotation.type === "delete" && isFullBlockAnnotation(annotation, block),
    );
    if (existing) {
      removeAnnotation(existing.id);
      return;
    }

    const annotation = createAnnotationFromAnchor({
      anchor: fullBlockAnchor(block),
      block,
      comment: "",
      createdAt: new Date().toISOString(),
      fileName: selectedFilePath,
      type: "delete",
    });
    setAnnotationsByFile((current) => ({
      ...current,
      [selectedFilePath]: [...(current[selectedFilePath] ?? []), annotation],
    }));
  }

  function editInlineAnnotation(annotationId: string) {
    const annotation = annotations.find((candidate) => candidate.id === annotationId);
    if (!annotation) {
      return;
    }

    setEditingAnnotationId(annotation.id);
    setDraftType(annotation.type);
    setDraftComment(annotation.comment);
    setDraftTarget({
      kind: "selection",
      anchors: [annotation.anchor],
      text: annotation.selectedText,
    });
  }

  function removeAnnotation(annotationId: string) {
    if (!selectedFilePath) {
      return;
    }

    setAnnotationsByFile((current) => {
      const target = (current[selectedFilePath] ?? []).find(
        (annotation) => annotation.id === annotationId,
      );
      const targetGroupId = target?.groupId;

      return {
        ...current,
        [selectedFilePath]: (current[selectedFilePath] ?? []).filter((annotation) =>
          targetGroupId ? annotation.groupId !== targetGroupId : annotation.id !== annotationId,
        ),
      };
    });
  }

  function resetSelectionState() {
    setSelectionAnchors([]);
    setSelectionHighlightRects([]);
    setSelectionToolbarPosition(null);
    window.getSelection()?.removeAllRanges();
  }

  function captureSelection() {
    const anchors = getSelectionAnchors(previewPaneRef.current);
    if (anchors.length === 0) {
      setSelectionAnchors([]);
      setSelectionHighlightRects([]);
      setSelectionToolbarPosition(null);
      return;
    }

    setSelectionAnchors(anchors);

    // 선택 영역을 markdown 컬럼 기준 상대좌표로 캡처한다. mouseup 후 브라우저
    // 선택이 풀려도 이 좌표로 오버레이를 그려 선택 영역을 시각적으로 유지한다(MA와 동일).
    const highlightRects = getSelectionRects(markdownColumnRef.current);
    setSelectionHighlightRects(highlightRects);
    const lastRect = highlightRects[highlightRects.length - 1];
    setSelectionToolbarPosition(
      lastRect ? { left: lastRect.left + lastRect.width + 8, top: lastRect.top } : null,
    );
  }

  function scheduleCaptureSelection() {
    window.setTimeout(captureSelection, 10);
  }

  // 툴바 노트: 선택 영역을 note draft로 열어 comment를 입력받는다.
  function requestSelectionNote() {
    if (selectionAnchors.length === 0) {
      return;
    }
    const text = selectionAnchors.map((anchor) => anchor.selectedText).filter(Boolean).join("\n");
    setEditingAnnotationId(null);
    setDraftTarget({ kind: "selection", anchors: selectionAnchors, text });
    setDraftType("note");
    setDraftComment("");
    setSelectionToolbarPosition(null);
  }

  // 툴바 삭제: 선택 영역에 delete annotation을 즉시 추가한다(모달 없이).
  function deleteSelection() {
    if (!selectedFilePath || selectionAnchors.length === 0) {
      return;
    }
    const createdAt = new Date().toISOString();
    const groupId = selectionAnchors.length > 1 ? crypto.randomUUID() : undefined;
    const nextAnnotations = selectionAnchors.map((anchor) =>
      createAnnotationFromAnchor({
        anchor,
        comment: "",
        createdAt,
        fileName: selectedFilePath,
        groupId,
        type: "delete",
      }),
    );
    setAnnotationsByFile((current) => ({
      ...current,
      [selectedFilePath]: [...(current[selectedFilePath] ?? []), ...nextAnnotations],
    }));
    resetSelectionState();
  }

  return (
    <>
      <ResizablePanelGroup orientation="horizontal" className="h-full min-h-0">
      <ResizablePanel id="markdown-workspace-tree" defaultSize="38%" minSize="260px">
        <div className="flex h-full min-h-0 flex-col border-r">
          <header className="flex shrink-0 items-center justify-between gap-2 border-b px-4 py-3">
            <div className="flex min-w-0 items-center gap-2">
              <FileTextIcon className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <h2 className="truncate text-sm font-medium">Markdown files</h2>
                <p className="truncate text-xs text-muted-foreground">
                  {markdownEntries.filter((entry) => !entry.isDir).length} files
                </p>
                <div className="mt-1 flex items-center gap-1.5">
                  {filesQuery.isFetching && !filesQuery.isLoading ? (
                    <Badge variant="secondary">Refreshing</Badge>
                  ) : null}
                  {staleFileSelection ? <Badge variant="destructive">Stale file</Badge> : null}
                  {filesQuery.isError ? <Badge variant="destructive">Error</Badge> : null}
                </div>
              </div>
            </div>
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="Markdown file tree 새로고침"
              disabled={filesQuery.isFetching}
              onClick={() => void filesQuery.refetch()}
            >
              <RefreshCwIcon className={cn(filesQuery.isFetching && "animate-spin")} />
            </Button>
          </header>
          <div className="min-h-0 flex-1 overflow-auto p-2">
            {filesQuery.isLoading ? (
              <InlineState icon={Loader2Icon} title="Markdown 파일을 불러오는 중입니다." spinning />
            ) : filesQuery.isError ? (
              <InlineState
                icon={AlertCircleIcon}
                title="Markdown 파일을 불러오지 못했습니다."
                description={String(filesQuery.error)}
                variant="destructive"
              />
            ) : rows.length === 0 ? (
              <EmptyPanel
                title="Markdown 파일 없음"
                description=".md, .markdown, .mdx 파일을 찾지 못했습니다."
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
                    onSelectFile={selectMarkdownFile}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </ResizablePanel>

      <ResizableHandle
        aria-label="Markdown preview 영역 크기 조정"
        className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        <div className="relative z-10 h-12 w-1 rounded-full bg-border transition-colors" />
      </ResizableHandle>

      <ResizablePanel id="markdown-workspace-preview" minSize="360px">
        <div className="flex h-full min-h-0 flex-col">
          <header className="shrink-0 border-b px-4 py-3">
            <div className="flex min-w-0 items-center gap-2">
              <FileTextIcon className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0">
                <h2 className="truncate text-sm font-medium">Markdown preview</h2>
                <p className="truncate font-mono text-xs text-muted-foreground">
                  {selectedFilePath ?? "No markdown file selected"}
                </p>
              </div>
            </div>
          </header>
          <div
            ref={previewPaneRef}
            className="min-h-0 flex-1 overflow-auto p-4"
            onMouseUp={scheduleCaptureSelection}
          >
            {selectedFilePath === null ? (
              <EmptyPanel
                title="Markdown 파일을 선택하세요"
                description="왼쪽 markdown tree에서 파일을 선택하면 preview를 표시합니다."
              />
            ) : staleFileSelection ? (
              <EmptyPanel
                title="선택한 Markdown 파일이 현재 목록에 없습니다"
                description={`${staleFileSelection.id} 파일이 삭제되었거나 이동되었을 수 있습니다.`}
              />
            ) : previewQuery.isLoading ? (
              <InlineState icon={Loader2Icon} title="Markdown 파일을 읽는 중입니다." spinning />
            ) : previewQuery.isError ? (
              <InlineState
                icon={AlertCircleIcon}
                title="Markdown preview를 표시할 수 없습니다."
                description={String(previewQuery.error)}
                variant="destructive"
              />
            ) : previewQuery.data ? (
              <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_20rem]">
                <div ref={markdownColumnRef} className="relative min-w-0">
                  <MarkdownViewer
                    blocks={blocks}
                    components={markdownViewerComponents}
                    annotatedBlockIds={viewerMaps.annotatedBlockIds}
                    deletedBlockIds={viewerMaps.deletedBlockIds}
                    inlineAnnotationsByBlock={viewerMaps.inlineAnnotationsByBlock}
                    noteAnnotationsByBlock={viewerMaps.noteAnnotationsByBlock}
                    onCancelInlineAnnotation={removeAnnotation}
                    onEditInlineAnnotation={editInlineAnnotation}
                    onRequestBlockComment={requestBlockComment}
                    onRequestBlockDelete={toggleBlockDelete}
                  />
                  {selectionHighlightRects.map((rect, index) => (
                    <div
                      aria-hidden="true"
                      className="pointer-events-none absolute bg-yellow-200/50"
                      key={`${rect.left}-${rect.top}-${index}`}
                      style={{ height: rect.height, left: rect.left, top: rect.top, width: rect.width }}
                    />
                  ))}
                  {selectionToolbarPosition ? (
                    <div
                      className="absolute z-10 flex items-center gap-1 rounded-lg border bg-popover p-1 shadow-sm"
                      style={{ left: selectionToolbarPosition.left, top: selectionToolbarPosition.top }}
                      onMouseDown={(event) => {
                        event.preventDefault();
                        event.stopPropagation();
                      }}
                      onMouseUp={(event) => event.stopPropagation()}
                      onClick={(event) => event.stopPropagation()}
                    >
                      <Button
                        type="button"
                        size="icon-sm"
                        variant="ghost"
                        aria-label="선택 영역 삭제"
                        onClick={deleteSelection}
                      >
                        <Trash2Icon />
                      </Button>
                      <Button
                        type="button"
                        size="icon-sm"
                        variant="ghost"
                        aria-label="선택 영역에 노트 추가"
                        onClick={requestSelectionNote}
                      >
                        <StickyNoteIcon />
                      </Button>
                    </div>
                  ) : null}
                </div>
                <aside className="flex flex-col gap-3">
                  <div className="rounded-md border p-3">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="text-sm font-medium">Annotations</h3>
                      <Badge variant="outline">{annotations.length}</Badge>
	                    </div>
	                    {annotations.length === 0 ? (
                      <p className="mt-3 text-sm text-muted-foreground">No annotations.</p>
                    ) : (
                      <div className="mt-3 grid gap-2">
                        {annotations.map((annotation) => (
                          <div key={annotation.id} className="rounded-md border p-2">
	                            <div className="flex items-start justify-between gap-2">
	                              <div className="min-w-0">
	                                <Badge variant={annotation.type === "delete" ? "destructive" : "secondary"}>
	                                  {annotation.type}
	                                </Badge>
	                                <p className="mt-1 text-xs text-muted-foreground">
	                                  Lines {annotation.anchor.startLine}-{annotation.anchor.endLine}
	                                </p>
	                              </div>
                              <div className="flex shrink-0 items-center gap-1">
                                <Button
                                  type="button"
                                  size="icon-xs"
                                  variant="ghost"
                                  aria-label="Annotation 편집"
                                  onClick={() => editInlineAnnotation(annotation.id)}
                                >
                                  <PencilLineIcon />
                                </Button>
                                <Button
                                  type="button"
                                  size="icon-xs"
                                  variant="ghost"
                                  aria-label="Annotation 삭제"
                                  onClick={() => removeAnnotation(annotation.id)}
                                >
                                  <Trash2Icon />
                                </Button>
                              </div>
	                            </div>
	                            <p className="mt-2 line-clamp-3 whitespace-pre-wrap break-words text-xs text-muted-foreground">
	                              {annotation.selectedText}
	                            </p>
	                            {annotation.comment ? (
	                              <p className="mt-2 break-words text-sm">{annotation.comment}</p>
	                            ) : null}
	                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                  {annotations.length > 0 ? (
                    <div className="rounded-md border">
                      <div className="flex items-center justify-between gap-2 border-b px-3 py-2">
                        <span className="text-sm font-medium">Agent prompt</span>
                        <Button
                          type="button"
                          size="sm"
                          disabled={!onSendAnnotationPrompt}
                          onClick={() => onSendAnnotationPrompt?.(annotationPrompt)}
                        >
                          <SendIcon data-icon="inline-start" />
                          Send
                        </Button>
                      </div>
                      <pre className="max-h-80 overflow-auto p-3 text-xs leading-5">
                        <code>{annotationPrompt}</code>
                      </pre>
                    </div>
                  ) : null}
                  <MarkdownPreviewToc
                    className="sticky bottom-4 mt-auto shadow-sm"
                    entries={tocEntries}
                    key={selectedFilePath}
                    onEntrySelect={(entry) => scrollToBlock(previewPaneRef.current, entry.blockId)}
                  />
                </aside>
              </div>
            ) : null}
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
      <AnnotationInputDialog
        open={draftTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            resetDraftState();
            resetSelectionState();
          }
        }}
        isEditing={editingAnnotationId !== null}
        selectedText={
          draftTarget
            ? draftTarget.kind === "selection"
              ? draftTarget.text
              : draftTarget.block.rawContent
            : ""
        }
        type={draftType}
        onTypeChange={setDraftType}
        comment={draftComment}
        onCommentChange={setDraftComment}
        onSubmit={saveAnnotation}
        components={annotationDialogComponents}
      />
    </>
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

function filterMarkdownTreeEntries(entries: WorktreeFileEntry[]) {
  const markdownFiles = entries.filter(
    (entry) => !entry.isDir && isMarkdownPath(entry.relativePath),
  );
  const folderPaths = new Set<string>();

  for (const file of markdownFiles) {
    const segments = file.relativePath.split("/").filter(Boolean);
    let folderPath = "";

    for (const segment of segments.slice(0, -1)) {
      folderPath = folderPath ? `${folderPath}/${segment}` : segment;
      folderPaths.add(folderPath);
    }
  }

  return entries.filter(
    (entry) =>
      (!entry.isDir && isMarkdownPath(entry.relativePath)) ||
      (entry.isDir && folderPaths.has(entry.relativePath)),
  );
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

function formatDate(value: string) {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}
