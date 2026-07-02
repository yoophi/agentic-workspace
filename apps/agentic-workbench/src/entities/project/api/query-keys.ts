export const projectQueryKeys = {
  all: ["projects"] as const,
  gitRemotes: (workingDirectory: string) =>
    ["projects", "git-remotes", workingDirectory] as const,
  gitBranches: (workingDirectory: string) =>
    ["projects", "git-branches", workingDirectory] as const,
  gitWorktrees: (workingDirectory: string) =>
    ["projects", "git-worktrees", workingDirectory] as const,
  // status 계산을 생략한 목록(includeStatus: false). 프로젝트 상세의 status 포함
  // 목록과 데이터 형태가 달라 캐시를 분리한다(specs/007 research R5).
  gitWorktreeRefs: (workingDirectory: string) =>
    ["projects", "git-worktree-refs", workingDirectory] as const,
  worktreeChanges: (workingDirectory: string) =>
    ["projects", "worktree-changes", workingDirectory] as const,
  worktreeFileDiff: (workingDirectory: string, path: string) =>
    ["projects", "worktree-file-diff", workingDirectory, path] as const,
};
