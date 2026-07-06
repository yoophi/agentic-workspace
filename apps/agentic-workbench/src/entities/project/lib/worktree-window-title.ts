export const AGENT_WINDOW_TITLE_MAX_LENGTH = 80;

export function formatWorktreeWindowTitle(projectName: string, worktreePath: string) {
  return `${fallbackProjectName(projectName)} / ${worktreeNameFromPath(worktreePath)}`;
}

export function normalizeAgentWindowTitle(title: string) {
  const normalized = title.trim();
  if (normalized.length === 0) {
    return null;
  }
  if (hasControlCharacter(normalized)) {
    return null;
  }
  if ([...normalized].length > AGENT_WINDOW_TITLE_MAX_LENGTH) {
    return null;
  }
  return normalized;
}

export function resolveWorktreeWindowTitle(defaultTitle: string, agentTitle: string | null) {
  return agentTitle ?? defaultTitle;
}

export function worktreeNameFromPath(worktreePath: string) {
  const normalized = worktreePath.trim().replace(/\\/g, "/");
  const parts = normalized.split("/").filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : "worktree";
}

function fallbackProjectName(projectName: string) {
  return projectName.trim() || "Project";
}

function hasControlCharacter(value: string) {
  return /[\u0000-\u001F\u007F]/.test(value);
}
