export const agentRunQueryKeys = {
  agents: ["agents"] as const,
  sessions: (agentId: string, cwd: string) =>
    ["provider-sessions", agentId, cwd] as const,
};
