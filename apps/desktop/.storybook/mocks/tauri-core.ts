import {
  sampleAgents,
  sampleBranches,
  sampleProjects,
  sampleRemotes,
  sampleWorktrees,
} from "../../src/shared/storybook/sample-data";

export async function invoke<T>(command: string, args?: Record<string, unknown>) {
  switch (command) {
    case "list_projects":
      return sampleProjects as T;
    case "create_project":
    case "update_project":
      return {
        id: "storybook-project",
        ...(args?.input as Record<string, unknown> | undefined),
      } as T;
    case "delete_project":
    case "create_git_worktree":
    case "delete_git_worktree":
    case "cancel_agent_run":
      return undefined as T;
    case "list_git_remotes":
      return sampleRemotes as T;
    case "list_git_branches":
      return sampleBranches as T;
    case "list_git_worktrees":
      return sampleWorktrees as T;
    case "list_agents":
      return sampleAgents as T;
    case "start_agent_run":
      return {
        id: String((args?.request as { runId?: string } | undefined)?.runId ?? "run-storybook"),
        goal: String((args?.request as { goal?: string } | undefined)?.goal ?? ""),
        agentId: String((args?.request as { agentId?: string } | undefined)?.agentId ?? "codex"),
      } as T;
    default:
      throw new Error(`Unhandled Storybook Tauri command: ${command}`);
  }
}
