const WORKTREE_PATH_PARAM = "worktreePath";

export function buildProjectWorktreeRoute(projectId: string, worktreePath: string) {
  const params = new URLSearchParams({ [WORKTREE_PATH_PARAM]: worktreePath });
  return `/projects/${projectId}/worktrees?${params.toString()}`;
}

export function readWorktreePath(searchParams: URLSearchParams) {
  return searchParams.get(WORKTREE_PATH_PARAM) ?? "";
}
