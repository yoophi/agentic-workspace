export type GitChangedFileGroup = "staged" | "unstaged" | "untracked" | "conflicted";

export type GitWorktreeChanges = {
  workingDirectory: string;
  files: GitChangedFile[];
  stagedCount: number;
  unstagedCount: number;
  untrackedCount: number;
  conflictedCount: number;
};

export type GitChangedFile = {
  path: string;
  oldPath?: string | null;
  stagedStatus?: string | null;
  unstagedStatus?: string | null;
  group: GitChangedFileGroup;
};

export type GitFileDiff = {
  path: string;
  diff: string;
  truncated: boolean;
  binary: boolean;
};
