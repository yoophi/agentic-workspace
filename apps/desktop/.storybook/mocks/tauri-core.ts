import {
  sampleAgents,
  sampleBranches,
  sampleProjects,
  sampleRemotes,
  sampleSavedPrompts,
  sampleWorktrees,
} from "../../src/shared/storybook/sample-data";
import type { SavedPrompt, SavedPromptInput } from "../../src/entities/saved-prompt/model/types";
import { emitMockEvent } from "./tauri-event";

const AGENT_RUN_EVENT = "agent-run-event";
let storybookSavedPrompts: SavedPrompt[] = [...sampleSavedPrompts];

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
