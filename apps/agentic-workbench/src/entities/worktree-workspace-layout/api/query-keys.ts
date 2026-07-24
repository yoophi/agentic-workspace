export const worktreeWorkspaceLayoutQueryKeys = {
  layout: (workingDirectory: string) => ["worktree-workspace-layout", workingDirectory] as const,
};
