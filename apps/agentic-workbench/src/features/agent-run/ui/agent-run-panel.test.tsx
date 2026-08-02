// @vitest-environment happy-dom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  cleanupAgentRunPanelTests,
  renderAgentRunPanel,
  waitForAgentRunPanel,
} from "./agent-run-panel.test-harness";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    switch (command) {
      case "list_agents":
        return [
          {
            id: "codex",
            label: "Codex",
            command: "codex-acp",
            models: [{ id: "gpt-5.6", label: "GPT-5.6" }],
            contextSizes: [{ id: "large", label: "Large" }],
          },
        ];
      case "get_agent_run_settings":
      case "get_goal":
        return null;
      case "list_agent_tool_command_candidates":
        return { candidates: [] };
      case "list_provider_sessions":
      case "list_saved_prompts":
        return [];
      case "save_agent_run_settings":
        return args?.settings;
      case "start_agent_run": {
        const request = args?.request as
          | { runId?: string; goal?: string; agentId?: string }
          | undefined;
        return {
          id: request?.runId ?? "run-test",
          goal: request?.goal ?? "",
          agentId: request?.agentId ?? "codex",
        };
      }
      default:
        throw new Error(`Unhandled AgentRunPanel test command: ${command}`);
    }
  });
});

afterEach(async () => {
  await cleanupAgentRunPanelTests();
});

describe("AgentRunPanel user boundary", () => {
  it("does not publish an empty run before orchestration hydration", async () => {
    const onRunStateChange = vi.fn();
    const panel = await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-main",
      existingRunId: null,
      existingIsRunning: false,
      runtimeHydrated: false,
      onRunStateChange,
    });

    expect(onRunStateChange).not.toHaveBeenCalled();

    await panel.rerender({ runtimeHydrated: true });
    await waitForAgentRunPanel(() => onRunStateChange.mock.calls.length === 1);

    expect(onRunStateChange).toHaveBeenLastCalledWith({
      panelId: "main-agent-run",
      isRunning: false,
      activeRunId: null,
    });
  });

  it("prepares the Main Coordinator before launching the ACP run", async () => {
    let finishPreparing: (() => void) | undefined;
    const preparing = new Promise<void>((resolve) => {
      finishPreparing = resolve;
    });
    const onBeforeRunStart = vi.fn(() => preparing);
    const panel = await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-main",
      onBeforeRunStart,
    });

    await panel.enterPrompt("Inspect the current worktree");
    await panel.pressPromptKey("Enter");
    await waitForAgentRunPanel(() => onBeforeRunStart.mock.calls.length === 1);

    expect(invocationsFor("start_agent_run")).toEqual([]);

    finishPreparing?.();
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 1);

    expect(invocationsFor("start_agent_run")[0]).toMatchObject({
      panelId: "main-agent-run",
      request: {
        goal: "Inspect the current worktree",
        agentId: "codex",
        cwd: "/tmp/agent-run-panel-main",
      },
    });
  });

  it("drives the same prompt and run-event contract in an additional panel", async () => {
    const panel = await renderAgentRunPanel({
      panelId: "child-agent-run",
      workingDirectory: "/tmp/agent-run-panel-child",
      variant: "extra",
      initialPermissionMode: "readOnly",
    });

    await panel.enterPrompt("Review the proposed change");
    await panel.clickButton("Run");
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 1);

    const startInvocation = invocationsFor("start_agent_run")[0] as {
      panelId: string;
      request: { runId: string };
    };
    expect(startInvocation).toMatchObject({
      panelId: "child-agent-run",
      request: {
        goal: "Review the proposed change",
        agentId: "codex",
        cwd: "/tmp/agent-run-panel-child",
        permissionMode: "readOnly",
      },
    });

    await panel.emitRunEvent({
      runId: startInvocation.request.runId,
      event: {
        type: "agentMessage",
        text: "The additional panel received the agent response.",
      },
    });

    await waitForAgentRunPanel(() =>
      panel.container.textContent?.includes(
        "The additional panel received the agent response.",
      ) ?? false,
    );
  });
});

function invocationsFor(command: string) {
  return invokeMock.mock.calls
    .filter(([calledCommand]) => calledCommand === command)
    .map(([, args]) => args);
}
