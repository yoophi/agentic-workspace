import { type RefObject, useEffect, useRef, useState } from "react";
import type { Layout } from "react-resizable-panels";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import {
  AlertCircle,
  ChevronDown,
  ChevronRight,
  EyeOff,
  Filter,
  Folder,
  FolderGit2,
  FolderOpen,
  FileText,
  GitBranch as GitBranchIcon,
  GitCommit,
  Loader2,
  RefreshCw,
} from "lucide-react";
import { Button } from "@yoophi/ui/components/button";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@yoophi/ui/components/resizable";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@yoophi/ui/components/table";
import {
  getAppInfo,
  getCommitDetail,
  getCommitGraph,
  getFileDiff,
  listBranches,
  listHistory,
  listWorktrees,
  repositoryKeys,
  type GitBranch,
  type GitCommitFileChange,
  type GitCommitGraph,
  type GitGraphCommit,
  type GitGraphRef,
  type GitWorktree,
  type Repository,
} from "@/entities/repository";
import {
  computeGitGraphRows,
  getMaxGraphLane,
  type GitGraphRow,
  type GitGraphSegment,
} from "@/features/history-tree/model/graph-layout";

type ChangesPanelProps = {
  selectedRepository?: Repository;
};

type HistoryView = "list" | "graph";
type FileChangeView = "tree" | "list";
type BranchGraphRefKind = "localBranch" | "remoteBranch";

const CHANGES_LAYOUT_STORAGE_KEY = "repository-detail-columns-layout";
const HISTORY_PAGE_SIZE = 100;
const GRAPH_PAGE_SIZE = 300;

type BranchTreeRow =
  | {
      id: string;
      depth: number;
      isExpanded: boolean;
      name: string;
      path: string;
      type: "folder";
    }
  | {
      branch: GitBranch;
      depth: number;
      id: string;
      name: string;
      type: "branch";
    };

type FileTreeRow =
  | {
      depth: number;
      id: string;
      isExpanded: boolean;
      name: string;
      path: string;
      type: "folder";
    }
  | {
      depth: number;
      file: GitCommitFileChange;
      id: string;
      name: string;
      type: "file";
    };

type FileTreeFolderNode = {
  files: GitCommitFileChange[];
  folders: Map<string, FileTreeFolderNode>;
  name: string;
  path: string;
};

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function getShortHash(hash: string) {
  return hash.slice(0, 8);
}

function getWorktreeKind(worktree: GitWorktree) {
  if (worktree.isBare) {
    return "Bare";
  }

  return worktree.isMain ? "Main" : "Linked";
}

function getShortCommit(commit: string) {
  return commit.slice(0, 8);
}

function loadColumnLayout(): Layout | undefined {
  try {
    return JSON.parse(localStorage.getItem(CHANGES_LAYOUT_STORAGE_KEY) ?? "");
  } catch {
    return undefined;
  }
}

function saveColumnLayout(layout: Layout) {
  localStorage.setItem(CHANGES_LAYOUT_STORAGE_KEY, JSON.stringify(layout));
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
          d={graphSegmentPath(segment, rowHeight)}
          fill="none"
          key={`${segment.type}:${segment.fromLane}:${segment.toLane}:${index}`}
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

function refsByTarget(refs: GitGraphRef[]) {
  const result = new Map<string, GitGraphRef[]>();

  for (const ref of refs) {
    const existing = result.get(ref.target) ?? [];
    existing.push(ref);
    result.set(ref.target, existing);
  }

  return result;
}

function branchGraphRefKind(branch: GitBranch): BranchGraphRefKind {
  return branch.isRemote ? "remoteBranch" : "localBranch";
}

function branchGraphRefKey(branch: GitBranch) {
  return `${branchGraphRefKind(branch)}:${branch.name}`;
}

function graphRefKey(ref: GitGraphRef) {
  return `${ref.kind}:${ref.name}`;
}

function branchHistoryRefs(
  branches: GitBranch[],
  filteredBranchKeys: ReadonlySet<string>,
  hiddenBranchKeys: ReadonlySet<string>,
) {
  const includedRefs: string[] = [];
  const excludedRefs: string[] = [];
  const hasBranchFilters = filteredBranchKeys.size > 0;

  for (const branch of branches) {
    const key = branchGraphRefKey(branch);

    if (hasBranchFilters && filteredBranchKeys.has(key)) {
      includedRefs.push(branch.fullName);
      continue;
    }

    if (!hasBranchFilters && hiddenBranchKeys.has(key)) {
      excludedRefs.push(branch.fullName);
    }
  }

  includedRefs.sort();
  excludedRefs.sort();

  return {
    excludedRefs,
    includedRefs,
  };
}

function getReachableCommitHashes(commits: GitGraphCommit[], targetHashes: ReadonlySet<string>) {
  const commitByHash = new Map(commits.map((commit) => [commit.hash, commit]));
  const reachable = new Set<string>();
  const stack = [...targetHashes];

  while (stack.length > 0) {
    const hash = stack.pop();

    if (!hash || reachable.has(hash)) {
      continue;
    }

    const commit = commitByHash.get(hash);

    if (!commit) {
      continue;
    }

    reachable.add(hash);
    stack.push(...commit.parents);
  }

  return reachable;
}

function filterGraphByBranchControls(
  graph: GitCommitGraph,
  filteredBranchKeys: ReadonlySet<string>,
  hiddenBranchKeys: ReadonlySet<string>,
): GitCommitGraph {
  const hasBranchFilters = filteredBranchKeys.size > 0;
  const visibleRefs = graph.refs.filter((ref) => {
    if (ref.kind === "tag") {
      return !hasBranchFilters;
    }

    const key = graphRefKey(ref);

    if (hasBranchFilters) {
      return filteredBranchKeys.has(key);
    }

    return !hiddenBranchKeys.has(key);
  });

  if (!hasBranchFilters) {
    return {
      ...graph,
      refs: visibleRefs,
    };
  }

  const targetHashes = new Set(visibleRefs.map((ref) => ref.target));
  const reachableCommitHashes = getReachableCommitHashes(graph.commits, targetHashes);
  const visibleCommits = graph.commits.filter((commit) => reachableCommitHashes.has(commit.hash));

  return {
    ...graph,
    commits: visibleCommits,
    refs: visibleRefs,
    page: {
      ...graph.page,
    },
  };
}

function combineGitCommitGraphPages(pages: GitCommitGraph[]) {
  const [firstPage] = pages;

  if (!firstPage) {
    return undefined;
  }

  const commits: GitGraphCommit[] = [];
  const commitHashes = new Set<string>();

  for (const page of pages) {
    for (const commit of page.commits) {
      if (commitHashes.has(commit.hash)) {
        continue;
      }

      commitHashes.add(commit.hash);
      commits.push(commit);
    }
  }

  const lastPage = pages[pages.length - 1] ?? firstPage;

  return {
    ...firstPage,
    commits,
    page: {
      ...lastPage.page,
      offset: 0,
    },
  };
}

function diffLineClassName(line: string) {
  if (line.startsWith("+++") || line.startsWith("---")) {
    return "bg-muted/70 text-muted-foreground";
  }

  if (line.startsWith("@@")) {
    return "bg-blue-500/10 text-blue-700 dark:text-blue-300";
  }

  if (line.startsWith("+")) {
    return "bg-green-500/15 text-green-800 dark:text-green-200";
  }

  if (line.startsWith("-")) {
    return "bg-red-500/15 text-red-800 dark:text-red-200";
  }

  if (line.startsWith("diff --git") || line.startsWith("index ")) {
    return "bg-muted/40 text-muted-foreground";
  }

  return "text-foreground";
}

function fileStatusClassName(status: string) {
  const normalizedStatus = status.charAt(0).toUpperCase();

  if (normalizedStatus === "A") {
    return "border-green-500/30 bg-green-500/10 text-green-700 dark:text-green-300";
  }

  if (normalizedStatus === "M") {
    return "border-yellow-500/30 bg-yellow-500/10 text-yellow-700 dark:text-yellow-300";
  }

  if (normalizedStatus === "D") {
    return "border-red-500/30 bg-red-500/10 text-red-700 dark:text-red-300";
  }

  if (normalizedStatus === "R") {
    return "border-blue-500/30 bg-blue-500/10 text-blue-700 dark:text-blue-300";
  }

  if (normalizedStatus === "C") {
    return "border-purple-500/30 bg-purple-500/10 text-purple-700 dark:text-purple-300";
  }

  return "border-border bg-background text-muted-foreground";
}

type DiffLine = {
  content: string;
  newLineNumber?: number;
  oldLineNumber?: number;
};

function parseHunkHeader(line: string) {
  const match = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);

  if (!match) {
    return null;
  }

  return {
    oldLineNumber: Number.parseInt(match[1], 10),
    newLineNumber: Number.parseInt(match[2], 10),
  };
}

function parseDiffLines(content: string): DiffLine[] {
  const lines = content ? content.split("\n") : ["No text diff available."];
  let oldLineNumber = 0;
  let newLineNumber = 0;

  return lines.map((line) => {
    const hunk = parseHunkHeader(line);

    if (hunk) {
      oldLineNumber = hunk.oldLineNumber;
      newLineNumber = hunk.newLineNumber;
      return { content: line };
    }

    if (line.startsWith("+++") || line.startsWith("---")) {
      return { content: line };
    }

    if (line.startsWith("+")) {
      const currentNewLineNumber = newLineNumber;
      newLineNumber += 1;

      return {
        content: line,
        newLineNumber: currentNewLineNumber,
      };
    }

    if (line.startsWith("-")) {
      const currentOldLineNumber = oldLineNumber;
      oldLineNumber += 1;

      return {
        content: line,
        oldLineNumber: currentOldLineNumber,
      };
    }

    if (
      line.startsWith("diff --git") ||
      line.startsWith("index ") ||
      line.startsWith("new file mode ") ||
      line.startsWith("deleted file mode ")
    ) {
      return { content: line };
    }

    const currentOldLineNumber = oldLineNumber;
    const currentNewLineNumber = newLineNumber;
    oldLineNumber += 1;
    newLineNumber += 1;

    return {
      content: line,
      oldLineNumber: currentOldLineNumber,
      newLineNumber: currentNewLineNumber,
    };
  });
}

function DiffViewer({ content }: { content: string }) {
  const lines = parseDiffLines(content);

  return (
    <pre className="max-h-96 overflow-auto rounded-md border bg-background font-mono text-xs leading-5">
      {lines.map((line, index) => (
        <div
          className={`grid min-w-max grid-cols-[3.5rem_3.5rem_minmax(0,1fr)] whitespace-pre ${diffLineClassName(line.content)}`}
          key={`${index}:${line.content}`}
        >
          <span className="select-none border-r px-2 text-right text-muted-foreground/70">
            {line.oldLineNumber ?? ""}
          </span>
          <span className="select-none border-r px-2 text-right text-muted-foreground/70">
            {line.newLineNumber ?? ""}
          </span>
          <span className="px-3">{line.content || " "}</span>
        </div>
      ))}
    </pre>
  );
}

function HistoryGraphView({
  graph,
  graphRefs,
  graphRows,
  hasNextPage,
  isFetchingNextPage,
  loadMoreRef,
  maxGraphLane,
  onSelectCommit,
  selectedCommitHash,
}: {
  graph: GitCommitGraph;
  graphRefs: Map<string, GitGraphRef[]>;
  graphRows: Map<string, GitGraphRow>;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  loadMoreRef: RefObject<HTMLDivElement | null>;
  maxGraphLane: number;
  onSelectCommit: (commitHash: string) => void;
  selectedCommitHash?: string;
}) {
  const rowHeight = graph.layoutHints.rowHeight || 32;

  return (
    <div className="overflow-hidden rounded-md border">
      <div className="grid grid-cols-[auto_minmax(0,1fr)_9rem_12rem] border-b bg-muted/40 px-2 py-2 text-xs font-medium text-muted-foreground">
        <span>Graph</span>
        <span>Commit</span>
        <span>Author</span>
        <span>Date</span>
      </div>
      <div>
        {graph.commits.map((commit) => (
          <HistoryGraphRow
            commit={commit}
            graphRefs={graphRefs.get(commit.hash) ?? []}
            graphRow={graphRows.get(commit.hash)}
            isSelected={commit.hash === selectedCommitHash}
            key={commit.hash}
            maxGraphLane={maxGraphLane}
            onSelectCommit={onSelectCommit}
            rowHeight={rowHeight}
          />
        ))}
      </div>
      <div className="border-t px-3 py-2 text-xs text-muted-foreground">
        <div ref={loadMoreRef} className="h-px w-px shrink-0" aria-hidden />
        {graph.commits.length} / {graph.page.totalCount} commits loaded
        {hasNextPage ? " · more commits available" : ""}
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
  onSelectCommit,
  rowHeight,
}: {
  commit: GitGraphCommit;
  graphRefs: GitGraphRef[];
  graphRow?: GitGraphRow;
  isSelected: boolean;
  maxGraphLane: number;
  onSelectCommit: (commitHash: string) => void;
  rowHeight: number;
}) {
  return (
    <button
      aria-label={`Commit ${commit.shortHash} by ${commit.author}: ${commit.message}`}
      className="grid w-full grid-cols-[auto_minmax(0,1fr)_9rem_12rem] items-center border-b px-2 text-left text-sm last:border-b-0 hover:bg-muted/50 data-[selected=true]:bg-muted"
      data-selected={isSelected}
      onClick={() => onSelectCommit(commit.hash)}
      style={{ minHeight: rowHeight }}
      type="button"
    >
      <GraphCell maxLane={maxGraphLane} row={graphRow} rowHeight={rowHeight} />
      <span className="flex min-w-0 items-center gap-2 pr-2">
        <span className="font-mono text-xs text-muted-foreground">{commit.shortHash}</span>
        {graphRefs.map((ref) => (
          <span
            className="max-w-40 truncate rounded-sm border bg-background px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground"
            key={`${ref.kind}:${ref.name}`}
            title={ref.name}
          >
            {ref.kind === "tag" ? "tag:" : ""}
            {ref.name}
          </span>
        ))}
        <span className="min-w-0 truncate">{commit.message}</span>
      </span>
      <span className="truncate pr-2 text-xs text-muted-foreground">{commit.author}</span>
      <span className="truncate font-mono text-xs text-muted-foreground">{commit.date}</span>
    </button>
  );
}

function getBranchFolderPaths(branches: GitBranch[]) {
  const folders = new Set<string>();

  for (const branch of branches) {
    const segments = branch.name.split("/").filter(Boolean);
    let folderPath = "";

    for (const segment of segments.slice(0, -1)) {
      folderPath = folderPath ? `${folderPath}/${segment}` : segment;
      folders.add(folderPath);
    }
  }

  return folders;
}

function isBranchVisible(branchName: string, expandedFolders: ReadonlySet<string>) {
  const segments = branchName.split("/").filter(Boolean);
  let folderPath = "";

  for (const segment of segments.slice(0, -1)) {
    folderPath = folderPath ? `${folderPath}/${segment}` : segment;

    if (!expandedFolders.has(folderPath)) {
      return false;
    }
  }

  return true;
}

function buildBranchTreeRows(
  branches: GitBranch[],
  expandedFolders: ReadonlySet<string>,
): BranchTreeRow[] {
  const rows: BranchTreeRow[] = [];
  const folders = new Set<string>();

  for (const branch of [...branches].sort((a, b) => a.name.localeCompare(b.name))) {
    const segments = branch.name.split("/").filter(Boolean);
    let folderPath = "";

    for (const [index, segment] of segments.slice(0, -1).entries()) {
      folderPath = folderPath ? `${folderPath}/${segment}` : segment;
      const parentPath = folderPath.includes("/")
        ? folderPath.slice(0, folderPath.lastIndexOf("/"))
        : "";

      if (parentPath && !expandedFolders.has(parentPath)) {
        continue;
      }

      if (!folders.has(folderPath)) {
        folders.add(folderPath);
        rows.push({
          id: `folder:${folderPath}`,
          depth: index,
          isExpanded: expandedFolders.has(folderPath),
          name: segment,
          path: folderPath,
          type: "folder",
        });
      }
    }

    if (isBranchVisible(branch.name, expandedFolders)) {
      rows.push({
        branch,
        depth: Math.max(segments.length - 1, 0),
        id: branch.fullName,
        name: segments[segments.length - 1] ?? branch.name,
        type: "branch",
      });
    }
  }

  return rows;
}

function getFileFolderPaths(files: GitCommitFileChange[]) {
  const folders = new Set<string>();

  for (const file of files) {
    const segments = file.path.split("/").filter(Boolean);
    let folderPath = "";

    for (const segment of segments.slice(0, -1)) {
      folderPath = folderPath ? `${folderPath}/${segment}` : segment;
      folders.add(folderPath);
    }
  }

  return folders;
}

function createFileTreeFolderNode(name: string, path: string): FileTreeFolderNode {
  return {
    files: [],
    folders: new Map(),
    name,
    path,
  };
}

function buildFileTree(files: GitCommitFileChange[]) {
  const root = createFileTreeFolderNode("", "");

  for (const file of [...files].sort((a, b) => a.path.localeCompare(b.path))) {
    const segments = file.path.split("/").filter(Boolean);

    if (segments.length === 0) {
      continue;
    }

    let current = root;

    for (const segment of segments.slice(0, -1)) {
      const folderPath = current.path ? `${current.path}/${segment}` : segment;
      let child = current.folders.get(segment);

      if (!child) {
        child = createFileTreeFolderNode(segment, folderPath);
        current.folders.set(segment, child);
      }

      current = child;
    }

    current.files.push(file);
  }

  return root;
}

function compressFileTreeFolder(node: FileTreeFolderNode) {
  const names = [node.name];
  let current = node;

  while (current.files.length === 0 && current.folders.size === 1) {
    const [next] = current.folders.values();

    if (!next) {
      break;
    }

    names.push(next.name);
    current = next;
  }

  return {
    name: names.join("/"),
    node: current,
  };
}

function appendFileTreeRows(
  node: FileTreeFolderNode,
  depth: number,
  expandedFolders: ReadonlySet<string>,
  rows: FileTreeRow[],
) {
  const folderNodes = [...node.folders.values()].sort((a, b) => a.name.localeCompare(b.name));

  for (const folderNode of folderNodes) {
    const compressedFolder = compressFileTreeFolder(folderNode);
    const isExpanded = expandedFolders.has(compressedFolder.node.path);

    rows.push({
      depth,
      id: `folder:${compressedFolder.node.path}`,
      isExpanded,
      name: compressedFolder.name,
      path: compressedFolder.node.path,
      type: "folder",
    });

    if (isExpanded) {
      appendFileTreeRows(compressedFolder.node, depth + 1, expandedFolders, rows);
    }
  }

  for (const file of [...node.files].sort((a, b) => a.path.localeCompare(b.path))) {
    const segments = file.path.split("/").filter(Boolean);

    rows.push({
      depth,
      file,
      id: `file:${file.status}:${file.path}`,
      name: segments[segments.length - 1] ?? file.path,
      type: "file",
    });
  }
}

function buildFileTreeRows(
  files: GitCommitFileChange[],
  expandedFolders: ReadonlySet<string>,
): FileTreeRow[] {
  const rows: FileTreeRow[] = [];
  appendFileTreeRows(buildFileTree(files), 0, expandedFolders, rows);

  return rows;
}

export function ChangesPanel({ selectedRepository }: ChangesPanelProps) {
  const [selectedCommitHash, setSelectedCommitHash] = useState<string>();
  const [selectedFilePath, setSelectedFilePath] = useState<string>();
  const [historyView, setHistoryView] = useState<HistoryView>("list");
  const [fileChangeView, setFileChangeView] = useState<FileChangeView>("tree");
  const [expandedBranchFolders, setExpandedBranchFolders] = useState<Set<string>>(new Set());
  const [expandedFileFolders, setExpandedFileFolders] = useState<Set<string>>(new Set());
  const [filteredBranchKeys, setFilteredBranchKeys] = useState<Set<string>>(new Set());
  const [hiddenBranchKeys, setHiddenBranchKeys] = useState<Set<string>>(new Set());
  const commitLogLoadMoreRef = useRef<HTMLDivElement | null>(null);
  const appInfo = useQuery({
    queryKey: ["app-info"],
    queryFn: getAppInfo,
  });
  const worktreesQuery = useQuery({
    enabled: Boolean(selectedRepository),
    queryKey: selectedRepository
      ? repositoryKeys.worktrees(selectedRepository.id)
      : ["repositories", "unselected", "worktrees"],
    queryFn: () => listWorktrees(selectedRepository?.id ?? ""),
  });
  const branchesQuery = useQuery({
    enabled: Boolean(selectedRepository),
    queryKey: selectedRepository
      ? repositoryKeys.branches(selectedRepository.id)
      : ["repositories", "unselected", "branches"],
    queryFn: () => listBranches(selectedRepository?.id ?? ""),
  });
  const branchRefs = branchHistoryRefs(
    branchesQuery.data ?? [],
    filteredBranchKeys,
    hiddenBranchKeys,
  );
  const branchRefsSignature = `${branchRefs.includedRefs.join("\0")}\u0001${branchRefs.excludedRefs.join("\0")}`;
  const historyQuery = useInfiniteQuery({
    enabled: Boolean(selectedRepository),
    queryKey: selectedRepository
      ? repositoryKeys.history(selectedRepository.id, {
          excludedRefs: branchRefs.excludedRefs,
          includedRefs: branchRefs.includedRefs,
          maxCount: HISTORY_PAGE_SIZE,
        })
      : ["repositories", "unselected", "history"],
    queryFn: ({ pageParam }) =>
      listHistory(selectedRepository?.id ?? "", {
        excludedRefs: branchRefs.excludedRefs,
        includedRefs: branchRefs.includedRefs,
        maxCount: HISTORY_PAGE_SIZE,
        offset: pageParam,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) =>
      lastPage.page.hasMore ? lastPage.page.offset + lastPage.commits.length : undefined,
  });
  const graphQuery = useInfiniteQuery({
    enabled: Boolean(selectedRepository),
    queryKey: selectedRepository
      ? repositoryKeys.commitGraph(selectedRepository.id, {
          excludedRefs: branchRefs.excludedRefs,
          includedRefs: branchRefs.includedRefs,
          maxCount: GRAPH_PAGE_SIZE,
        })
      : ["repositories", "unselected", "commitGraph"],
    queryFn: ({ pageParam }) =>
      getCommitGraph(selectedRepository?.id ?? "", {
        excludedRefs: branchRefs.excludedRefs,
        includedRefs: branchRefs.includedRefs,
        maxCount: GRAPH_PAGE_SIZE,
        offset: pageParam,
      }),
    initialPageParam: 0,
    getNextPageParam: (lastPage) =>
      lastPage.page.hasMore ? lastPage.page.offset + lastPage.commits.length : undefined,
  });
  const commitDetailQuery = useQuery({
    enabled: Boolean(selectedRepository && selectedCommitHash),
    queryKey:
      selectedRepository && selectedCommitHash
        ? repositoryKeys.commitDetail(selectedRepository.id, selectedCommitHash)
        : ["repositories", "unselected", "commits", "unselected"],
    queryFn: () => getCommitDetail(selectedRepository?.id ?? "", selectedCommitHash ?? ""),
  });
  const fileDiffQuery = useQuery({
    enabled: Boolean(selectedRepository && selectedCommitHash && selectedFilePath),
    queryKey:
      selectedRepository && selectedCommitHash && selectedFilePath
        ? repositoryKeys.fileDiff(selectedRepository.id, selectedCommitHash, selectedFilePath)
        : ["repositories", "unselected", "commits", "unselected", "files", "unselected", "diff"],
    queryFn: () =>
      getFileDiff(selectedRepository?.id ?? "", selectedCommitHash ?? "", selectedFilePath ?? ""),
  });
  const branchRows = buildBranchTreeRows(branchesQuery.data ?? [], expandedBranchFolders);
  const fileRows = buildFileTreeRows(commitDetailQuery.data?.files ?? [], expandedFileFolders);
  const historyCommits = historyQuery.data?.pages.flatMap((page) => page.commits) ?? [];
  const rawGraphData = combineGitCommitGraphPages(graphQuery.data?.pages ?? []);
  const graphData = rawGraphData
    ? filterGraphByBranchControls(rawGraphData, filteredBranchKeys, hiddenBranchKeys)
    : undefined;
  const graphRows = graphData ? computeGitGraphRows(graphData.commits) : new Map<string, GitGraphRow>();
  const maxGraphLane = getMaxGraphLane(graphRows);
  const graphRefs = graphData ? refsByTarget(graphData.refs) : new Map<string, GitGraphRef[]>();
  const isRefreshing =
    worktreesQuery.isFetching ||
    branchesQuery.isFetching ||
    historyQuery.isFetching ||
    graphQuery.isFetching ||
    commitDetailQuery.isFetching ||
    fileDiffQuery.isFetching;

  useEffect(() => {
    setSelectedCommitHash(undefined);
    setSelectedFilePath(undefined);
    setFilteredBranchKeys(new Set());
    setHiddenBranchKeys(new Set());
  }, [selectedRepository?.id]);

  useEffect(() => {
    setSelectedFilePath(undefined);
  }, [selectedCommitHash]);

  useEffect(() => {
    setSelectedCommitHash(undefined);
    setSelectedFilePath(undefined);
  }, [branchRefsSignature]);

  useEffect(() => {
    setExpandedBranchFolders(getBranchFolderPaths(branchesQuery.data ?? []));
  }, [branchesQuery.data]);

  useEffect(() => {
    setExpandedFileFolders(getFileFolderPaths(commitDetailQuery.data?.files ?? []));
  }, [commitDetailQuery.data?.files]);

  useEffect(() => {
    const loadMoreElement = commitLogLoadMoreRef.current;

    if (!loadMoreElement || !selectedRepository) {
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) {
          return;
        }

        if (
          historyView === "list" &&
          historyQuery.hasNextPage &&
          !historyQuery.isFetchingNextPage
        ) {
          void historyQuery.fetchNextPage();
          return;
        }

        if (
          historyView === "graph" &&
          graphQuery.hasNextPage &&
          !graphQuery.isFetchingNextPage
        ) {
          void graphQuery.fetchNextPage();
        }
      },
      {
        rootMargin: "240px 0px",
      },
    );

    observer.observe(loadMoreElement);

    return () => {
      observer.disconnect();
    };
  }, [
    graphQuery,
    graphQuery.hasNextPage,
    graphQuery.isFetchingNextPage,
    historyQuery,
    historyQuery.hasNextPage,
    historyQuery.isFetchingNextPage,
    historyView,
    selectedRepository,
  ]);

  function toggleBranchFolder(path: string) {
    setExpandedBranchFolders((current) => {
      const next = new Set(current);

      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }

      return next;
    });
  }

  function toggleBranchFilter(branch: GitBranch) {
    const key = branchGraphRefKey(branch);

    setFilteredBranchKeys((current) => {
      const next = new Set(current);

      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }

      return next;
    });
  }

  function toggleBranchHidden(branch: GitBranch) {
    const key = branchGraphRefKey(branch);

    setHiddenBranchKeys((current) => {
      const next = new Set(current);

      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }

      return next;
    });
  }

  function toggleFileFolder(path: string) {
    setExpandedFileFolders((current) => {
      const next = new Set(current);

      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }

      return next;
    });
  }

  const repositoryInfo = (
    <section className="flex h-full min-h-0 flex-col">
      <header className="flex items-center justify-between border-b px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <FolderGit2 className="size-4 text-muted-foreground" />
          <div className="min-w-0">
            <h2 className="truncate text-sm font-medium">
              {selectedRepository?.name ?? "Repository info"}
            </h2>
            <p className="truncate text-xs text-muted-foreground">
              {selectedRepository?.path ??
                (appInfo.data
                  ? `${appInfo.data.name} ${appInfo.data.version}`
                  : "Select a repository")}
            </p>
          </div>
        </div>
        <Button
          aria-label="Refresh repository data"
          disabled={!selectedRepository || isRefreshing}
          size="icon-sm"
          variant="outline"
          onClick={() => {
            void worktreesQuery.refetch();
            void branchesQuery.refetch();
            void historyQuery.refetch();
            void graphQuery.refetch();
            if (selectedCommitHash) {
              void commitDetailQuery.refetch();
            }
            if (selectedFilePath) {
              void fileDiffQuery.refetch();
            }
          }}
        >
          {isRefreshing ? <Loader2 className="animate-spin" /> : <RefreshCw />}
        </Button>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        {!selectedRepository ? (
          <div className="flex h-full min-h-80 items-center justify-center">
            <div className="max-w-xs text-center">
              <FolderGit2 className="mx-auto size-10 text-muted-foreground" />
              <h2 className="mt-3 text-sm font-medium">No repository selected</h2>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                Add a local Git repository from the sidebar, then select it.
              </p>
            </div>
          </div>
        ) : (
          <div className="grid gap-6">
            <div className="grid gap-2">
              <h3 className="text-sm font-medium">Info</h3>
              <div className="grid gap-2 rounded-md border p-3 text-sm">
                <div className="grid gap-1">
                  <span className="text-xs text-muted-foreground">Name</span>
                  <span className="truncate">{selectedRepository.name}</span>
                </div>
                <div className="grid gap-1">
                  <span className="text-xs text-muted-foreground">Path</span>
                  <span className="truncate font-mono text-xs">{selectedRepository.path}</span>
                </div>
              </div>
            </div>
            <div>
              <div className="mb-2 flex items-center gap-2">
                <FolderGit2 className="size-4 text-muted-foreground" />
                <h3 className="text-sm font-medium">Worktrees</h3>
              </div>
              {worktreesQuery.isLoading ? (
                <p className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="size-4 animate-spin" />
                  Loading worktrees
                </p>
              ) : worktreesQuery.isError ? (
                <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
                  <AlertCircle className="mt-0.5 size-4 shrink-0" />
                  <span>{getErrorMessage(worktreesQuery.error)}</span>
                </p>
              ) : worktreesQuery.data?.length === 0 ? (
                <p className="text-sm text-muted-foreground">No worktrees found.</p>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Path</TableHead>
                      <TableHead className="w-28">Branch</TableHead>
                      <TableHead className="w-20">Kind</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {worktreesQuery.data?.map((worktree) => (
                      <TableRow key={worktree.path}>
                        <TableCell className="max-w-0 truncate font-mono text-xs">
                          {worktree.path}
                        </TableCell>
                        <TableCell className="max-w-0 truncate">
                          {worktree.branch ?? getShortCommit(worktree.commit)}
                        </TableCell>
                        <TableCell>{getWorktreeKind(worktree)}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </div>
            <div>
              <div className="mb-2 flex items-center gap-2">
                <GitBranchIcon className="size-4 text-muted-foreground" />
                <h3 className="text-sm font-medium">Branches</h3>
              </div>
              {branchesQuery.isLoading ? (
                <p className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="size-4 animate-spin" />
                  Loading branches
                </p>
              ) : branchesQuery.isError ? (
                <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
                  <AlertCircle className="mt-0.5 size-4 shrink-0" />
                  <span>{getErrorMessage(branchesQuery.error)}</span>
                </p>
              ) : branchRows.length === 0 ? (
                <p className="text-sm text-muted-foreground">No branches found.</p>
              ) : (
                <div className="overflow-hidden rounded-md border text-sm">
                  {branchRows.map((row) =>
                    row.type === "folder" ? (
                      <button
                        aria-expanded={row.isExpanded}
                        className="flex h-8 w-full items-center gap-1 border-b px-2 text-left last:border-b-0 hover:bg-muted/50"
                        key={row.id}
                        onClick={() => toggleBranchFolder(row.path)}
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
                      (() => {
                        const branchKey = branchGraphRefKey(row.branch);
                        const isFiltered = filteredBranchKeys.has(branchKey);
                        const isHidden = hiddenBranchKeys.has(branchKey);

                        return (
                          <div
                            className="group/branch flex h-8 items-center gap-2 border-b px-2 last:border-b-0"
                            key={row.id}
                            style={{ paddingLeft: `${28 + row.depth * 18}px` }}
                            title={row.branch.fullName}
                          >
                            <GitBranchIcon className="size-4 shrink-0 text-muted-foreground" />
                            <span className="min-w-0 flex-1 truncate">{row.name}</span>
                            <span className="shrink-0 rounded-sm border px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground">
                              {row.branch.isRemote ? "remote" : "local"}
                            </span>
                            {row.branch.isCurrent ? (
                              <span className="shrink-0 rounded-sm bg-secondary px-1.5 py-0.5 text-[10px] leading-none">
                                current
                              </span>
                            ) : null}
                            <div className="ml-auto flex shrink-0 items-center gap-0.5">
                              <Button
                                aria-label={`Filter graph to ${row.branch.name}`}
                                aria-pressed={isFiltered}
                                className={
                                  isFiltered
                                    ? "text-blue-600 opacity-100 dark:text-blue-300"
                                    : "opacity-0 group-hover/branch:opacity-100 group-focus-within/branch:opacity-100"
                                }
                                size="icon-sm"
                                type="button"
                                variant={isFiltered ? "secondary" : "ghost"}
                                onClick={() => toggleBranchFilter(row.branch)}
                              >
                                <Filter />
                              </Button>
                              <Button
                                aria-label={`${isHidden ? "Show" : "Hide"} ${row.branch.name} in graph`}
                                aria-pressed={isHidden}
                                className={
                                  isHidden
                                    ? "text-blue-600 opacity-100 dark:text-blue-300"
                                    : "opacity-0 group-hover/branch:opacity-100 group-focus-within/branch:opacity-100"
                                }
                                size="icon-sm"
                                type="button"
                                variant={isHidden ? "secondary" : "ghost"}
                                onClick={() => toggleBranchHidden(row.branch)}
                              >
                                <EyeOff />
                              </Button>
                            </div>
                          </div>
                        );
                      })()
                    ),
                  )}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </section>
  );

  const commitLog = (
    <section className="flex h-full min-h-0 flex-col">
      <header className="flex items-center justify-between gap-2 border-b px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <GitCommit className="size-4 text-muted-foreground" />
          <div className="min-w-0">
            <h2 className="truncate text-sm font-medium">Commit log</h2>
            <p className="truncate text-xs text-muted-foreground">
              {historyView === "graph" ? "Graph view" : "List view"}
            </p>
          </div>
        </div>
        <div className="flex rounded-md border p-0.5">
          <Button
            size="sm"
            variant={historyView === "list" ? "secondary" : "ghost"}
            onClick={() => setHistoryView("list")}
          >
            List
          </Button>
          <Button
            size="sm"
            variant={historyView === "graph" ? "secondary" : "ghost"}
            onClick={() => setHistoryView("graph")}
          >
            Graph
          </Button>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        {!selectedRepository ? (
          <p className="text-sm text-muted-foreground">Select a repository to view commits.</p>
        ) : historyView === "list" && historyQuery.isLoading ? (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading history
          </p>
        ) : historyView === "list" && historyQuery.isError ? (
          <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <span>{getErrorMessage(historyQuery.error)}</span>
          </p>
        ) : historyView === "graph" && graphQuery.isLoading ? (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading graph
          </p>
        ) : historyView === "graph" && graphQuery.isError ? (
          <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <span>{getErrorMessage(graphQuery.error)}</span>
          </p>
        ) : historyView === "list" && historyCommits.length === 0 ? (
          <p className="text-sm text-muted-foreground">No commits found.</p>
        ) : historyView === "graph" && graphData?.commits.length === 0 ? (
          <p className="text-sm text-muted-foreground">No commits found.</p>
        ) : historyView === "graph" && graphData ? (
          <HistoryGraphView
            graph={graphData}
            graphRefs={graphRefs}
            graphRows={graphRows}
            hasNextPage={graphQuery.hasNextPage}
            isFetchingNextPage={graphQuery.isFetchingNextPage}
            loadMoreRef={commitLogLoadMoreRef}
            maxGraphLane={maxGraphLane}
            onSelectCommit={setSelectedCommitHash}
            selectedCommitHash={selectedCommitHash}
          />
        ) : (
          <div className="grid gap-2">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-28">Hash</TableHead>
                  <TableHead>Message</TableHead>
                  <TableHead className="w-40">Author</TableHead>
                  <TableHead className="w-48">Date</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {historyCommits.map((commit) => {
                  const isSelected = commit.hash === selectedCommitHash;

                  return (
                    <TableRow
                      className="cursor-pointer data-[selected=true]:bg-muted"
                      data-selected={isSelected}
                      key={commit.hash}
                      onClick={() => setSelectedCommitHash(commit.hash)}
                    >
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        {getShortHash(commit.hash)}
                      </TableCell>
                      <TableCell className="max-w-0 truncate">{commit.message}</TableCell>
                      <TableCell className="max-w-0 truncate text-muted-foreground">
                        {commit.author}
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        {commit.date}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
            <div className="flex items-center justify-between gap-2 text-xs text-muted-foreground">
              <div ref={commitLogLoadMoreRef} className="h-px w-px shrink-0" aria-hidden />
              <span>
                {historyCommits.length} /{" "}
                {historyQuery.data?.pages[historyQuery.data.pages.length - 1]?.page.totalCount ??
                  historyCommits.length}{" "}
                commits loaded
                {historyQuery.isFetchingNextPage ? " · loading older commits" : ""}
              </span>
            </div>
          </div>
        )}
      </div>
    </section>
  );

  const commitDetail = (
    <section className="flex h-full min-h-0 flex-col">
      <header className="border-b px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <GitCommit className="size-4 text-muted-foreground" />
          <div className="min-w-0">
            <h2 className="truncate text-sm font-medium">Selected commit</h2>
            <p className="truncate text-xs text-muted-foreground">
              {selectedCommitHash ? getShortHash(selectedCommitHash) : "No commit selected"}
            </p>
          </div>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        {!selectedRepository || !selectedCommitHash ? (
          <div className="flex min-h-60 items-center justify-center">
            <div className="max-w-xs text-center">
              <GitCommit className="mx-auto size-8 text-muted-foreground" />
              <h3 className="mt-3 text-sm font-medium">No commit selected</h3>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                Select a commit from the log to inspect changed files and diff.
              </p>
            </div>
          </div>
        ) : commitDetailQuery.isLoading ? (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Loading commit detail
          </p>
        ) : commitDetailQuery.isError ? (
          <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
            <AlertCircle className="mt-0.5 size-4 shrink-0" />
            <span>{getErrorMessage(commitDetailQuery.error)}</span>
          </p>
        ) : commitDetailQuery.data ? (
          <div className="grid gap-4">
            <div className="grid gap-1">
              <p className="font-mono text-xs text-muted-foreground">
                {commitDetailQuery.data.hash}
              </p>
              <h3 className="break-words text-sm font-medium">{commitDetailQuery.data.message}</h3>
              <p className="text-sm text-muted-foreground">
                {commitDetailQuery.data.author} · {commitDetailQuery.data.date}
              </p>
            </div>
            <div className="grid gap-3">
              <div className="flex items-center justify-between gap-2">
                <h3 className="text-sm font-medium">Changed files</h3>
                <div className="flex rounded-md border p-0.5">
                  <Button
                    size="sm"
                    variant={fileChangeView === "tree" ? "secondary" : "ghost"}
                    onClick={() => setFileChangeView("tree")}
                  >
                    Tree
                  </Button>
                  <Button
                    size="sm"
                    variant={fileChangeView === "list" ? "secondary" : "ghost"}
                    onClick={() => setFileChangeView("list")}
                  >
                    List
                  </Button>
                </div>
              </div>
              {fileChangeView === "tree" ? (
                <div className="overflow-hidden rounded-md border text-sm">
                  {fileRows.map((row) =>
                    row.type === "folder" ? (
                      <button
                        aria-expanded={row.isExpanded}
                        className="flex h-8 w-full items-center gap-1 border-b px-2 text-left last:border-b-0 hover:bg-muted/50"
                        key={row.id}
                        onClick={() => toggleFileFolder(row.path)}
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
                        onClick={() => setSelectedFilePath(row.file.path)}
                        style={{ paddingLeft: `${28 + row.depth * 18}px` }}
                        title={row.file.path}
                        type="button"
                      >
                        <FileText className="size-4 shrink-0 text-muted-foreground" />
                        <span className="min-w-0 flex-1 truncate font-mono text-xs">
                          {row.name}
                        </span>
                        <span
                          className={`ml-auto shrink-0 rounded-sm border px-1.5 py-0.5 font-mono text-[10px] leading-none ${fileStatusClassName(row.file.status)}`}
                        >
                          {row.file.status}
                        </span>
                      </button>
                    ),
                  )}
                </div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead className="w-20">Status</TableHead>
                      <TableHead>File</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {commitDetailQuery.data.files.map((file) => {
                      const isSelected = file.path === selectedFilePath;

                      return (
                        <TableRow
                          className="cursor-pointer data-[selected=true]:bg-muted"
                          data-selected={isSelected}
                          key={`${file.status}:${file.path}`}
                          onClick={() => setSelectedFilePath(file.path)}
                        >
                          <TableCell className="font-mono text-xs">
                            <span
                              className={`rounded-sm border px-1.5 py-0.5 text-[10px] leading-none ${fileStatusClassName(file.status)}`}
                            >
                              {file.status}
                            </span>
                          </TableCell>
                          <TableCell className="max-w-0 truncate font-mono text-xs">
                            {file.path}
                          </TableCell>
                        </TableRow>
                      );
                    })}
                  </TableBody>
                </Table>
              )}
              {!selectedFilePath ? (
                <p className="text-sm text-muted-foreground">
                  Select a changed file to inspect its diff.
                </p>
              ) : fileDiffQuery.isLoading ? (
                <p className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="size-4 animate-spin" />
                  Loading diff
                </p>
              ) : fileDiffQuery.isError ? (
                <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
                  <AlertCircle className="mt-0.5 size-4 shrink-0" />
                  <span>{getErrorMessage(fileDiffQuery.error)}</span>
                </p>
              ) : fileDiffQuery.data?.isBinary ? (
                <p className="text-sm text-muted-foreground">
                  This file is binary and cannot be displayed as text diff.
                </p>
              ) : fileDiffQuery.data ? (
                <div className="grid gap-2">
                  {fileDiffQuery.data.isTruncated ? (
                    <p className="text-xs text-muted-foreground">Large diff truncated for display.</p>
                  ) : null}
                  <DiffViewer content={fileDiffQuery.data.content} />
                </div>
              ) : null}
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );

  return (
    <ResizablePanelGroup defaultLayout={loadColumnLayout()} onLayoutChanged={saveColumnLayout}>
      <ResizablePanel id="repository-info" defaultSize="28%" minSize="240px" maxSize="38%">
        {repositoryInfo}
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel id="commit-log" defaultSize="42%" minSize="340px">
        {commitLog}
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel id="commit-detail" defaultSize="30%" minSize="320px" maxSize="45%">
        {commitDetail}
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
