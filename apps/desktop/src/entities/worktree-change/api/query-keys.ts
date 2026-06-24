export const worktreeChangeQueryKeys = {
  changes: (workingDirectory: string) => ["worktree-changes", workingDirectory] as const,
};
