import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  AlertCircle,
  FolderGit2,
  GitBranch as GitBranchIcon,
  GitCommit,
  Loader2,
  RefreshCw,
} from "lucide-react";
import { Button } from "@yoophi/ui/components/button";
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
  getFileDiff,
  listBranches,
  listHistory,
  listWorktrees,
  repositoryKeys,
  type GitBranch,
  type GitWorktree,
  type Repository,
} from "@/entities/repository";

type ChangesPanelProps = {
  selectedRepository?: Repository;
};

type BranchTreeRow =
  | {
      id: string;
      depth: number;
      name: string;
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

function buildBranchTreeRows(branches: GitBranch[]): BranchTreeRow[] {
  const rows: BranchTreeRow[] = [];
  const folders = new Set<string>();

  for (const branch of [...branches].sort((a, b) => a.name.localeCompare(b.name))) {
    const segments = branch.name.split("/").filter(Boolean);
    let folderPath = "";

    for (const [index, segment] of segments.slice(0, -1).entries()) {
      folderPath = folderPath ? `${folderPath}/${segment}` : segment;

      if (!folders.has(folderPath)) {
        folders.add(folderPath);
        rows.push({
          id: `folder:${folderPath}`,
          depth: index,
          name: segment,
          type: "folder",
        });
      }
    }

    rows.push({
      branch,
      depth: Math.max(segments.length - 1, 0),
      id: branch.fullName,
      name: segments[segments.length - 1] ?? branch.name,
      type: "branch",
    });
  }

  return rows;
}

export function ChangesPanel({ selectedRepository }: ChangesPanelProps) {
  const [selectedCommitHash, setSelectedCommitHash] = useState<string>();
  const [selectedFilePath, setSelectedFilePath] = useState<string>();
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
  const historyQuery = useQuery({
    enabled: Boolean(selectedRepository),
    queryKey: selectedRepository
      ? repositoryKeys.history(selectedRepository.id)
      : ["repositories", "unselected", "history"],
    queryFn: () => listHistory(selectedRepository?.id ?? ""),
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
  const branchRows = buildBranchTreeRows(branchesQuery.data ?? []);
  const isRefreshing =
    worktreesQuery.isFetching ||
    branchesQuery.isFetching ||
    historyQuery.isFetching ||
    commitDetailQuery.isFetching ||
    fileDiffQuery.isFetching;

  useEffect(() => {
    setSelectedCommitHash(undefined);
    setSelectedFilePath(undefined);
  }, [selectedRepository?.id]);

  useEffect(() => {
    setSelectedFilePath(undefined);
  }, [selectedCommitHash]);

  return (
    <section className="flex h-full min-h-0 flex-col">
      <header className="flex items-center justify-between border-b px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <GitCommit className="size-4 text-muted-foreground" />
          <div className="min-w-0">
            <h2 className="truncate text-sm font-medium">
              {selectedRepository?.name ?? "Working Tree"}
            </h2>
            <p className="truncate text-xs text-muted-foreground">
              {selectedRepository?.path ??
                (appInfo.data
                  ? `${appInfo.data.name} ${appInfo.data.version}`
                  : "Loading app info")}
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
      <div className="min-h-0 flex-1 overflow-auto">
        {!selectedRepository ? (
          <div className="flex h-full min-h-80 items-center justify-center p-4">
            <div className="max-w-sm text-center">
              <FolderGit2 className="mx-auto size-10 text-muted-foreground" />
              <h2 className="mt-3 text-sm font-medium">No repository selected</h2>
              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                Add a local Git repository from the sidebar, then select it to inspect its state.
              </p>
            </div>
          </div>
        ) : (
          <div className="grid gap-6 p-4">
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
                      <TableHead>Branch</TableHead>
                      <TableHead className="w-28">Commit</TableHead>
                      <TableHead className="w-24">Kind</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {worktreesQuery.data?.map((worktree) => (
                      <TableRow key={worktree.path}>
                        <TableCell className="max-w-0 truncate font-mono text-xs">
                          {worktree.path}
                        </TableCell>
                        <TableCell className="max-w-0 truncate">
                          {worktree.branch ?? "Detached"}
                        </TableCell>
                        <TableCell className="font-mono text-xs text-muted-foreground">
                          {getShortCommit(worktree.commit)}
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
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Name</TableHead>
                      <TableHead className="w-24">Scope</TableHead>
                      <TableHead className="w-24">State</TableHead>
                      <TableHead>Worktree</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {branchRows.map((row) => (
                      <TableRow key={row.id}>
                        <TableCell
                          className="max-w-0 truncate"
                          style={{ paddingLeft: `${12 + row.depth * 18}px` }}
                        >
                          {row.type === "folder" ? (
                            <span className="text-muted-foreground">{row.name}</span>
                          ) : (
                            row.name
                          )}
                        </TableCell>
                        <TableCell>
                          {row.type === "branch" ? (row.branch.isRemote ? "Remote" : "Local") : ""}
                        </TableCell>
                        <TableCell>
                          {row.type === "branch" && row.branch.isCurrent ? "Current" : ""}
                        </TableCell>
                        <TableCell className="max-w-0 truncate font-mono text-xs text-muted-foreground">
                          {row.type === "branch" ? (row.branch.worktreePath ?? "") : ""}
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              )}
            </div>
            <div>
              <div className="mb-2 flex items-center gap-2">
                <GitCommit className="size-4 text-muted-foreground" />
                <h3 className="text-sm font-medium">History</h3>
              </div>
              {historyQuery.isLoading ? (
                <p className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="size-4 animate-spin" />
                  Loading history
                </p>
              ) : historyQuery.isError ? (
                <p className="flex items-start gap-1.5 text-sm leading-5 text-red-600">
                  <AlertCircle className="mt-0.5 size-4 shrink-0" />
                  <span>{getErrorMessage(historyQuery.error)}</span>
                </p>
              ) : historyQuery.data?.length === 0 ? (
                <p className="text-sm text-muted-foreground">No commits found.</p>
              ) : (
                <div className="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(360px,0.7fr)]">
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead className="w-28">Hash</TableHead>
                        <TableHead>Message</TableHead>
                        <TableHead className="w-48">Author</TableHead>
                        <TableHead className="w-52">Date</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {historyQuery.data?.map((commit) => {
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
                  <div className="min-w-0 border-l pl-4">
                    {!selectedCommitHash ? (
                      <div className="flex min-h-60 items-center justify-center">
                        <div className="max-w-xs text-center">
                          <GitCommit className="mx-auto size-8 text-muted-foreground" />
                          <h3 className="mt-3 text-sm font-medium">No commit selected</h3>
                          <p className="mt-1 text-sm leading-6 text-muted-foreground">
                            Select a commit from the history list to inspect its files.
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
                          <h3 className="break-words text-sm font-medium">
                            {commitDetailQuery.data.message}
                          </h3>
                          <p className="text-sm text-muted-foreground">
                            {commitDetailQuery.data.author} · {commitDetailQuery.data.date}
                          </p>
                        </div>
                        <div className="grid gap-3">
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
                                      {file.status}
                                    </TableCell>
                                    <TableCell className="max-w-0 truncate font-mono text-xs">
                                      {file.path}
                                    </TableCell>
                                  </TableRow>
                                );
                              })}
                            </TableBody>
                          </Table>
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
                                <p className="text-xs text-muted-foreground">
                                  Large diff truncated for display.
                                </p>
                              ) : null}
                              <pre className="max-h-96 overflow-auto rounded-md border bg-muted/40 p-3 font-mono text-xs leading-5">
                                {fileDiffQuery.data.content || "No text diff available."}
                              </pre>
                            </div>
                          ) : null}
                        </div>
                      </div>
                    ) : null}
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </section>
  );
}
