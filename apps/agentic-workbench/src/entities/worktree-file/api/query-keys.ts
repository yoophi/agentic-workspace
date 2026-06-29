export const worktreeFileQueryKeys = {
  all: ["worktree-files"] as const,
  list: (workingDirectory: string) =>
    ["worktree-files", "list", workingDirectory] as const,
  textFile: (workingDirectory: string, path: string) =>
    ["worktree-files", "text-file", workingDirectory, path] as const,
};
