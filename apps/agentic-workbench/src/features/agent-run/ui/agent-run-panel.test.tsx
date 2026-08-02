// @vitest-environment happy-dom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  AgentDescriptor,
  AgentRunSettings,
  AgentToolCommandCandidate,
  AgentToolCommandCandidateResponse,
} from "@/entities/agent-run/model/types";

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

let toolCommandCandidateResponse: AgentToolCommandCandidateResponse;
let loadToolCommandCandidates: () => Promise<AgentToolCommandCandidateResponse>;
let savedRunSettings: AgentRunSettings | null;
let loadAgents: () => Promise<AgentDescriptor[]>;

const codexAgents: AgentDescriptor[] = [
  {
    id: "codex",
    label: "Codex",
    command: "codex-acp",
    models: [{ id: "gpt-5.6", label: "GPT-5.6" }],
    efforts: [
      { id: "low", label: "Low" },
      { id: "high", label: "High" },
    ],
    contextSizes: [{ id: "large", label: "Large" }],
  },
];

const mixedCommandCandidates: AgentToolCommandCandidate[] = [
  {
    id: "session:set_window_title",
    name: "set_window_title",
    description: "Change the current Worktree Session window title.",
    insertText: "$set_window_title",
    source: "sessionTool",
    scope: { runId: "run-autocomplete", agentId: "codex", workingDirectory: "/tmp/repo" },
  },
  {
    id: "app:goal",
    name: "goal",
    description: "Manage the current AW goal.",
    insertText: "/goal",
    source: "appCommand",
    scope: { agentId: "codex", workingDirectory: "/tmp/repo" },
  },
  {
    id: "extension:speckit-implement",
    name: "speckit-implement",
    description: "Execute the current specification tasks.",
    insertText: "$speckit-implement",
    source: "extension",
    scope: { agentId: "codex", workingDirectory: "/tmp/repo" },
  },
];

beforeEach(() => {
  invokeMock.mockReset();
  toolCommandCandidateResponse = { status: "empty", candidates: [] };
  savedRunSettings = null;
  loadAgents = async () => codexAgents;
  loadToolCommandCandidates = async () => toolCommandCandidateResponse;
  invokeMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    switch (command) {
      case "list_agents":
        return loadAgents();
      case "get_agent_run_settings":
        return savedRunSettings;
      case "get_goal":
        return null;
      case "list_agent_tool_command_candidates":
        return loadToolCommandCandidates();
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
  it("reloads the worktree-scoped Codex model and effort selections", async () => {
    let finishLoadingAgents: ((agents: AgentDescriptor[]) => void) | undefined;
    loadAgents = () =>
      new Promise<AgentDescriptor[]>((resolve) => {
        finishLoadingAgents = resolve;
      });
    savedRunSettings = {
      workingDirectory: "/tmp/agent-run-panel-restored-settings",
      agentId: "codex",
      permissionMode: "default",
      modelId: "gpt-5.6",
      effortId: "high",
      contextSize: "default",
      sessionMode: "new",
      ralphLoop: {
        enabled: false,
        maxIterations: 5,
        delayMs: 0,
        stopOnError: true,
        stopOnPermission: false,
        promptTemplate: "",
      },
    };
    await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-restored-settings",
    });

    await waitForAgentRunPanel(() => invocationsFor("get_agent_run_settings").length > 0);
    finishLoadingAgents?.(codexAgents);

    await waitForAgentRunPanel(() =>
      document
        .querySelector("button[aria-label='main-agent-run model']")
        ?.textContent?.includes("GPT-5.6") ?? false,
    );
    expect(
      document.querySelector("button[aria-label='main-agent-run effort']")?.textContent,
    ).toContain("High");
  });

  it("shows compact Codex settings in the external production composer and sends selections", async () => {
    const runConfigurationPortal = document.createElement("div");
    document.body.append(runConfigurationPortal);
    const panel = await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-codex-settings",
      showPromptComposer: false,
      runConfigurationPortal,
    });

    await waitForAgentRunPanel(() =>
      Boolean(document.querySelector("button[aria-label='main-agent-run model']")),
    );
    const model = document.querySelector<HTMLButtonElement>(
      "button[aria-label='main-agent-run model']",
    );
    const effort = document.querySelector<HTMLButtonElement>(
      "button[aria-label='main-agent-run effort']",
    );
    expect(model?.textContent).toContain("Provider default");
    expect(effort?.textContent).toContain("Provider default");

    await panel.selectOption("main-agent-run model", "GPT-5.6");
    await panel.selectOption("main-agent-run effort", "High");
    await panel.rerender({
      externalPromptRequest: {
        id: "production-composer-request",
        text: "Use the selected Codex configuration",
        delivery: "send",
      },
    });
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 1);

    expect(invocationsFor("start_agent_run")[0]).toMatchObject({
      request: {
        modelId: "gpt-5.6",
        effortId: "high",
      },
    });
    expect(model?.disabled).toBe(true);
    expect(effort?.disabled).toBe(true);

    await waitForAgentRunPanel(() =>
      invocationsFor("save_agent_run_settings").some(
        (args) =>
          (args as { settings?: { effortId?: string } }).settings?.effortId === "high",
      ),
    );
  });

  it("persists a focused additional panel selection through the main worktree owner", async () => {
    let worktreeRunConfiguration: { modelId: string; effortId: string } = {
      modelId: "providerDefault",
      effortId: "providerDefault",
    };
    const onWorktreeRunConfigurationChange = vi.fn((configuration) => {
      worktreeRunConfiguration = configuration;
    });
    const runConfigurationPortal = document.createElement("div");
    document.body.append(runConfigurationPortal);
    const extraPanel = await renderAgentRunPanel({
      panelId: "extra-agent-run",
      workingDirectory: "/tmp/agent-run-panel-shared-settings",
      variant: "extra",
      showPromptComposer: false,
      runConfigurationPortal,
      worktreeRunConfiguration,
      onWorktreeRunConfigurationChange,
    });

    await extraPanel.selectOption("extra-agent-run model", "GPT-5.6");
    await extraPanel.selectOption("extra-agent-run effort", "High");
    await waitForAgentRunPanel(
      () => worktreeRunConfiguration?.modelId === "gpt-5.6" && worktreeRunConfiguration.effortId === "high",
    );
    await extraPanel.unmount();

    await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-shared-settings",
      worktreeRunConfiguration,
    });

    await waitForAgentRunPanel(
      () =>
        document
          .querySelector("button[aria-label='main-agent-run effort']")
          ?.textContent?.includes("High") ?? false,
    );
    await waitForAgentRunPanel(() => invocationsFor("save_agent_run_settings").length > 0);
    const saveInvocations = invocationsFor("save_agent_run_settings");
    expect(saveInvocations[saveInvocations.length - 1]).toMatchObject({
      settings: { modelId: "gpt-5.6", effortId: "high" },
    });
  });

  it("shows only slash command sources and applies the highlighted command", async () => {
    toolCommandCandidateResponse = {
      status: "ready",
      candidates: mixedCommandCandidates,
    };
    const panel = await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-main",
    });

    await panel.enterPrompt("/");
    await waitForAgentRunPanel(() => Boolean(panel.container.querySelector("[role='listbox']")));

    const suggestions = panel.container.querySelector("[role='listbox']")?.textContent ?? "";
    expect(suggestions).toContain("goal");
    expect(suggestions).toContain("Manage the current AW goal.");
    expect(suggestions).toContain("appCommand");
    expect(suggestions).not.toContain("set_window_title");
    expect(suggestions).not.toContain("speckit-implement");

    await panel.pressPromptKey("Enter");
    expect(panel.promptValue()).toBe("/goal");
    expect(panel.promptSelection()).toEqual({ start: 5, end: 5 });
    expect(invocationsFor("start_agent_run")).toEqual([]);
  });

  it("uses dollar command sources with keyboard and pointer selection in an additional panel", async () => {
    toolCommandCandidateResponse = {
      status: "ready",
      candidates: mixedCommandCandidates,
    };
    const panel = await renderAgentRunPanel({
      panelId: "child-agent-run",
      workingDirectory: "/tmp/agent-run-panel-child",
      variant: "extra",
    });

    await panel.enterPrompt("$");
    await waitForAgentRunPanel(() => Boolean(panel.container.querySelector("[role='listbox']")));

    const suggestions = panel.container.querySelector("[role='listbox']")?.textContent ?? "";
    expect(suggestions).toContain("set_window_title");
    expect(suggestions).toContain("sessionTool");
    expect(suggestions).toContain("speckit-implement");
    expect(suggestions).toContain("extension");
    expect(suggestions).not.toContain("Manage the current AW goal.");

    await panel.pressPromptKey("ArrowDown");
    await waitForAgentRunPanel(() =>
      Boolean(
        [...panel.container.querySelectorAll("[role='option']")].find(
          (option) =>
            option.textContent?.includes("set_window_title") &&
            option.getAttribute("aria-selected") === "true",
        ),
      ),
    );
    await panel.pressPromptKey("ArrowUp");
    await waitForAgentRunPanel(() =>
      Boolean(
        [...panel.container.querySelectorAll("[role='option']")].find(
          (option) =>
            option.textContent?.includes("speckit-implement") &&
            option.getAttribute("aria-selected") === "true",
        ),
      ),
    );
    await panel.pressPromptKey("ArrowDown");
    await panel.pressPromptKey("Tab");

    const keyboardSelection = "$set_window_title";
    expect(panel.promptValue()).toBe(keyboardSelection);
    expect(panel.promptSelection()).toEqual({
      start: keyboardSelection.length,
      end: keyboardSelection.length,
    });

    await panel.enterPrompt("$spec");
    await waitForAgentRunPanel(() =>
      panel.container.textContent?.includes("speckit-implement") ?? false,
    );
    await panel.selectSuggestionWithPointer("speckit-implement");

    const pointerSelection = "$speckit-implement";
    expect(panel.promptValue()).toBe(pointerSelection);
    expect(panel.promptSelection()).toEqual({
      start: pointerSelection.length,
      end: pointerSelection.length,
    });
    expect(invocationsFor("start_agent_run")).toEqual([]);
  });

  it("treats candidates for the other prefix as an empty source", async () => {
    toolCommandCandidateResponse = {
      status: "ready",
      candidates: [mixedCommandCandidates[0]],
    };
    const panel = await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-empty-slash",
    });

    await panel.enterPrompt("/");
    await waitForAgentRunPanel(() =>
      panel.container.textContent?.includes("No commands available") ?? false,
    );

    expect(panel.promptValue()).toBe("/");
    expect(panel.container.querySelectorAll("[role='option']")).toHaveLength(0);
  });

  it.each([
    ["loading", "Loading commands..."],
    ["empty", "No commands available"],
    ["noMatch", "No matching commands"],
    ["error", "Commands unavailable"],
  ] as const)("keeps prompt editing available in the %s fallback", async (status, message) => {
    let prompt = "/";
    if (status === "loading") {
      loadToolCommandCandidates = () =>
        new Promise<AgentToolCommandCandidateResponse>(() => undefined);
    } else if (status === "noMatch") {
      prompt = "/missing";
      toolCommandCandidateResponse = {
        status: "ready",
        candidates: [mixedCommandCandidates[1]],
      };
    } else if (status === "error") {
      loadToolCommandCandidates = async () => {
        throw new Error("candidate lookup failed");
      };
    }
    const panel = await renderAgentRunPanel({
      panelId: "child-agent-run",
      workingDirectory: `/tmp/agent-run-panel-${status}`,
      variant: "extra",
    });

    await panel.enterPrompt(prompt);
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(message) ?? false);
    expect(panel.promptValue()).toBe(prompt);

    await panel.pressPromptKey("Escape");
    await waitForAgentRunPanel(() => !panel.container.querySelector("[role='listbox']"));
    expect(panel.promptValue()).toBe(prompt);

    const continuedPrompt = `${prompt} keep typing`;
    await panel.enterPrompt(continuedPrompt);
    expect(panel.promptValue()).toBe(continuedPrompt);
  });

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
    expect(
      document.querySelector<HTMLButtonElement>(
        "button[aria-label='main-agent-run model']",
      )?.disabled,
    ).toBe(true);
    expect(
      document.querySelector<HTMLButtonElement>(
        "button[aria-label='main-agent-run effort']",
      )?.disabled,
    ).toBe(true);

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
