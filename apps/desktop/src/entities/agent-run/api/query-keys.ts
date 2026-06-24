export const agentRunQueryKeys = {
  agents: ["agents"] as const,
  goal: (workingDirectory: string) => ["goal", workingDirectory] as const,
  sessions: (agentId: string, cwd: string) =>
    ["provider-sessions", agentId, cwd] as const,
};
