// @vitest-environment happy-dom

import { act } from "react";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import type {
  AgentDescriptor,
  AgentRunSettings,
  AgentToolCommandCandidate,
  AgentToolCommandCandidateResponse,
} from "@/entities/agent-run/model/types";

import {
  cleanupAgentRunPanelTests,
  renderAgentRunPanel,
  setRunEventEmitter,
  waitForAgentRunPanel,
} from "./agent-run-panel.test-harness";
import {
  compatTransport,
  MESSAGE_NOT_APPLIED,
  MESSAGE_RESULT_UNKNOWN,
  setTransport,
} from "@/shared/api/transport";
import {
  startCompatSimulatingServer,
  type CompatSimulatingServer,
} from "@/shared/api/transport/testing/compat-simulating-server";

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
      case "discard_agent_exchange_delivery":
      case "send_prompt_to_run":
        return null;
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

// 043 T027(FR-012): 같은 시나리오·같은 기대값을 호환 경로와 네트워크 경로(실제 루프백 HTTP의 가짜 Workbench 서버 →
// 실제 fetch → call client → HttpTransport)에서 모두 돌린다. 가짜 서버는 받은 operation을 command 인자로 되돌려 같은
// `invokeMock`을 부르므로 호출 기록 단정도 두 경로에서 같은 뜻이다.
describe.each(["compat", "http"] as const)("AgentRunPanel user boundary [%s]", (path) => {
  let server: CompatSimulatingServer | undefined;

  beforeAll(async () => {
    if (path === "http") {
      server = await startCompatSimulatingServer((command, args) => invokeMock(command, args));
    }
  });

  afterAll(async () => {
    setTransport(compatTransport);
    setRunEventEmitter(undefined);
    await server?.close();
  });

  beforeEach(() => {
    setTransport(path === "http" && server ? server.transport : compatTransport);
    // 네트워크 경로의 run 이벤트는 창 삽입이 아니라 서버 스트림(`run:<id>`)으로 온다.
    setRunEventEmitter(
      path === "http" && server
        ? (envelope) => void server?.hub.publish(`run:${envelope.runId}`, envelope.event, "run.event.v1")
        : undefined,
    );
  });

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
    // 후보를 다 불러온 목록을 기다린다(목록은 먼저 "Loading commands..."로 뜬다 — 네트워크 경로에서 드러난 대기 조건).
    await waitForLoadedSuggestions(panel.container);

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
    // 후보를 다 불러온 목록을 기다린다(목록은 먼저 "Loading commands..."로 뜬다 — 네트워크 경로에서 드러난 대기 조건).
    await waitForLoadedSuggestions(panel.container);

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

    // agent 목록을 불러온 뒤(모델 버튼이 있음) 입력한다 — 호환 경로에서는 렌더 직후 이미 불러온 상태였다.
    await waitForAgentRunPanel(() =>
      Boolean(document.querySelector("button[aria-label='main-agent-run model']")),
    );
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

  it("starts a run for an exchange prompt with the exchange delivery key (043 T036)", async () => {
    const recordedBefore = server?.calls.length ?? 0;
    await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-main",
      externalPromptRequest: { id: "x-42", text: "Handle the peer request", exchangeRequestId: "x-42" },
    });
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 1);
    expect(invocationsFor("start_agent_run")[0]).toMatchObject({
      request: { goal: "Handle the peer request", cwd: "/tmp/agent-run-panel-main" },
    });
    if (path === "http") {
      // 네트워크 경로: 서버가 받은 run.start에 교환 요청 id로 만든 멱등성 키가 실린다.
      const starts = (server?.calls.slice(recordedBefore) ?? []).filter((call) => call.operation === "run.start");
      expect(starts.map((call) => call.idempotencyKey)).toEqual(["exchange-delivery:x-42"]);
    }
  });

  // Codex r7(apps medium): 바쁜 run에 도착한 교환 prompt는 패널 대기열에 들어가고, 서버에는 이미 전달 확인(`delivered`)됐다.
  // 그 항목을 지우면 서버에 전달 포기를 알려야 하고(아니면 미소비 교환이 wait-stop을 막는다), steer(즉시 전송)는 교환
  // 소비를 싣지 못하므로 교환 항목에서는 막는다. 일반 대기 prompt의 제거는 서버를 부르지 않는다.
  it("discards a removed exchange prompt on the server and does not steer it", async () => {
    const panel = await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-main",
      externalPromptRequest: { id: "start-1", text: "Work on the task", delivery: "send" },
    });
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 1);
    const runId = (invocationsFor("start_agent_run")[0] as { request: { runId: string } }).request.runId;
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptSent", message: "sent" } });

    await panel.rerender({
      externalPromptRequest: {
        id: "x-7",
        text: "Handle the peer request",
        delivery: "queue",
        exchangeRequestId: "x-7",
      },
    });
    await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") !== null);
    await panel.rerender({
      externalPromptRequest: { id: "manual-2", text: "A plain follow-up", delivery: "queue" },
    });
    await waitForAgentRunPanel(() => queuedPromptButton(2, "제거") !== null);

    // 교환 항목은 steer 불가, 일반 항목은 steer 가능.
    expect(queuedPromptButton(1, "즉시 전송")?.disabled).toBe(true);
    expect(queuedPromptButton(2, "즉시 전송")?.disabled).toBe(false);

    await act(async () => {
      queuedPromptButton(1, "제거")?.click();
    });
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length === 1);
    expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([{ requestId: "x-7" }]);
    await waitForAgentRunPanel(() => !panel.container.textContent?.includes("Handle the peer request"));

    // 남은 일반 항목의 제거는 서버를 부르지 않는다.
    await act(async () => {
      queuedPromptButton(1, "제거")?.click();
    });
    await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") === null);
    expect(invocationsFor("discard_agent_exchange_delivery")).toHaveLength(1);
    expect(invocationsFor("steer_prompt_to_run")).toEqual([]);

    // turn이 끝나도 지운 교환은 보내지 않는다.
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    expect(
      invocationsFor("send_prompt_to_run").filter(
        (args) => (args as { continuation?: unknown }).continuation !== undefined,
      ),
    ).toEqual([]);
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

// Codex r8(apps medium): 취소가 서버에서 끝났는지 모르는 결과(네트워크 경로의 notApplied·unknown)는 "취소됨"이 아니다.
// run은 살아 있을 수 있으므로 패널은 대기열(서버가 이미 전달 확인한 교환 항목 포함)을 버리지 않고, 새 run을 시작하지
// 않는다. 실제 run 상태는 복구된 run 이벤트가 맞춘다: 살아 있으면 turn 끝에 교환이 이어 가기 표지로 전달되고, 실제로
// 취소됐으면 run 끝 이벤트가 패널을 정리한다(대상 run이 없는 교환은 서버가 세지 않는다). 결과 분류는 네트워크 경로에만
// 있으므로 여기서는 호환 transport 앞에 `cancel_agent_run` 결과만 1회 바꾸는 transport를 둔다(나머지는 그대로 통과).
describe("AgentRunPanel when a cancel is not known to have reached the server (Codex r8)", () => {
  let cancelOutcome: string | undefined;
  /** 결과를 돌려주기 전에 할 일(서버가 취소를 적용해 끝 이벤트를 먼저 보낸 경우). */
  let beforeCancelReply: (() => Promise<void>) | undefined;
  /** Codex r9: 다음 취소 한 번의 답을 붙잡는 문(열릴 때까지 답하지 않는다 — 그 사이 대기열이 바뀌는 순서). */
  let cancelGate: Promise<void> | undefined;
  /** 화면이 보낸 취소 요청 수(답을 받기 전에 센다). */
  let cancelRequests = 0;
  /** Codex r11: 다음 전달 포기 한 번의 답을 붙잡는 문과, 그 답을 실패로 바꾸는 오류. */
  let discardGate: Promise<void> | undefined;
  let discardOutcome: string | undefined;
  /** Codex r12: 화면이 보낸 `send_prompt_to_run`의 호출 옵션(멱등성 키). */
  let sendOptions: Array<{ args: unknown; options: unknown }> = [];

  beforeEach(() => {
    sendOptions = [];
    cancelOutcome = undefined;
    beforeCancelReply = undefined;
    cancelGate = undefined;
    cancelRequests = 0;
    discardGate = undefined;
    discardOutcome = undefined;
    setTransport({
      kind: "http",
      invoke: async (command, args, options) => {
        if (command === "send_prompt_to_run") {
          sendOptions.push({ args, options });
        }
        if (command === "discard_agent_exchange_delivery" && (discardGate || discardOutcome !== undefined)) {
          const gate = discardGate;
          const thrown = discardOutcome;
          discardGate = undefined;
          discardOutcome = undefined;
          await gate;
          if (thrown !== undefined) {
            // 기록은 남긴다(요청은 보냈다).
            await compatTransport.invoke(command, args, options);
            throw thrown;
          }
        }
        if (command === "cancel_agent_run") {
          cancelRequests += 1;
          const gate = cancelGate;
          cancelGate = undefined;
          if (gate) {
            await gate;
          }
        }
        if (command === "cancel_agent_run" && cancelOutcome !== undefined) {
          const thrown = cancelOutcome;
          cancelOutcome = undefined;
          await beforeCancelReply?.();
          throw thrown;
        }
        return compatTransport.invoke(command, args, options);
      },
      listen: (event, callback) => compatTransport.listen(event, callback),
    });
    const base = invokeMock.getMockImplementation();
    invokeMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "steer_prompt_to_run") {
        throw "steer unsupported: active-turn steer is not supported by this ACP agent; choose Cancel & send or Queue";
      }
      if (command === "cancel_agent_run") {
        return null;
      }
      return base?.(command, args);
    });
  });

  afterAll(() => {
    setTransport(compatTransport);
  });

  async function busyRunWithAQueuedExchange() {
    const panel = await renderAgentRunPanel({
      panelId: "main-agent-run",
      workingDirectory: "/tmp/agent-run-panel-main",
      externalPromptRequest: { id: "start-1", text: "Work on the task", delivery: "send" },
    });
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 1);
    const runId = (invocationsFor("start_agent_run")[0] as { request: { runId: string } }).request.runId;
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptSent", message: "sent" } });
    await panel.rerender({
      externalPromptRequest: { id: "x-7", text: "Handle the peer request", delivery: "queue", exchangeRequestId: "x-7" },
    });
    await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") !== null);
    return { panel, runId };
  }

  async function rejectASteer(panel: Awaited<ReturnType<typeof renderAgentRunPanel>>) {
    await panel.rerender({ externalPromptRequest: { id: "manual-2", text: "Change direction", delivery: "queue" } });
    await waitForAgentRunPanel(() => queuedPromptButton(2, "즉시 전송") !== null);
    await act(async () => {
      queuedPromptButton(2, "즉시 전송")?.click();
    });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("Steer rejected #1") ?? false);
  }

  function exchangeDeliveries() {
    return invocationsFor("send_prompt_to_run").filter(
      (args) => (args as { continuation?: { exchangeRequestId?: string } }).continuation?.exchangeRequestId === "x-7",
    );
  }

  it.each([
    ["notApplied", MESSAGE_NOT_APPLIED],
    ["unknown", MESSAGE_RESULT_UNKNOWN],
  ])("keeps the queued exchange when a full restart's cancel is %s, and delivers it after the turn", async (_kind, error) => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await rejectASteer(panel);

    cancelOutcome = error;
    await panel.clickButton("Full restart");
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(error) ?? false);

    expect(invocationsFor("start_agent_run"), "no new run replaces a run that may still be alive").toHaveLength(1);
    expect(invocationsFor("discard_agent_exchange_delivery"), "the exchange is not abandoned").toEqual([]);
    expect(panel.container.textContent).toContain("Handle the peer request");
    expect(panel.container.textContent, "the rejected steer stays for another try").toContain("Steer rejected #1");
    expect(exchangeDeliveries(), "nothing is sent while the turn is still running").toEqual([]);

    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await waitForAgentRunPanel(() => exchangeDeliveries().length === 1);
    expect(exchangeDeliveries()[0]).toMatchObject({ runId, prompt: "Handle the peer request" });
  });

  it("continues an unknown full restart exactly once when the recovered events show the cancel applied", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await rejectASteer(panel);

    cancelOutcome = MESSAGE_RESULT_UNKNOWN;
    await panel.clickButton("Full restart");
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false);
    expect(invocationsFor("start_agent_run")).toHaveLength(1);
    expect(exchangeDeliveries(), "nothing is sent while the result is unknown and the turn runs").toEqual([]);

    // 서버는 실제로 취소했다(응답만 유실): 복구된 스트림에 그 run의 취소 끝이 온다 → 재시작을 한 번 잇는다.
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 2);
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length === 1);
    expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([{ requestId: "x-7" }]);
    const restarted = invocationsFor("start_agent_run")[1] as { request: { goal: string } };
    expect(restarted.request.goal).toContain("Change direction");
    expect(exchangeDeliveries(), "the exchange never goes to the replacement run").toEqual([]);

    // 같은 run의 늦은 끝 이벤트가 다시 와도 두 번째 재시작은 없다.
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    expect(invocationsFor("start_agent_run")).toHaveLength(2);
  });

  it("continues an unknown full restart when the run's cancel end arrived before the result", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await rejectASteer(panel);

    cancelOutcome = MESSAGE_RESULT_UNKNOWN;
    beforeCancelReply = () =>
      panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    await panel.clickButton("Full restart");
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 2);
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length === 1);
    expect(exchangeDeliveries()).toEqual([]);
    expect(panel.container.textContent, "the ended run's queue is not revived").not.toContain("Handle the peer request");
  });

  it("drops an unknown full restart once the run accepts a new turn (the cancel did not apply)", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await rejectASteer(panel);

    cancelOutcome = MESSAGE_RESULT_UNKNOWN;
    await panel.clickButton("Full restart");
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false);
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await waitForAgentRunPanel(() => exchangeDeliveries().length === 1);
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptSent", message: "sent" } });
    // 나중에 이 run이 끝나도(예: 사용자가 취소) 버린 재시작은 되살아나지 않는다.
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    expect(invocationsFor("start_agent_run")).toHaveLength(1);
  });

  it.each([
    ["notApplied", MESSAGE_NOT_APPLIED],
    ["unknown", MESSAGE_RESULT_UNKNOWN],
  ])("keeps the run and its queued exchange when a cancel is %s", async (_kind, error) => {
    const { panel, runId } = await busyRunWithAQueuedExchange();

    cancelOutcome = error;
    await panel.clickButton("Cancel");
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(error) ?? false);

    expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([]);
    expect(panel.container.textContent).toContain("Handle the peer request");
    expect(exchangeDeliveries(), "nothing is sent while the turn is still running").toEqual([]);

    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await waitForAgentRunPanel(() => exchangeDeliveries().length === 1);
  });

  it("lets the recovered run events settle a cancel that did apply: the run ends and nothing is delivered", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();

    cancelOutcome = MESSAGE_RESULT_UNKNOWN;
    await panel.clickButton("Cancel");
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false);
    // 서버는 실제로 취소했다: 복구된 스트림에 run 끝이 온다.
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    await waitForAgentRunPanel(() => !(panel.container.textContent?.includes("Handle the peer request") ?? true));
    expect(exchangeDeliveries()).toEqual([]);
  });

  function deferred() {
    let release!: () => void;
    const promise = new Promise<void>((resolve) => {
      release = resolve;
    });
    return { promise, release };
  }

  function deliveriesOf(requestId: string) {
    return invocationsFor("send_prompt_to_run").filter(
      (args) => (args as { continuation?: { exchangeRequestId?: string } }).continuation?.exchangeRequestId === requestId,
    );
  }

  // Codex r9(apps medium): 재시작 취소의 답을 기다리는 동안 대기열에 들어온 항목(서버가 이미 전달 확인한 새 교환 포함)은 취소가
  // 끝나지 않았을 때 호출 전 대기열로 덮어써 지우지 않는다 — 살아 있는 run에 turn 끝에 전달된다.
  it.each([
    ["unknown", MESSAGE_RESULT_UNKNOWN],
    ["refused by the server", "cancel refused by the server"],
  ])(
    "keeps an exchange that arrived while a full restart's cancel was pending when the cancel is %s",
    async (_kind, error) => {
      const { panel, runId } = await busyRunWithAQueuedExchange();
      await rejectASteer(panel);

      const gate = deferred();
      cancelGate = gate.promise;
      cancelOutcome = error;
      await panel.clickButton("Full restart");
      await panel.rerender({
        externalPromptRequest: { id: "x-8", text: "Second peer request", delivery: "queue", exchangeRequestId: "x-8" },
      });
      await waitForAgentRunPanel(() => panel.container.textContent?.includes("Second peer request") ?? false);
      await act(async () => {
        gate.release();
      });
      await waitForAgentRunPanel(() => panel.container.textContent?.includes(error) ?? false);

      expect(panel.container.textContent, "the exchange that arrived during the cancel stays queued").toContain(
        "Second peer request",
      );
      expect(panel.container.textContent).toContain("Handle the peer request");
      expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([]);
      expect(invocationsFor("start_agent_run")).toHaveLength(1);

      await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
      await waitForAgentRunPanel(() => deliveriesOf("x-7").length === 1);
      await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptSent", message: "sent" } });
      await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
      await waitForAgentRunPanel(() => deliveriesOf("x-8").length === 1);
      expect(deliveriesOf("x-8")[0]).toMatchObject({ runId, prompt: "Second peer request" });
    },
  );

  // 취소가 끝났으면 그 사이 들어온 항목도 같은 규칙이다: 교환(취소한 run이 대상)은 버리고 서버에서도 끝내며, 일반 prompt는
  // 새 run으로 옮긴다.
  it("moves prompts that arrived during a successful full-restart cancel and abandons the exchanges that did", async () => {
    const { panel } = await busyRunWithAQueuedExchange();
    await rejectASteer(panel);

    const gate = deferred();
    cancelGate = gate.promise;
    await panel.clickButton("Full restart");
    await panel.rerender({
      externalPromptRequest: { id: "x-8", text: "Second peer request", delivery: "queue", exchangeRequestId: "x-8" },
    });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("Second peer request") ?? false);
    await panel.rerender({ externalPromptRequest: { id: "manual-3", text: "Also do this", delivery: "queue" } });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("Also do this") ?? false);
    await act(async () => {
      gate.release();
    });

    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 2);
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length === 2);
    expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([{ requestId: "x-7" }, { requestId: "x-8" }]);
    expect(panel.container.textContent, "the plain prompt moves to the replacement run").toContain("Also do this");
    expect(panel.container.textContent).not.toContain("Second peer request");
  });

  // Codex r10(apps medium): 취소의 답을 기다리는 동안 그 run의 turn이 끝났다(`promptCompleted`). 취소 진행은 turn 응답 대기와
  // 다른 상태다: 기다리는 동안에는 취소 중인 run에 대기열을 보내지 않고, 취소가 끝나지 않았으면(결과 모름·거절·미전송) 호출 전
  // 응답 대기 값으로 되돌리지 않는다 — 쉬는 run에 대기한 교환이 이어 가기 표지로 전달된다.
  it.each([
    ["unknown", MESSAGE_RESULT_UNKNOWN],
    ["refused by the server", "cancel refused by the server"],
  ])(
    "a turn that ended while a full restart's cancel was pending sends the queued exchange once the cancel is %s",
    async (_kind, error) => {
      const { panel, runId } = await busyRunWithAQueuedExchange();
      await rejectASteer(panel);

      const gate = deferred();
      cancelGate = gate.promise;
      cancelOutcome = error;
      await panel.clickButton("Full restart");
      await waitForAgentRunPanel(() => cancelRequests === 1);
      await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
      expect(deliveriesOf("x-7"), "nothing is sent into a run whose cancel is pending").toEqual([]);
      await act(async () => {
        gate.release();
      });
      await waitForAgentRunPanel(() => panel.container.textContent?.includes(error) ?? false);

      await waitForAgentRunPanel(() => deliveriesOf("x-7").length === 1);
      expect(deliveriesOf("x-7")[0]).toMatchObject({ runId, prompt: "Handle the peer request" });
      expect(invocationsFor("start_agent_run"), "no replacement run").toHaveLength(1);
      expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([]);
    },
  );

  it.each([
    ["unknown", MESSAGE_RESULT_UNKNOWN],
    ["notApplied", MESSAGE_NOT_APPLIED],
  ])("a turn that ended while a cancel was pending sends the queued exchange only after the cancel is %s", async (_kind, error) => {
    const { panel, runId } = await busyRunWithAQueuedExchange();

    const gate = deferred();
    cancelGate = gate.promise;
    cancelOutcome = error;
    await panel.clickButton("Cancel");
    await waitForAgentRunPanel(() => cancelRequests === 1);
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    expect(deliveriesOf("x-7"), "nothing is sent into a run whose cancel is pending").toEqual([]);
    await act(async () => {
      gate.release();
    });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(error) ?? false);

    await waitForAgentRunPanel(() => deliveriesOf("x-7").length === 1);
    expect(deliveriesOf("x-7")[0]).toMatchObject({ runId, prompt: "Handle the peer request" });
  });

  // Codex r9(apps medium): 결과를 몰랐던(실제로는 적용되지 않은) 재시작 뒤 다시 누른 재시작은 앞 보류를 대체한다. 그 취소의 끝
  // 이벤트가 성공 답보다 먼저 와도 대체 run은 정확히 하나다.
  it("a retried full restart replaces the held one: the cancel end before the success reply starts exactly one run", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await rejectASteer(panel);

    cancelOutcome = MESSAGE_RESULT_UNKNOWN;
    await panel.clickButton("Full restart");
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false);
    expect(invocationsFor("start_agent_run")).toHaveLength(1);

    // 두 번째 취소는 성공한다: 그 run의 끝 이벤트가 답보다 먼저 온다(답을 문으로 붙잡고 끝 이벤트를 먼저 넣는다).
    const gate = deferred();
    cancelGate = gate.promise;
    await panel.clickButton("Full restart");
    await waitForAgentRunPanel(() => cancelRequests === 2);
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    await act(async () => {
      gate.release();
    });
    await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length >= 2);
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length >= 1);
    // 뒤늦은 시작이 있으면 여기까지 온다(같은 run의 끝 이벤트도 한 번 더).
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    await act(async () => {
      await Promise.resolve();
    });
    expect(invocationsFor("start_agent_run"), "exactly one replacement run").toHaveLength(2);
    expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([{ requestId: "x-7" }]);
  });

  // 사용자가 Cancel을 누르면 앞 보류는 버려진다: 그 run이 나중에 취소로 끝나도 재시작하지 않는다.
  it("a cancel replaces a held full restart: the later cancel end does not restart", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await rejectASteer(panel);

    cancelOutcome = MESSAGE_RESULT_UNKNOWN;
    await panel.clickButton("Full restart");
    await waitForAgentRunPanel(() => panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false);

    const gate = deferred();
    cancelGate = gate.promise;
    await panel.clickButton("Cancel");
    await waitForAgentRunPanel(() => cancelRequests === 2);
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "cancelled", message: "cancelled" } });
    await act(async () => {
      gate.release();
    });
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length >= 1);
    await act(async () => {
      await Promise.resolve();
    });
    expect(invocationsFor("start_agent_run"), "the cancelled restart does not come back").toHaveLength(1);
  });

  // Codex r11(apps medium): 교환 항목을 지우고 전달 포기의 답을 기다리는 동안 Full restart가 run을 바꿨다. 포기가 실패해도 그
  // 교환은 끝난 run이 대상이라 새 run의 대기열에 되살리지 않는다(서버가 다른 run으로의 전달을 거절한다 — 되살리면 선두에서 계속
  // 거절돼 뒤 prompt를 막는다). 끝난 run이 대상인 교환은 서버가 세지 않으므로 wait-stop에는 영향이 없다.
  it.each([
    ["refused by the server", "discard refused by the server"],
    ["notApplied", MESSAGE_NOT_APPLIED],
  ])(
    "a discard that fails (%s) after a full restart replaced its run does not revive the exchange in the new run",
    async (_kind, error) => {
      const { panel } = await busyRunWithAQueuedExchange();
      await rejectASteer(panel);

      const gate = deferred();
      discardGate = gate.promise;
      discardOutcome = error;
      await act(async () => {
        queuedPromptButton(1, "제거")?.click();
      });
      await waitForAgentRunPanel(() => !(panel.container.textContent?.includes("Handle the peer request") ?? true));
      await panel.clickButton("Full restart");
      await waitForAgentRunPanel(() => invocationsFor("start_agent_run").length === 2);
      const replacement = (invocationsFor("start_agent_run")[1] as { request: { runId: string } }).request.runId;
      await panel.rerender({ externalPromptRequest: { id: "manual-3", text: "Follow-up work", delivery: "queue" } });
      await waitForAgentRunPanel(() => panel.container.textContent?.includes("Follow-up work") ?? false);
      await act(async () => {
        gate.release();
      });
      await waitForAgentRunPanel(() => panel.container.textContent?.includes(error) ?? false);

      expect(panel.container.textContent, "the old run's exchange is not revived in the new run's queue").not.toContain(
        "Handle the peer request",
      );
      await panel.emitRunEvent({ runId: replacement, event: { type: "lifecycle", status: "promptSent", message: "sent" } });
      await panel.emitRunEvent({
        runId: replacement,
        event: { type: "lifecycle", status: "promptCompleted", message: "done" },
      });
      await waitForAgentRunPanel(() =>
        invocationsFor("send_prompt_to_run").some((args) => (args as { prompt?: string }).prompt === "Follow-up work"),
      );
      expect(deliveriesOf("x-7"), "the old exchange never goes to the replacement run").toEqual([]);
      expect(
        invocationsFor("send_prompt_to_run").find((args) => (args as { prompt?: string }).prompt === "Follow-up work"),
      ).toMatchObject({ runId: replacement });
    },
  );

  // 같은 run이 살아 있는 동안의 포기 실패는 지금처럼 항목을 되돌린다(그 run에 전달할 교환이다).
  it("a discard that fails while its run is still active puts the exchange back", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    discardOutcome = "discard refused by the server";
    await act(async () => {
      queuedPromptButton(1, "제거")?.click();
    });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("discard refused by the server") ?? false);
    expect(panel.container.textContent).toContain("Handle the peer request");
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await waitForAgentRunPanel(() => deliveriesOf("x-7").length === 1);
  });

  // Codex r11: 서버가 교환 전달을 거절하면(대상 run 불일치 등 — 서버의 답인 오류) 그 교환을 대기열 선두에 다시 넣지 않는다.
  // 계속 거절돼 뒤 prompt를 막기 때문이다. 서버에 닿지 않은(notApplied) 전달만 다시 시도한다.
  it("a queued exchange whose delivery the server refuses is dropped instead of blocking later prompts", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await panel.rerender({ externalPromptRequest: { id: "manual-3", text: "Follow-up work", delivery: "queue" } });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("Follow-up work") ?? false);
    const base = invokeMock.getMockImplementation();
    invokeMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (
        command === "send_prompt_to_run" &&
        (args as { continuation?: { exchangeRequestId?: string } } | undefined)?.continuation?.exchangeRequestId === "x-7"
      ) {
        throw "exchange x-7 targets another run";
      }
      return base?.(command, args);
    });

    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("exchange x-7 targets another run") ?? false);
    await waitForAgentRunPanel(() =>
      invocationsFor("send_prompt_to_run").some((args) => (args as { prompt?: string }).prompt === "Follow-up work"),
    );
    expect(deliveriesOf("x-7"), "the refused exchange is tried once, not re-queued").toHaveLength(1);
    expect(panel.container.textContent).not.toContain("Handle the peer request");
  });

  it("a queued exchange whose delivery did not reach the server is retried", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    const base = invokeMock.getMockImplementation();
    let failed = false;
    invokeMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "send_prompt_to_run" && !failed) {
        failed = true;
        throw MESSAGE_NOT_APPLIED;
      }
      return base?.(command, args);
    });

    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await waitForAgentRunPanel(() => deliveriesOf("x-7").length === 2);
  });

  // Codex r12(apps medium): 전송이 서버에 적용돼 그 turn(`promptSent`→`promptCompleted`)까지 관측됐는데 호출 결과가 unknown으로
  // 끝났다. 같은 키로 다시 보내면 서버는 저장된 결과만 재생해 새 lifecycle 이벤트가 없다 — 응답 대기가 영영 풀리지 않아 뒤 항목이
  // 막힌다. 관측된 turn이 있으면 적용된 것으로 보고 다시 보내지 않는다.
  function holdNextSend(matches: (args: Record<string, unknown> | undefined) => boolean, outcome: string) {
    const base = invokeMock.getMockImplementation();
    let armed = true;
    let release!: () => void;
    const held = new Promise<void>((resolve) => {
      release = resolve;
    });
    let reached!: () => void;
    const sent = new Promise<void>((resolve) => {
      reached = resolve;
    });
    invokeMock.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
      if (command === "send_prompt_to_run" && armed && matches(args)) {
        armed = false;
        // 서버가 적용했다(기록·효과) — 답만 잃는다.
        await base?.(command, args);
        reached();
        await held;
        throw outcome;
      }
      return base?.(command, args);
    });
    return { sent, release };
  }

  it("an exchange delivery the server applied but answered unknown is not resent once its turn was seen (Codex r12)", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    const send = holdNextSend(
      (args) => (args as { continuation?: { exchangeRequestId?: string } } | undefined)?.continuation?.exchangeRequestId === "x-7",
      MESSAGE_RESULT_UNKNOWN,
    );

    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await act(async () => {
      await send.sent;
    });
    // 적용된 전달의 turn이 답보다 먼저 관측된다.
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptSent", message: "sent" } });
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await act(async () => {
      send.release();
    });

    await panel.rerender({
      externalPromptRequest: { id: "x-8", text: "Second peer request", delivery: "queue", exchangeRequestId: "x-8" },
    });
    await waitForAgentRunPanel(() => deliveriesOf("x-8").length === 1);
    expect(deliveriesOf("x-7"), "the applied delivery is not replayed as a new turn").toHaveLength(1);
  });

  it("a queued prompt the server applied but answered unknown is not resent once its turn was seen (Codex r12)", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    await act(async () => {
      queuedPromptButton(1, "제거")?.click();
    });
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length === 1);
    await panel.rerender({ externalPromptRequest: { id: "manual-4", text: "Follow-up work", delivery: "queue" } });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("Follow-up work") ?? false);
    const send = holdNextSend((args) => (args as { prompt?: string } | undefined)?.prompt === "Follow-up work", MESSAGE_RESULT_UNKNOWN);

    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await act(async () => {
      await send.sent;
    });
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptSent", message: "sent" } });
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await act(async () => {
      send.release();
    });

    await panel.rerender({ externalPromptRequest: { id: "manual-5", text: "Next work", delivery: "queue" } });
    await waitForAgentRunPanel(() =>
      invocationsFor("send_prompt_to_run").some((args) => (args as { prompt?: string }).prompt === "Next work"),
    );
    expect(
      invocationsFor("send_prompt_to_run").filter((args) => (args as { prompt?: string }).prompt === "Follow-up work"),
      "the applied prompt is not sent again",
    ).toHaveLength(1);
    expect(panel.container.textContent, "the applied prompt stays in the transcript").toContain("Follow-up work");
  });

  it("an exchange delivery answered unknown before any turn was seen is retried with the same key (Codex r12)", async () => {
    const { panel, runId } = await busyRunWithAQueuedExchange();
    const send = holdNextSend(
      (args) => (args as { continuation?: { exchangeRequestId?: string } } | undefined)?.continuation?.exchangeRequestId === "x-7",
      MESSAGE_RESULT_UNKNOWN,
    );

    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await act(async () => {
      await send.sent;
      send.release();
    });
    await waitForAgentRunPanel(() => deliveriesOf("x-7").length === 2);
    const [first, second] = sendOptions
      .filter(
        ({ args }) =>
          (args as { continuation?: { exchangeRequestId?: string } }).continuation?.exchangeRequestId === "x-7",
      )
      .map(({ options }) => (options as { idempotencyKey?: string } | undefined)?.idempotencyKey);
    expect(first, "the exchange carries its delivery key").toBe("exchange-delivery:x-7");
    expect(second, "the retry replays the same key").toBe(first);
    // 적용된 원래 전달의 turn이 늦게 와도 응답 대기가 풀리고 뒤 교환이 전달된다.
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptSent", message: "sent" } });
    await panel.emitRunEvent({ runId, event: { type: "lifecycle", status: "promptCompleted", message: "done" } });
    await panel.rerender({
      externalPromptRequest: { id: "x-8", text: "Second peer request", delivery: "queue", exchangeRequestId: "x-8" },
    });
    await waitForAgentRunPanel(() => deliveriesOf("x-8").length === 1);
  });

  it("still abandons the queued exchange when the cancel succeeds", async () => {
    const { panel } = await busyRunWithAQueuedExchange();
    await panel.clickButton("Cancel");
    await waitForAgentRunPanel(() => invocationsFor("discard_agent_exchange_delivery").length === 1);
    expect(invocationsFor("discard_agent_exchange_delivery")).toEqual([{ requestId: "x-7" }]);
  });
});

async function waitForLoadedSuggestions(container: HTMLElement) {
  await waitForAgentRunPanel(() => {
    const listbox = container.querySelector("[role='listbox']");
    return Boolean(listbox) && !(listbox?.textContent ?? "").includes("Loading commands...");
  });
}

function queuedPromptButton(position: number, action: "제거" | "즉시 전송") {
  return document.querySelector<HTMLButtonElement>(`button[aria-label='${position}번 대기 prompt ${action}']`);
}

function invocationsFor(command: string) {
  return invokeMock.mock.calls
    .filter(([calledCommand]) => calledCommand === command)
    .map(([, args]) => args);
}
