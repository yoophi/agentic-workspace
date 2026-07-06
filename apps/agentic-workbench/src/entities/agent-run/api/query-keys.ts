export const agentRunQueryKeys = {
  agents: ["agents"] as const,
  goal: (workingDirectory: string) => ["goal", workingDirectory] as const,
  settings: (workingDirectory: string) =>
    ["agent-run-settings", workingDirectory] as const,
  sessions: (agentId: string, cwd: string) =>
    ["provider-sessions", agentId, cwd] as const,
  toolCommandCandidates: (
    runId: string | null | undefined,
    agentId: string,
    workingDirectory: string,
    sessionMode: string,
  ) =>
    [
      "agent-tool-command-candidates",
      runId ?? "no-run",
      agentId,
      workingDirectory,
      sessionMode,
    ] as const,
  worktreeChanges: (workingDirectory: string) =>
    ["worktree-changes", workingDirectory] as const,
};
