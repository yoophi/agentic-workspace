export const agentOrchestrationQueryKeys = {
  workspace: (worktreePath: string) =>
    ["agent-orchestration", "workspace", worktreePath] as const,
  runtimeEvents: (runId: string) =>
    ["agent-orchestration", "runtime-events", runId] as const,
};
