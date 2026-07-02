import { useEffect, useMemo, useState } from "react";
import type { Layout } from "react-resizable-panels";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import {
  AlertCircle,
  ChevronDown,
  ChevronRight,
  EyeOff,
  FileDiff,
  Filter,
  Folder,
  FolderGit2,
  FolderOpen,
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
  getWorktreeFileDiff,
  getWorktreeStatus,
  listBranches,
  listHistory,
  listWorktrees,
  repositoryKeys,
  type GitBranch,
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
} from "@/features/history-tree/model/graph-layout";
import {
  CommitDetailView,
  HistoryGraphView,
  InfiniteLoadSentinel,
  WorktreeChangesView,
  combineGitCommitGraphPages,
  refsByTarget,
} from "@yoophi/git-ui";
import {
  autoRefreshQueryOptions,
  findStaleCommitSelection,
  type StaleSelection,
} from "@yoophi/workspace-auto-refresh";

type ChangesPanelProps = {
  selectedRepository?: Repository;
};

type HistoryView = "list" | "graph";
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

export function ChangesPanel({ selectedRepository }: ChangesPanelProps) {
  const [selectedCommitHash, setSelectedCommitHash] = useState<string>();
  const [selectedFilePath, setSelectedFilePath] = useState<string>();
  const [historyView, setHistoryView] = useState<HistoryView>("list");
  const [expandedBranchFolders, setExpandedBranchFolders] = useState<Set<string>>(new Set());
  const [filteredBranchKeys, setFilteredBranchKeys] = useState<Set<string>>(new Set());
  const [hiddenBranchKeys, setHiddenBranchKeys] = useState<Set<string>>(new Set());
  const [staleCommitSelection, setStaleCommitSelection] = useState<StaleSelection | null>(null);
  const [viewMode, setViewMode] = useState<"commit" | "worktree">("commit");
  const [worktreeFilePath, setWorktreeFilePath] = useState<string>();

  function selectCommit(commitHash: string) {
    setViewMode("commit");
    setSelectedCommitHash(commitHash);
    setStaleCommitSelection(null);
  }
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
    ...autoRefreshQueryOptions,
  });
  const branchesQuery = useQuery({
    enabled: Boolean(selectedRepository),
    queryKey: selectedRepository
      ? repositoryKeys.branches(selectedRepository.id)
      : ["repositories", "unselected", "branches"],
    queryFn: () => listBranches(selectedRepository?.id ?? ""),
    ...autoRefreshQueryOptions,
  });
  const branchRefs = useMemo(
    () => branchHistoryRefs(branchesQuery.data ?? [], filteredBranchKeys, hiddenBranchKeys),
    [branchesQuery.data, filteredBranchKeys, hiddenBranchKeys],
  );
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
    ...autoRefreshQueryOptions,
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
    ...autoRefreshQueryOptions,
  });
  const commitDetailQuery = useQuery({
    enabled: Boolean(selectedRepository && selectedCommitHash),
    queryKey:
      selectedRepository && selectedCommitHash
        ? repositoryKeys.commitDetail(selectedRepository.id, selectedCommitHash)
        : ["repositories", "unselected", "commits", "unselected"],
    queryFn: () => getCommitDetail(selectedRepository?.id ?? "", selectedCommitHash ?? ""),
    ...autoRefreshQueryOptions,
  });
  const fileDiffQuery = useQuery({
    enabled: Boolean(selectedRepository && selectedCommitHash && selectedFilePath),
    queryKey:
      selectedRepository && selectedCommitHash && selectedFilePath
        ? repositoryKeys.fileDiff(selectedRepository.id, selectedCommitHash, selectedFilePath)
        : ["repositories", "unselected", "commits", "unselected", "files", "unselected", "diff"],
    queryFn: () =>
      getFileDiff(selectedRepository?.id ?? "", selectedCommitHash ?? "", selectedFilePath ?? ""),
    ...autoRefreshQueryOptions,
  });
  const worktreeStatusQuery = useQuery({
    enabled: Boolean(selectedRepository),
    queryKey: selectedRepository
      ? repositoryKeys.worktreeStatus(selectedRepository.id)
      : ["repositories", "unselected", "worktreeStatus"],
    queryFn: () => getWorktreeStatus(selectedRepository?.id ?? ""),
  });
  const worktreeDiffQuery = useQuery({
    enabled: Boolean(selectedRepository && viewMode === "worktree" && worktreeFilePath),
    queryKey:
      selectedRepository && worktreeFilePath
        ? repositoryKeys.worktreeFileDiff(selectedRepository.id, worktreeFilePath)
        : ["repositories", "unselected", "worktreeFileDiff"],
    queryFn: () => getWorktreeFileDiff(selectedRepository?.id ?? "", worktreeFilePath ?? ""),
  });
  const branchRows = useMemo(
    () => buildBranchTreeRows(branchesQuery.data ?? [], expandedBranchFolders),
    [branchesQuery.data, expandedBranchFolders],
  );
  const historyCommits = useMemo(
    () => historyQuery.data?.pages.flatMap((page) => page.commits) ?? [],
    [historyQuery.data?.pages],
  );
  const rawGraphData = useMemo(
    () => combineGitCommitGraphPages(graphQuery.data?.pages ?? []),
    [graphQuery.data?.pages],
  );
  const graphData = useMemo(
    () =>
      rawGraphData
        ? filterGraphByBranchControls(rawGraphData, filteredBranchKeys, hiddenBranchKeys)
        : undefined,
    [rawGraphData, filteredBranchKeys, hiddenBranchKeys],
  );
  const graphRows = useMemo(
    () => (graphData ? computeGitGraphRows(graphData.commits) : new Map<string, GitGraphRow>()),
    [graphData],
  );
  const maxGraphLane = useMemo(() => getMaxGraphLane(graphRows), [graphRows]);
  const graphRefs = useMemo(
    () => (graphData ? refsByTarget(graphData.refs) : new Map<string, GitGraphRef[]>()),
    [graphData],
  );
  const isRefreshing =
    worktreesQuery.isFetching ||
    branchesQuery.isFetching ||
    historyQuery.isFetching ||
    graphQuery.isFetching ||
    commitDetailQuery.isFetching ||
    fileDiffQuery.isFetching ||
    worktreeStatusQuery.isFetching ||
    worktreeDiffQuery.isFetching;

  useEffect(() => {
    setSelectedCommitHash(undefined);
    setSelectedFilePath(undefined);
    setStaleCommitSelection(null);
    setFilteredBranchKeys(new Set());
    setHiddenBranchKeys(new Set());
    setViewMode("commit");
    setWorktreeFilePath(undefined);
  }, [selectedRepository?.id]);

  useEffect(() => {
    setSelectedFilePath(undefined);
  }, [selectedCommitHash]);

  useEffect(() => {
    setExpandedBranchFolders(getBranchFolderPaths(branchesQuery.data ?? []));
  }, [branchesQuery.data]);

  useEffect(() => {
    if (!selectedCommitHash) {
      setStaleCommitSelection(null);
      return;
    }

    const availableCommitHashes = new Set<string>();
    for (const commit of historyCommits) {
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
      setSelectedFilePath(undefined);
    }
  }, [graphData?.commits, historyCommits, selectedCommitHash]);

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
        <div className="flex shrink-0 items-center gap-2">
          {isRefreshing ? (
            <span className="rounded-sm border px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground">
              Refreshing
            </span>
          ) : null}
          {staleCommitSelection ? (
            <span className="rounded-sm border border-red-300 px-1.5 py-0.5 text-[10px] leading-none text-red-600">
              Stale commit
            </span>
          ) : null}
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
        </div>
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
        <div className="flex items-center gap-2">
          {(historyQuery.isFetching || graphQuery.isFetching) &&
          !historyQuery.isLoading &&
          !graphQuery.isLoading ? (
            <span className="rounded-sm border px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground">
              Refreshing
            </span>
          ) : null}
          {staleCommitSelection ? (
            <span className="rounded-sm border border-red-300 px-1.5 py-0.5 text-[10px] leading-none text-red-600">
              Stale
            </span>
          ) : null}
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
            onLoadMore={() => void graphQuery.fetchNextPage()}
            maxGraphLane={maxGraphLane}
            onSelectCommit={selectCommit}
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
                      onClick={() => selectCommit(commit.hash)}
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
              <InfiniteLoadSentinel
                hasNextPage={historyQuery.hasNextPage}
                isFetchingNextPage={historyQuery.isFetchingNextPage}
                onLoadMore={() => void historyQuery.fetchNextPage()}
              />
              <span>
                {historyCommits.length} /{" "}
                {historyQuery.data?.pages[0]?.page.totalCount ?? historyCommits.length}{" "}
                commits loaded
                {historyQuery.isFetchingNextPage ? " · loading older commits" : ""}
              </span>
            </div>
          </div>
        )}
      </div>
    </section>
  );

  const worktreeChangeCount = worktreeStatusQuery.data?.files.length ?? 0;

  const commitDetail = (
    <section className="flex h-full min-h-0 flex-col">
      <header className="border-b px-4 py-3">
        <div className="flex items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2">
            {viewMode === "commit" ? (
              <GitCommit className="size-4 text-muted-foreground" />
            ) : (
              <FileDiff className="size-4 text-muted-foreground" />
            )}
            <div className="min-w-0">
              <h2 className="truncate text-sm font-medium">
                {viewMode === "commit" ? "Selected commit" : "Working tree"}
              </h2>
              <p className="truncate text-xs text-muted-foreground">
                {viewMode === "commit"
                  ? selectedCommitHash
                    ? getShortHash(selectedCommitHash)
                    : "No commit selected"
                  : "Uncommitted changes"}
              </p>
            </div>
          </div>
          <div className="flex shrink-0 rounded-md border p-0.5">
            <Button
              size="sm"
              variant={viewMode === "commit" ? "secondary" : "ghost"}
              onClick={() => setViewMode("commit")}
            >
              Commit
            </Button>
            <Button
              size="sm"
              variant={viewMode === "worktree" ? "secondary" : "ghost"}
              onClick={() => setViewMode("worktree")}
            >
              Working tree{worktreeChangeCount > 0 ? ` (${worktreeChangeCount})` : ""}
            </Button>
          </div>
        </div>
      </header>
      <div className="min-h-0 flex-1 overflow-auto p-4">
        {!selectedRepository ? (
          <div className="flex min-h-60 items-center justify-center">
            <div className="max-w-xs text-center">
              <GitCommit className="mx-auto size-8 text-muted-foreground" />
              <h3 className="mt-3 text-sm font-medium">No repository selected</h3>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                Select a repository to inspect commits and working-tree changes.
              </p>
            </div>
          </div>
        ) : viewMode === "worktree" ? (
          worktreeStatusQuery.isLoading ? (
            <p className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              Loading changes
            </p>
          ) : worktreeStatusQuery.isError ? (
            <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
              <AlertCircle className="mt-0.5 size-4 shrink-0" />
              <span>{getErrorMessage(worktreeStatusQuery.error)}</span>
            </p>
          ) : (
            <WorktreeChangesView
              changes={worktreeStatusQuery.data}
              selectedFilePath={worktreeFilePath}
              onSelectFile={setWorktreeFilePath}
              diff={worktreeDiffQuery.data}
              diffLoading={worktreeDiffQuery.isLoading}
              diffError={
                worktreeDiffQuery.isError ? getErrorMessage(worktreeDiffQuery.error) : undefined
              }
            />
          )
        ) : !selectedCommitHash ? (
          <div className="flex min-h-60 items-center justify-center">
            <div className="max-w-xs text-center">
              <GitCommit className="mx-auto size-8 text-muted-foreground" />
              <h3 className="mt-3 text-sm font-medium">No commit selected</h3>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                Select a commit from the log to inspect changed files and diff.
              </p>
            </div>
          </div>
        ) : staleCommitSelection ? (
          <div className="flex min-h-60 items-center justify-center">
            <div className="max-w-xs text-center">
              <GitCommit className="mx-auto size-8 text-muted-foreground" />
              <h3 className="mt-3 text-sm font-medium">Commit no longer in current history</h3>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                {staleCommitSelection.id.slice(0, 8)} may have disappeared after a branch change,
                rebase, or reset. Select another commit from the refreshed log.
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
          <CommitDetailView
            commit={commitDetailQuery.data}
            files={commitDetailQuery.data.files}
            selectedFilePath={selectedFilePath}
            onSelectFile={setSelectedFilePath}
            diff={fileDiffQuery.data}
            diffLoading={fileDiffQuery.isLoading}
            diffError={fileDiffQuery.isError ? getErrorMessage(fileDiffQuery.error) : undefined}
          />
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
