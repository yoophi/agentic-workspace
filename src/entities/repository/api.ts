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

export const repositoryKeys = {
  all: ["repositories"] as const,
  worktrees: (repositoryId: string) => ["repositories", repositoryId, "worktrees"] as const,
  branches: (repositoryId: string) => ["repositories", repositoryId, "branches"] as const,
  history: (repositoryId: string) => ["repositories", repositoryId, "history"] as const,
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

export function listHistory(repositoryId: string) {
  return invoke<GitCommitSummary[]>("list_history", {
    request: {
      repositoryId,
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
