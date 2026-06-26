import { invoke } from "@tauri-apps/api/core";

export type AppInfo = {
  name: string;
  version: string;
};

export type Repository = {
  id: string;
  name: string;
  path: string;
};

export type GitWorktree = {
  path: string;
  branch?: string | null;
  commit: string;
  isBare: boolean;
  isMain: boolean;
};

export type GitBranch = {
  name: string;
  fullName: string;
  isRemote: boolean;
  isCurrent: boolean;
  worktreePath?: string | null;
};

export type GitCommitSummary = {
  hash: string;
  message: string;
  author: string;
  date: string;
};

export type GitCommitHistory = {
  commits: GitCommitSummary[];
  page: GitCommitPage;
};

export type GitCommitPage = {
  offset: number;
  limit: number;
  totalCount: number;
  hasMore: boolean;
};

export type GitCommitGraph = {
  commits: GitGraphCommit[];
  refs: GitGraphRef[];
  page: GitGraphPage;
  layoutHints: GitGraphLayoutHints;
};

export type GitGraphCommit = {
  hash: string;
  shortHash: string;
  parents: string[];
  message: string;
  author: string;
  date: string;
  isHead: boolean;
  isMerge: boolean;
};

export type GitGraphPage = {
  offset: number;
  limit: number;
  totalCount: number;
  hasMore: boolean;
};

export type GitGraphLayoutHints = {
  rowHeight: number;
  maxInitialLanes: number;
};

export type GitGraphRef = {
  name: string;
  target: string;
  kind: "localBranch" | "remoteBranch" | "tag";
};

export type GitCommitFileChange = {
  path: string;
  status: string;
};

export type GitCommitDetail = GitCommitSummary & {
  files: GitCommitFileChange[];
};

export type GitFileDiff = {
  commitHash: string;
  path: string;
  content: string;
  isBinary: boolean;
  isTruncated: boolean;
};

export type GitCommitQueryOptions = {
  maxCount?: number;
  offset?: number;
  includedRefs?: string[];
  excludedRefs?: string[];
};

export const repositoryKeys = {
  all: ["repositories"] as const,
  worktrees: (repositoryId: string) => ["repositories", repositoryId, "worktrees"] as const,
  branches: (repositoryId: string) => ["repositories", repositoryId, "branches"] as const,
  historyRoot: (repositoryId: string) => ["repositories", repositoryId, "history"] as const,
  history: (repositoryId: string, options?: GitCommitQueryOptions) =>
    [
      "repositories",
      repositoryId,
      "history",
      options?.maxCount ?? 100,
      options?.includedRefs ?? [],
      options?.excludedRefs ?? [],
    ] as const,
  commitGraphRoot: (repositoryId: string) => ["repositories", repositoryId, "commitGraph"] as const,
  commitGraph: (repositoryId: string, options?: GitCommitQueryOptions) =>
    [
      "repositories",
      repositoryId,
      "commitGraph",
      options?.maxCount ?? 300,
      options?.includedRefs ?? [],
      options?.excludedRefs ?? [],
    ] as const,
  commitDetail: (repositoryId: string, commitHash: string) =>
    ["repositories", repositoryId, "commits", commitHash] as const,
  fileDiff: (repositoryId: string, commitHash: string, filePath: string) =>
    ["repositories", repositoryId, "commits", commitHash, "files", filePath, "diff"] as const,
};

export function getAppInfo() {
  return invoke<AppInfo>("app_info");
}

export function listRepositories() {
  return invoke<Repository[]>("list_repositories");
}

export function createRepository(path: string) {
  return invoke<Repository>("create_repository", {
    request: {
      path,
    },
  });
}

export function renameRepository(repositoryId: string, name: string) {
  return invoke<Repository>("rename_repository", {
    request: {
      repositoryId,
      name,
    },
  });
}

export function deleteRepository(repositoryId: string) {
  return invoke<void>("delete_repository", {
    request: {
      repositoryId,
    },
  });
}

export function listWorktrees(repositoryId: string) {
  return invoke<GitWorktree[]>("list_worktrees", {
    request: {
      repositoryId,
    },
  });
}

export function listBranches(repositoryId: string) {
  return invoke<GitBranch[]>("list_branches", {
    request: {
      repositoryId,
    },
  });
}

export function listHistory(repositoryId: string, options?: GitCommitQueryOptions) {
  return invoke<GitCommitHistory>("list_history", {
    request: {
      repositoryId,
      maxCount: options?.maxCount,
      offset: options?.offset,
      includedRefs: options?.includedRefs,
      excludedRefs: options?.excludedRefs,
    },
  });
}

export function getCommitGraph(repositoryId: string, options?: GitCommitQueryOptions) {
  return invoke<GitCommitGraph>("get_commit_graph", {
    request: {
      repositoryId,
      maxCount: options?.maxCount,
      offset: options?.offset,
      includedRefs: options?.includedRefs,
      excludedRefs: options?.excludedRefs,
    },
  });
}

export function getCommitDetail(repositoryId: string, commitHash: string) {
  return invoke<GitCommitDetail>("get_commit_detail", {
    request: {
      repositoryId,
      commitHash,
    },
  });
}

export function getFileDiff(repositoryId: string, commitHash: string, filePath: string) {
  return invoke<GitFileDiff>("get_file_diff", {
    request: {
      repositoryId,
      commitHash,
      filePath,
    },
  });
}

export function startRepositoryWatchers() {
  return invoke<void>("start_repository_watchers");
}

export function stopRepositoryWatchers() {
  return invoke<void>("stop_repository_watchers");
}
