import type { ElementType, ReactNode } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
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
  MessageSquareIcon,
  PencilLineIcon,
  RefreshCwIcon,
  SendIcon,
  Trash2Icon,
  XIcon,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  Group as ResizablePanelGroup,
  Panel as ResizablePanel,
  Separator as ResizableHandle,
} from "react-resizable-panels";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Markdown } from "@/components/ui/markdown";
import { Textarea } from "@/components/ui/textarea";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import { getWorktreeChanges } from "@/entities/project/api/git-worktree-repository";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import { worktreeFileQueryKeys } from "@/entities/worktree-file/api/query-keys";
import {
  listWorktreeFiles,
  readWorktreeTextFile,
} from "@/entities/worktree-file/api/worktree-file-repository";
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
  GitGraphLayoutHints,
  GitGraphCommit,
  GitGraphRef,
} from "@/entities/worktree-git/model/types";
import {
  computeGitGraphRows,
  getMaxGraphLane,
  type GitGraphRow,
  type GitGraphSegment,
} from "@/features/worktree-workspace/model/git-graph-layout";
import { formatAnnotationsForAgent } from "@/features/markdown-annotation/model/format-annotations-for-agent";
import { parseMarkdownToBlocks } from "@/features/markdown-annotation/model/parse-markdown-to-blocks";
import type {
  AnnotationDraft,
  AnnotationAnchor,
  AnnotationType,
  MarkdownBlock,
} from "@/features/markdown-annotation/model/types";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type WorktreeWorkspacePanelProps = {
  worktree: GitWorktree;
  onSendAnnotationPrompt?: (prompt: string) => void;
};

type WorkspaceTabId = "git" | "files" | "markdown";
type GitHistoryView = "graph" | "list";

type FileTreeRow = WorktreeFileEntry & {
  depth: number;
  isExpanded: boolean;
};

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

const workspaceTabs: Array<{
  id: WorkspaceTabId;
  label: string;
  icon: typeof GitBranchIcon;
}> = [
  { id: "git", label: "Git", icon: GitBranchIcon },
  { id: "files", label: "Files", icon: FileIcon },
  { id: "markdown", label: "Markdown", icon: FileTextIcon },
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

function isFullBlockAnnotation(annotation: AnnotationDraft, block: MarkdownBlock) {
  return (
    annotation.anchor.startOffset === 0 &&
    annotation.anchor.endOffset === block.content.length
  );
}

function buildInlineAnnotationsByBlock(annotations: AnnotationDraft[], blocks: MarkdownBlock[]) {
  const inlineAnnotations = new Map<string, AnnotationDraft[]>();

  for (const annotation of annotations) {
    const block = blocks.find((candidate) => candidate.id === annotation.anchor.blockId);
    if (
      !block ||
      annotation.anchor.startOffset === undefined ||
      annotation.anchor.endOffset === undefined ||
      isFullBlockAnnotation(annotation, block)
    ) {
      continue;
    }

    const blockAnnotations = inlineAnnotations.get(annotation.anchor.blockId) ?? [];
    blockAnnotations.push(annotation);
    inlineAnnotations.set(annotation.anchor.blockId, blockAnnotations);
  }

  return inlineAnnotations;
}

function formatDraftTargetRange(target: AnnotationDraftTarget) {
  if (target.kind === "block") {
    return `Block lines ${target.block.startLine}-${target.block.endLine}`;
  }

  const first = target.anchors[0];
  const last = target.anchors[target.anchors.length - 1];
  if (!first || !last) {
    return "Selection";
  }

  return `Selection lines ${first.startLine}-${last.endLine}`;
}

function getSelectionAnchors(root: HTMLElement | null): AnnotationAnchor[] {
  const selection = window.getSelection();
  if (!root || !selection || selection.isCollapsed || selection.rangeCount === 0) {
    return [];
  }

  const range = selection.getRangeAt(0);
  if (
    !selection.anchorNode ||
    !selection.focusNode ||
    !root.contains(selection.anchorNode) ||
    !root.contains(selection.focusNode)
  ) {
    return [];
  }

  const anchors: AnnotationAnchor[] = [];

  for (const contentElement of Array.from(root.querySelectorAll<HTMLElement>("[data-block-content]"))) {
      if (!range.intersectsNode(contentElement)) {
        continue;
      }

      const blockElement = contentElement.closest<HTMLElement>("[data-block-id]");
      const blockId = blockElement?.dataset.blockId;
      if (!blockElement || !blockId) {
        continue;
      }

      const text = contentElement.textContent ?? "";
      const startOffset = contentElement.contains(range.startContainer)
        ? getTextOffsetWithin(contentElement, range.startContainer, range.startOffset)
        : 0;
      const endOffset = contentElement.contains(range.endContainer)
        ? getTextOffsetWithin(contentElement, range.endContainer, range.endOffset)
        : text.length;
      const normalizedStart = Math.max(0, Math.min(startOffset, text.length));
      const normalizedEnd = Math.max(normalizedStart, Math.min(endOffset, text.length));
      if (normalizedStart === normalizedEnd) {
        continue;
      }

      anchors.push({
        blockId,
        startLine: Number(blockElement.dataset.startLine ?? 0),
        endLine: Number(blockElement.dataset.endLine ?? 0),
        startOffset: normalizedStart,
        endOffset: normalizedEnd,
        selectedText: text.slice(normalizedStart, normalizedEnd),
      });
    }

  return anchors;
}

function getTextOffsetWithin(root: Node, target: Node, targetOffset: number) {
  const range = document.createRange();
  range.selectNodeContents(root);
  range.setEnd(target, targetOffset);
  return range.toString().length;
}

export function WorktreeWorkspacePanel({
  worktree,
  onSendAnnotationPrompt,
}: WorktreeWorkspacePanelProps) {
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
          <MarkdownWorkspaceTab
            worktree={worktree}
            onSendAnnotationPrompt={onSendAnnotationPrompt}
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
  const [selectedCommitHash, setSelectedCommitHash] = useState<string | null>(null);
  const [selectedDiffPath, setSelectedDiffPath] = useState<string | null>(null);
  const historyQuery = useInfiniteQuery({
    queryKey: worktreeGitQueryKeys.history(worktree.path),
    queryFn: ({ pageParam }) =>
      listWorktreeGitHistory(worktree.path, {
        maxCount: 100,
        offset: pageParam,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) =>
      lastPage.page.hasMore ? lastPage.page.offset + lastPage.commits.length : undefined,
  });
  const graphQuery = useInfiniteQuery({
    queryKey: worktreeGitQueryKeys.graph(worktree.path),
    queryFn: ({ pageParam }) =>
      getWorktreeGitGraph(worktree.path, {
        maxCount: 300,
        offset: pageParam,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) =>
      lastPage.page.hasMore ? lastPage.page.offset + lastPage.commits.length : undefined,
  });
  const statusQuery = useQuery({
    queryKey: projectQueryKeys.worktreeChanges(worktree.path),
    queryFn: () => getWorktreeChanges(worktree.path),
  });
  const commitDetailQuery = useQuery({
    enabled: selectedCommitHash !== null,
    queryKey: selectedCommitHash
      ? worktreeGitQueryKeys.commitDetail(worktree.path, selectedCommitHash)
      : worktreeGitQueryKeys.commitDetail(worktree.path, ""),
    queryFn: () => getWorktreeCommitDetail(worktree.path, selectedCommitHash ?? ""),
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
  });
  const historyData = useMemo(
    () => combineGitHistoryPages(historyQuery.data?.pages ?? []),
    [historyQuery.data?.pages],
  );
  const graphData = useMemo(
    () => combineGitGraphPages(graphQuery.data?.pages ?? []),
    [graphQuery.data?.pages],
  );
  const graphRows = useMemo(() => computeGitGraphRows(graphData?.commits ?? []), [graphData]);
  const maxGraphLane = useMemo(() => getMaxGraphLane(graphRows), [graphRows]);
  const graphRefs = useMemo(() => refsByTarget(graphData?.refs ?? []), [graphData?.refs]);

  function selectCommit(commitHash: string) {
    setSelectedCommitHash(commitHash);
    setSelectedDiffPath(null);
  }

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
                    selectedCommitHash={selectedCommitHash}
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
            {selectedCommitHash === null ? (
              <EmptyPanel
                title="선택된 commit 없음"
                description="왼쪽 graph 또는 commit list에서 commit을 선택하면 상세 정보를 표시합니다."
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
              <div className="grid gap-4">
                <div className="grid gap-1">
                  <p className="font-mono text-xs text-muted-foreground">
                    {commitDetailQuery.data.hash}
                  </p>
                  <h3 className="break-words text-sm font-medium">
                    {commitDetailQuery.data.message}
                  </h3>
                  <p className="text-sm text-muted-foreground">
                    {commitDetailQuery.data.author} · {formatDate(commitDetailQuery.data.date)}
                  </p>
                </div>
                <div className="grid gap-2">
                  <h3 className="text-sm font-medium">Changed files</h3>
                  <div className="overflow-hidden rounded-md border text-sm">
                    {commitDetailQuery.data.files.length === 0 ? (
                      <p className="p-3 text-sm text-muted-foreground">No changed files.</p>
                    ) : (
                      commitDetailQuery.data.files.map((file) => (
                        <button
                          key={`${file.status}:${file.path}`}
                          type="button"
                          className="flex h-8 w-full min-w-0 items-center gap-2 border-b px-2 text-left last:border-b-0 hover:bg-muted data-[selected=true]:bg-muted"
                          data-selected={selectedDiffPath === file.path}
                          onClick={() => setSelectedDiffPath(file.path)}
                        >
                          <Badge variant="outline" className="w-10 justify-center font-mono">
                            {file.status}
                          </Badge>
                          <span className="min-w-0 flex-1 truncate font-mono text-xs">
                            {file.path}
                          </span>
                        </button>
                      ))
                    )}
                  </div>
                </div>
                {selectedDiffPath ? (
                  <div className="grid gap-2">
                    <h3 className="text-sm font-medium">Diff</h3>
                    {fileDiffQuery.isLoading ? (
                      <InlineState icon={Loader2Icon} title="Diff를 불러오는 중입니다." spinning />
                    ) : fileDiffQuery.isError ? (
                      <InlineState
                        icon={AlertCircleIcon}
                        title="Diff를 불러오지 못했습니다."
                        description={String(fileDiffQuery.error)}
                        variant="destructive"
                      />
                    ) : fileDiffQuery.data ? (
                      <pre className="max-h-[44svh] overflow-auto rounded-md border bg-muted/20 p-3 text-xs leading-5">
                        <code>{fileDiffQuery.data.content}</code>
                      </pre>
                    ) : null}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        </div>
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function HistoryGraphView({
  graph,
  graphRefs,
  graphRows,
  maxGraphLane,
  selectedCommitHash,
  onSelectCommit,
  hasNextPage,
  isFetchingNextPage,
  onLoadMore,
}: {
  graph: GitCommitGraph;
  graphRefs: Map<string, GitGraphRef[]>;
  graphRows: Map<string, GitGraphRow>;
  maxGraphLane: number;
  selectedCommitHash: string | null;
  onSelectCommit: (commitHash: string) => void;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onLoadMore: () => void;
}) {
  const rowHeight = graph.layoutHints.rowHeight || 32;

  return (
    <div className="overflow-hidden rounded-md border">
      <div className="grid grid-cols-[auto_minmax(0,1fr)] border-b bg-muted/40 px-2 py-2 text-xs font-medium text-muted-foreground">
        <span>Graph</span>
        <span>Commit</span>
      </div>
      {graph.commits.map((commit) => (
        <HistoryGraphRow
          key={commit.hash}
          commit={commit}
          graphRefs={graphRefs.get(commit.hash) ?? []}
          graphRow={graphRows.get(commit.hash)}
          isSelected={commit.hash === selectedCommitHash}
          maxGraphLane={maxGraphLane}
          rowHeight={rowHeight}
          onSelectCommit={onSelectCommit}
        />
      ))}
      <div className="border-t px-3 py-2 text-xs text-muted-foreground">
        <InfiniteLoadSentinel
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          onLoadMore={onLoadMore}
        />
        {graph.commits.length} / {graph.page.totalCount} commits loaded
        {isFetchingNextPage ? " · loading older commits" : ""}
      </div>
    </div>
  );
}

function HistoryGraphRow({
  commit,
  graphRefs,
  graphRow,
  isSelected,
  maxGraphLane,
  rowHeight,
  onSelectCommit,
}: {
  commit: GitGraphCommit;
  graphRefs: GitGraphRef[];
  graphRow?: GitGraphRow;
  isSelected: boolean;
  maxGraphLane: number;
  rowHeight: number;
  onSelectCommit: (commitHash: string) => void;
}) {
  return (
    <button
      type="button"
      aria-label={`Commit ${commit.shortHash}: ${commit.message}`}
      className="grid w-full grid-cols-[auto_minmax(0,1fr)] items-center border-b px-2 text-left text-sm last:border-b-0 hover:bg-muted/50 data-[selected=true]:bg-muted"
      data-selected={isSelected}
      style={{ minHeight: rowHeight }}
      onClick={() => onSelectCommit(commit.hash)}
    >
      <GraphCell maxLane={maxGraphLane} row={graphRow} rowHeight={rowHeight} />
      <span className="flex min-w-0 items-center gap-2 pr-2">
        <span className="font-mono text-xs text-muted-foreground">{commit.shortHash}</span>
        {graphRefs.map((ref) => (
          <span
            key={`${ref.kind}:${ref.name}`}
            className="max-w-28 truncate rounded-sm border bg-background px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground"
            title={ref.name}
          >
            {ref.kind === "tag" ? "tag:" : ""}
            {ref.name}
          </span>
        ))}
        <span className="min-w-0 truncate">{commit.message}</span>
      </span>
    </button>
  );
}

function GraphCell({
  maxLane,
  row,
  rowHeight,
}: {
  maxLane: number;
  row?: GitGraphRow;
  rowHeight: number;
}) {
  const width = 20 + (maxLane + 1) * 20;
  const nodeX = row ? laneX(row.lane) : 10;
  const centerY = rowHeight / 2;

  return (
    <svg aria-hidden className="block shrink-0" height={rowHeight} width={width}>
      {row?.connections.map((segment, index) => (
        <path
          key={`${segment.type}:${segment.fromLane}:${segment.toLane}:${index}`}
          d={graphSegmentPath(segment, rowHeight)}
          fill="none"
          stroke={segment.color}
          strokeDasharray={segment.type.startsWith("merge") ? "4 3" : undefined}
          strokeWidth="2"
        />
      ))}
      {row ? (
        row.nodeType === "head" ? (
          <>
            <circle cx={nodeX} cy={centerY} fill="none" r="6" stroke="currentColor" strokeWidth="2" />
            <circle cx={nodeX} cy={centerY} fill={row.color} r="4" />
          </>
        ) : row.nodeType === "merge" ? (
          <>
            <circle cx={nodeX} cy={centerY} fill="none" r="5" stroke={row.color} strokeWidth="1.5" />
            <circle cx={nodeX} cy={centerY} fill={row.color} r="3" />
          </>
        ) : (
          <circle cx={nodeX} cy={centerY} fill={row.color} r="4" />
        )
      ) : null}
    </svg>
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
  return (
    <div className="overflow-hidden rounded-md border text-sm">
      {commits.map((commit) => (
        <button
          key={commit.hash}
          type="button"
          className="grid w-full grid-cols-[5rem_minmax(0,1fr)] gap-2 border-b px-3 py-2 text-left last:border-b-0 hover:bg-muted/50 data-[selected=true]:bg-muted"
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
      <div className="border-t px-3 py-2 text-xs text-muted-foreground">
        <InfiniteLoadSentinel
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          onLoadMore={onLoadMore}
        />
        {commits.length} / {page.totalCount} commits loaded
        {isFetchingNextPage ? " · loading older commits" : ""}
      </div>
    </div>
  );
}

function InfiniteLoadSentinel({
  hasNextPage,
  isFetchingNextPage,
  onLoadMore,
}: {
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onLoadMore: () => void;
}) {
  const sentinelRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const node = sentinelRef.current;
    if (!node || !hasNextPage || isFetchingNextPage) {
      return;
    }

    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        onLoadMore();
      }
    });
    observer.observe(node);

    return () => observer.disconnect();
  }, [hasNextPage, isFetchingNextPage, onLoadMore]);

  return <div ref={sentinelRef} className="h-px w-px" aria-hidden />;
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

function MarkdownWorkspaceTab({
  worktree,
  onSendAnnotationPrompt,
}: {
  worktree: GitWorktree;
  onSendAnnotationPrompt?: (prompt: string) => void;
}) {
  const previewPaneRef = useRef<HTMLDivElement | null>(null);
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(() => new Set());
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [annotationsByFile, setAnnotationsByFile] = useState<Record<string, AnnotationDraft[]>>({});
  const [draftTarget, setDraftTarget] = useState<AnnotationDraftTarget | null>(null);
  const [draftType, setDraftType] = useState<AnnotationType>("note");
  const [draftComment, setDraftComment] = useState("");
  const [selectionText, setSelectionText] = useState("");
  const [selectionAnchors, setSelectionAnchors] = useState<AnnotationAnchor[]>([]);
  const filesQuery = useQuery({
    queryKey: worktreeFileQueryKeys.list(worktree.path),
    queryFn: () => listWorktreeFiles(worktree.path),
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
  });
  const blocks = useMemo(
    () => parseMarkdownToBlocks(previewQuery.data?.content ?? ""),
    [previewQuery.data?.content],
  );
  const annotations = selectedFilePath ? (annotationsByFile[selectedFilePath] ?? []) : [];
  const annotationPrompt = selectedFilePath
    ? formatAnnotationsForAgent(selectedFilePath, annotations, blocks)
    : "";
  const inlineAnnotationsByBlock = useMemo(
    () => buildInlineAnnotationsByBlock(annotations, blocks),
    [annotations, blocks],
  );

  function selectMarkdownFile(path: string) {
    setSelectedFilePath(path);
    setDraftTarget(null);
    setDraftComment("");
    setDraftType("note");
    resetSelectionState();
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
  }

  function saveAnnotation() {
    if (!selectedFilePath || !draftTarget || (draftType !== "delete" && !draftComment.trim())) {
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
              comment: draftComment.trim(),
              createdAt,
              fileName: selectedFilePath,
              type: draftType,
            }),
          ]
        : draftTarget.anchors.map((anchor) =>
            createAnnotationFromAnchor({
              anchor,
              comment: draftComment.trim(),
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
    setDraftTarget(null);
    setDraftComment("");
    setDraftType("note");
    resetSelectionState();
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
    setSelectionText("");
    setSelectionAnchors([]);
    window.getSelection()?.removeAllRanges();
  }

  function captureSelection() {
    const anchors = getSelectionAnchors(previewPaneRef.current);
    if (anchors.length === 0) {
      setSelectionText("");
      setSelectionAnchors([]);
      return;
    }

    setSelectionAnchors(anchors);
    setSelectionText(anchors.map((anchor) => anchor.selectedText).filter(Boolean).join("\n"));
  }

  function scheduleCaptureSelection() {
    window.setTimeout(captureSelection, 10);
  }

  return (
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
	                <div className="grid gap-3">
	                  {blocks.map((block) => (
	                    <MarkdownAnnotationBlock
	                      key={block.id}
	                      block={block}
	                      annotations={annotations.filter(
	                        (annotation) => annotation.anchor.blockId === block.id,
	                      )}
	                      inlineAnnotations={inlineAnnotationsByBlock.get(block.id) ?? []}
	                      onAnnotate={(type) => {
	                        setDraftTarget({ kind: "block", block });
	                        setDraftType(type);
	                        setDraftComment("");
	                      }}
	                      onRemoveAnnotation={removeAnnotation}
	                    />
	                  ))}
	                </div>
                <aside className="grid content-start gap-3">
                  <div className="rounded-md border p-3">
                    <div className="flex items-center justify-between gap-2">
                      <h3 className="text-sm font-medium">Annotations</h3>
                      <Badge variant="outline">{annotations.length}</Badge>
	                    </div>
	                    {selectionText ? (
	                      <div className="mt-3 rounded-md border bg-muted/30 p-2">
	                        <div className="flex items-start justify-between gap-2">
	                          <div className="min-w-0">
	                            <p className="text-xs font-medium">Selected text</p>
	                            <p className="mt-1 line-clamp-3 whitespace-pre-wrap break-words text-xs text-muted-foreground">
	                              {selectionText}
	                            </p>
	                          </div>
	                          <Button
	                            type="button"
	                            size="icon-xs"
	                            variant="ghost"
	                            aria-label="선택 영역 해제"
	                            onClick={resetSelectionState}
	                          >
	                            <XIcon />
	                          </Button>
	                        </div>
	                        <div className="mt-2 flex flex-wrap gap-1">
	                          <Button
	                            type="button"
	                            size="sm"
	                            variant="secondary"
	                            onClick={() => {
	                              setDraftTarget({
	                                kind: "selection",
	                                anchors: selectionAnchors,
	                                text: selectionText,
	                              });
	                              setDraftType("note");
	                              setDraftComment("");
	                            }}
	                          >
	                            Note
	                          </Button>
	                          <Button
	                            type="button"
	                            size="sm"
	                            variant="secondary"
	                            onClick={() => {
	                              setDraftTarget({
	                                kind: "selection",
	                                anchors: selectionAnchors,
	                                text: selectionText,
	                              });
	                              setDraftType("change-request");
	                              setDraftComment("");
	                            }}
	                          >
	                            Change
	                          </Button>
	                          <Button
	                            type="button"
	                            size="sm"
	                            variant="destructive"
	                            onClick={() => {
	                              setDraftTarget({
	                                kind: "selection",
	                                anchors: selectionAnchors,
	                                text: selectionText,
	                              });
	                              setDraftType("delete");
	                              setDraftComment("");
	                            }}
	                          >
	                            Delete
	                          </Button>
	                        </div>
	                      </div>
	                    ) : null}
	                    {draftTarget ? (
	                      <div className="mt-3 grid gap-2">
	                        <div className="flex rounded-md border p-0.5">
	                          <Button
	                            type="button"
	                            size="sm"
	                            variant={draftType === "note" ? "secondary" : "ghost"}
	                            onClick={() => setDraftType("note")}
	                          >
	                            Note
	                          </Button>
	                          <Button
	                            type="button"
	                            size="sm"
	                            variant={draftType === "change-request" ? "secondary" : "ghost"}
	                            onClick={() => setDraftType("change-request")}
	                          >
	                            Change
	                          </Button>
	                          <Button
	                            type="button"
	                            size="sm"
	                            variant={draftType === "delete" ? "destructive" : "ghost"}
	                            onClick={() => setDraftType("delete")}
	                          >
	                            Delete
	                          </Button>
	                        </div>
	                        <p className="text-xs text-muted-foreground">
	                          {formatDraftTargetRange(draftTarget)}
	                        </p>
	                        <Textarea
	                          value={draftComment}
	                          onChange={(event) => setDraftComment(event.target.value)}
	                          placeholder={
	                            draftType === "delete"
	                              ? "삭제 이유를 입력하세요. 선택 사항입니다."
	                              : draftType === "change-request"
	                                ? "변경 요청 내용을 입력하세요."
	                                : "참고 메모를 입력하세요."
	                          }
	                          className="min-h-24 text-sm"
	                        />
	                        <div className="flex justify-end gap-2">
	                          <Button
	                            type="button"
	                            size="sm"
	                            variant="ghost"
	                            onClick={() => {
	                              setDraftTarget(null);
	                              setDraftComment("");
	                            }}
	                          >
	                            Cancel
	                          </Button>
	                          <Button
	                            type="button"
	                            size="sm"
	                            disabled={draftType !== "delete" && !draftComment.trim()}
	                            onClick={saveAnnotation}
	                          >
	                            Add
	                          </Button>
	                        </div>
                      </div>
                    ) : annotations.length === 0 ? (
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
                </aside>
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

function MarkdownAnnotationBlock({
  block,
  annotations,
  inlineAnnotations,
  onAnnotate,
  onRemoveAnnotation,
}: {
  block: MarkdownBlock;
  annotations: AnnotationDraft[];
  inlineAnnotations: AnnotationDraft[];
  onAnnotate: (type: AnnotationType) => void;
  onRemoveAnnotation: (annotationId: string) => void;
}) {
  const hasAnnotations = annotations.length > 0;
  const deleted = annotations.some((annotation) => isFullBlockAnnotation(annotation, block) && annotation.type === "delete");
  const noteAnnotations = annotations.filter((annotation) => annotation.type === "note");

  return (
    <section
      data-block-id={block.id}
      data-start-line={block.startLine}
      data-end-line={block.endLine}
      className={cn(
        "group/markdown-block rounded-md border bg-background p-3 transition-colors",
        hasAnnotations && "border-primary/30 bg-primary/5",
        deleted && "border-destructive/30 bg-destructive/5",
      )}
    >
      <div className="flex items-start gap-3">
        <div
          className={cn(
            "min-w-0 flex-1",
            deleted && "text-destructive line-through decoration-destructive decoration-2",
          )}
        >
          <MarkdownBlockContent block={block} inlineAnnotations={inlineAnnotations} />
        </div>
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover/markdown-block:opacity-100 group-focus-within/markdown-block:opacity-100">
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            aria-label="Note annotation 추가"
            onClick={() => onAnnotate("note")}
          >
            <MessageSquareIcon />
          </Button>
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            aria-label="Delete annotation 추가"
            onClick={() => onAnnotate("delete")}
          >
            <Trash2Icon />
          </Button>
          <Button
            type="button"
            size="icon-xs"
            variant="ghost"
            aria-label="Change request annotation 추가"
            onClick={() => onAnnotate("change-request")}
          >
            <PencilLineIcon />
          </Button>
        </div>
      </div>
      {noteAnnotations.length > 0 ? (
        <div className="mt-3 grid gap-2 border-t pt-3">
          {noteAnnotations.map((annotation) => (
            <div
              key={annotation.id}
              className="flex items-start justify-between gap-2 rounded-md bg-background px-2 py-1.5 text-sm"
            >
              <div className="min-w-0">
                <Badge variant="outline">{annotation.type}</Badge>
                <p className="mt-1 break-words text-muted-foreground">{annotation.comment}</p>
              </div>
              <Button
                type="button"
                size="icon-xs"
                variant="ghost"
                aria-label="Annotation 삭제"
                onClick={() => onRemoveAnnotation(annotation.id)}
              >
                <Trash2Icon />
              </Button>
            </div>
          ))}
        </div>
      ) : null}
      {annotations.length > 0 && noteAnnotations.length === 0 ? (
        <div className="mt-3 grid gap-2 border-t pt-3">
          {annotations.map((annotation) => (
            <div
              key={annotation.id}
              className="flex items-start justify-between gap-2 rounded-md bg-background px-2 py-1.5 text-sm"
            >
              <div className="min-w-0">
                <Badge variant={annotation.type === "delete" ? "destructive" : "outline"}>
                  {annotation.type}
                </Badge>
                {annotation.comment ? (
                  <p className="mt-1 break-words text-muted-foreground">{annotation.comment}</p>
                ) : null}
              </div>
              <Button
                type="button"
                size="icon-xs"
                variant="ghost"
                aria-label="Annotation 삭제"
                onClick={() => onRemoveAnnotation(annotation.id)}
              >
                <Trash2Icon />
              </Button>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function MarkdownBlockContent({
  block,
  inlineAnnotations,
}: {
  block: MarkdownBlock;
  inlineAnnotations: AnnotationDraft[];
}) {
  const content = (
    <AnnotatedInlineText annotations={inlineAnnotations}>{block.content}</AnnotatedInlineText>
  );

  if (block.type === "heading") {
    const Tag = `h${block.level ?? 1}` as ElementType;
    return <Tag data-block-content>{content}</Tag>;
  }

  if (block.type === "code") {
    return (
      <pre data-block-content>
        <code>{content}</code>
      </pre>
    );
  }

  if (block.type === "hr") {
    return (
      <div data-block-content>
        <hr />
      </div>
    );
  }

  if (block.type === "paragraph" || block.type === "blockquote" || block.type === "list-item") {
    return (
      <div data-block-content>
        {inlineAnnotations.length > 0 ? (
          content
        ) : (
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{block.rawContent}</ReactMarkdown>
        )}
      </div>
    );
  }

  return (
    <div data-block-content>
      <Markdown>{block.rawContent}</Markdown>
    </div>
  );
}

function AnnotatedInlineText({
  annotations,
  children,
}: {
  annotations: AnnotationDraft[];
  children: string;
}) {
  const sortedAnnotations = [...annotations]
    .filter(
      (annotation) =>
        annotation.anchor.startOffset !== undefined &&
        annotation.anchor.endOffset !== undefined &&
        annotation.anchor.endOffset > annotation.anchor.startOffset,
    )
    .sort((left, right) => (left.anchor.startOffset ?? 0) - (right.anchor.startOffset ?? 0));
  const segments: ReactNode[] = [];
  let cursor = 0;

  for (const annotation of sortedAnnotations) {
    const startOffset = Math.max(0, Math.min(annotation.anchor.startOffset ?? 0, children.length));
    const endOffset = Math.max(startOffset, Math.min(annotation.anchor.endOffset ?? startOffset, children.length));
    if (startOffset < cursor || startOffset === endOffset) {
      continue;
    }

    if (cursor < startOffset) {
      segments.push(children.slice(cursor, startOffset));
    }

    segments.push(
      <mark
        key={annotation.id}
        className={cn(
          "rounded-sm px-0.5",
          annotation.type === "delete" &&
            "bg-transparent text-destructive line-through decoration-destructive decoration-2",
          annotation.type === "change-request" && "bg-sky-100 text-foreground",
          annotation.type === "note" && "bg-yellow-200 text-foreground",
        )}
        title={annotation.comment || annotation.type}
      >
        {children.slice(startOffset, endOffset)}
      </mark>,
    );
    cursor = endOffset;
  }

  if (cursor < children.length) {
    segments.push(children.slice(cursor));
  }

  return <>{segments}</>;
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

function isMarkdownPath(path: string) {
  const normalized = path.toLowerCase();
  return (
    normalized.endsWith(".md") ||
    normalized.endsWith(".markdown") ||
    normalized.endsWith(".mdx")
  );
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

function refsByTarget(refs: GitGraphRef[]) {
  const result = new Map<string, GitGraphRef[]>();

  for (const ref of refs) {
    const existing = result.get(ref.target) ?? [];
    existing.push(ref);
    result.set(ref.target, existing);
  }

  return result;
}

function combineGitHistoryPages(pages: Array<{ commits: GitCommitSummary[]; page: GitCommitGraph["page"] }>) {
  if (pages.length === 0) {
    return null;
  }

  const commits = pages.flatMap((page) => page.commits);
  const lastPage = pages[pages.length - 1].page;

  return {
    commits,
    page: {
      ...lastPage,
      offset: 0,
      limit: commits.length,
      hasMore: lastPage.hasMore,
    },
  };
}

function combineGitGraphPages(pages: GitCommitGraph[]) {
  if (pages.length === 0) {
    return null;
  }

  const commits: GitGraphCommit[] = [];
  const commitHashes = new Set<string>();
  const refs = new Map<string, GitGraphRef>();
  let layoutHints: GitGraphLayoutHints = pages[0].layoutHints;
  let totalCount = pages[0].page.totalCount;
  let hasMore = false;

  for (const page of pages) {
    layoutHints = page.layoutHints;
    totalCount = page.page.totalCount;
    hasMore = page.page.hasMore;

    for (const commit of page.commits) {
      if (!commitHashes.has(commit.hash)) {
        commitHashes.add(commit.hash);
        commits.push(commit);
      }
    }

    for (const ref of page.refs) {
      refs.set(`${ref.kind}:${ref.name}:${ref.target}`, ref);
    }
  }

  return {
    commits,
    refs: [...refs.values()],
    layoutHints,
    page: {
      offset: 0,
      limit: commits.length,
      totalCount,
      hasMore,
    },
  };
}

function laneX(lane: number) {
  return 10 + lane * 20;
}

function graphSegmentPath(segment: GitGraphSegment, rowHeight: number) {
  const fromX = laneX(segment.fromLane);
  const toX = laneX(segment.toLane);
  const centerY = rowHeight / 2;

  if (segment.type === "vertical") {
    return `M ${fromX} 0 L ${toX} ${rowHeight}`;
  }

  if (segment.type === "vertical-top") {
    return `M ${fromX} 0 L ${fromX} ${centerY}`;
  }

  if (segment.type === "vertical-bottom") {
    return `M ${fromX} ${centerY} L ${fromX} ${rowHeight}`;
  }

  return `M ${fromX} ${centerY} C ${fromX} ${rowHeight}, ${toX} ${rowHeight}, ${toX} ${rowHeight}`;
}

function formatDate(value: string) {
  const date = new Date(value);

  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}
