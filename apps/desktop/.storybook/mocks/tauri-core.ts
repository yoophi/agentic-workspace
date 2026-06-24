import {
  sampleAgents,
  sampleBranches,
  sampleGoal,
  sampleProjects,
  sampleRemotes,
  sampleSavedPrompts,
  sampleWorktreeChanges,
  sampleWorktrees,
} from "../../src/shared/storybook/sample-data";
import type { SavedPrompt, SavedPromptInput } from "../../src/entities/saved-prompt/model/types";
import type {
  GoalInput,
  GoalProgressInput,
  GoalUpdateInput,
  ThreadGoal,
} from "../../src/entities/agent-run/model/types";
import { emitMockEvent } from "./tauri-event";

const AGENT_RUN_EVENT = "agent-run-event";
let storybookSavedPrompts: SavedPrompt[] = [...sampleSavedPrompts];
let storybookGoal: ThreadGoal | null = sampleGoal;

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
      return undefined as T;
    case "delete_saved_prompt":
      storybookSavedPrompts = storybookSavedPrompts.filter(
        (savedPrompt) => savedPrompt.id !== args?.id,
      );
      return undefined as T;
    case "list_saved_prompts":
      return storybookSavedPrompts as T;
    case "get_goal":
      return storybookGoal as T;
    case "create_goal": {
      const input = args?.input as GoalInput | undefined;
      storybookGoal = {
        workingDirectory: input?.workingDirectory ?? sampleGoal.workingDirectory,
        objective: input?.objective ?? "",
        status: "active",
        tokenBudget: input?.tokenBudget ?? null,
        tokensUsed: 0,
        timeUsedSeconds: 0,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
      };
      return storybookGoal as T;
    }
    case "update_goal": {
      const input = args?.input as GoalUpdateInput | undefined;
      if (storybookGoal) {
        storybookGoal = {
          ...storybookGoal,
          ...(input?.objective !== undefined ? { objective: input.objective } : {}),
          ...(input?.status !== undefined ? { status: input.status } : {}),
          ...(input?.tokenBudget !== undefined ? { tokenBudget: input.tokenBudget } : {}),
          updatedAt: new Date().toISOString(),
        };
      }
      return storybookGoal as T;
    }
    case "clear_goal":
      storybookGoal = null;
      return undefined as T;
    case "record_goal_progress": {
      const input = args?.input as GoalProgressInput | undefined;
      if (storybookGoal) {
        const tokensUsed = Math.max(storybookGoal.tokensUsed, input?.tokensUsed ?? 0);
        storybookGoal = {
          ...storybookGoal,
          tokensUsed,
          timeUsedSeconds: storybookGoal.timeUsedSeconds + (input?.timeUsedSeconds ?? 0),
          status:
            storybookGoal.tokenBudget && tokensUsed >= storybookGoal.tokenBudget
              ? "budgetLimited"
              : storybookGoal.status,
          updatedAt: new Date().toISOString(),
        };
      }
      return storybookGoal as T;
    }
    case "create_saved_prompt": {
      const input = args?.input as SavedPromptInput | undefined;
      const savedPrompt = {
        id: `saved-storybook-${storybookSavedPrompts.length + 1}`,
        label: input?.label ?? "",
        prompt: input?.prompt ?? "",
      };
      storybookSavedPrompts = [...storybookSavedPrompts, savedPrompt];
      return savedPrompt as T;
    }
    case "update_saved_prompt": {
      const input = args?.input as SavedPromptInput | undefined;
      const savedPrompt = {
        id: String(args?.id ?? "saved-storybook-updated"),
        label: input?.label ?? "",
        prompt: input?.prompt ?? "",
      };
      storybookSavedPrompts = storybookSavedPrompts.map((current) =>
        current.id === savedPrompt.id ? savedPrompt : current,
      );
      return savedPrompt as T;
    }
    case "cancel_agent_run":
      emitMockEvent(AGENT_RUN_EVENT, {
        runId: String(args?.runId ?? "run-storybook"),
        event: { type: "lifecycle", status: "cancelled", message: "storybook cancel" },
      });
      return undefined as T;
    case "send_prompt_to_run":
      emitMockEvent(AGENT_RUN_EVENT, {
        runId: String(args?.runId ?? "run-storybook"),
        event: { type: "lifecycle", status: "promptSent", message: "storybook queued prompt sent" },
      });
      return undefined as T;
    case "list_git_remotes":
      return sampleRemotes as T;
    case "list_git_branches":
      return sampleBranches as T;
    case "list_git_worktrees":
      return sampleWorktrees as T;
    case "get_worktree_changes":
      return {
        ...sampleWorktreeChanges,
        workingDirectory: String(args?.workingDirectory ?? sampleWorktreeChanges.workingDirectory),
      } as T;
    case "get_worktree_file_diff":
      return {
        path: String(args?.path ?? "sample.ts"),
        diff: [
          "diff --git a/sample.ts b/sample.ts",
          "index 1111111..2222222 100644",
          "--- a/sample.ts",
          "+++ b/sample.ts",
          "@@ -1,3 +1,3 @@",
          "-old line",
          "+new line",
        ].join("\n"),
        truncated: false,
        binary: false,
      } as T;
    case "list_agents":
      return sampleAgents as T;
    case "start_agent_run": {
      const request = args?.request as { runId?: string; goal?: string; agentId?: string } | undefined;
      const runId = String(request?.runId ?? "run-storybook");
      queueMicrotask(() => {
        emitMockEvent(AGENT_RUN_EVENT, {
          runId,
          event: { type: "lifecycle", status: "started", message: "storybook run started" },
        });
        emitMockEvent(AGENT_RUN_EVENT, {
          runId,
          event: { type: "lifecycle", status: "promptSent", message: "storybook prompt sent" },
        });
        emitMockEvent(AGENT_RUN_EVENT, {
          runId,
          event: { type: "usage", used: 42000, size: 128000 },
        });
        emitMockEvent(AGENT_RUN_EVENT, {
          runId,
          event: {
            type: "agentMessage",
            text: "Storybook mock response is streaming so the panel can be inspected without launching an ACP process.",
          },
        });
      });
      return {
        id: runId,
        goal: String(request?.goal ?? ""),
        agentId: String(request?.agentId ?? "codex"),
      } as T;
    }
    default:
      throw new Error(`Unhandled Storybook Tauri command: ${command}`);
  }
}
