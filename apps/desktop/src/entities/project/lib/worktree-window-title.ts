export function formatWorktreeWindowTitle(projectName: string, worktreePath: string) {
  return `${fallbackProjectName(projectName)} / ${worktreeNameFromPath(worktreePath)}`;
}

export function worktreeNameFromPath(worktreePath: string) {
  const normalized = worktreePath.trim().replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "worktree";
}

function fallbackProjectName(projectName: string) {
  return projectName.trim() || "Project";
}
