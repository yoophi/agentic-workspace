// 043 T024: HttpTransport는 command 이름·인자를 받아 compat과 같은 결과·오류를 돌려준다. 작업대는 `ensure_window_bench`로
// 얻고(`ensure`면 열고, `lookup`이면 있을 때만), 작업대가 없을 때의 결과·오류, 결과 변환(orchestration 묶임 표시),
// 오류 문자열(교환·orchestration), 항상 성공하는 command(`list_agents`)가 compat Rust와 같다.
import type { CallOutcome, WorkbenchClient } from "@yoophi/workbench-client";
import { describe, expect, it, vi } from "vitest";

import { createHttpTransport, MESSAGE_NOT_APPLIED, MESSAGE_RESULT_UNKNOWN } from "./http-transport";

type Call = { operation: string; input: unknown; key?: string };

function setup(reply: (call: Call) => CallOutcome<unknown>, bench: string | null = "bench-1") {
  const calls: Call[] = [];
  const client: WorkbenchClient = {
    call: vi.fn(async (operation: string, input: unknown, options?: { idempotencyKey?: string }) => {
      const call = { operation, input, key: options?.idempotencyKey };
      calls.push(call);
      return reply(call);
    }) as unknown as WorkbenchClient["call"],
  };
  const ensureWindowBench = vi.fn(async (open: boolean, _hint?: string | null) => (open ? "bench-1" : bench));
  const transport = createHttpTransport({ client, ensureWindowBench, windowLabel: "session-7" });
  return { transport, calls, ensureWindowBench };
}

const ok = (output: unknown): CallOutcome<unknown> => ({ kind: "ok", output, revision: undefined });
const fault = (message: string, details?: unknown): CallOutcome<unknown> => ({
  kind: "fault",
  fault: { code: "conflict", message, retryable: false, outcome: "notApplied", requestId: "r", details: details as never },
});

describe("createHttpTransport", () => {
  it("sends global commands without a bench and returns the output", async () => {
    const { transport, calls, ensureWindowBench } = setup(() => ok([{ id: "p1" }]));
    expect(await transport.invoke("list_projects")).toEqual([{ id: "p1" }]);
    expect(calls).toEqual([{ operation: "project.list", input: {}, key: undefined }]);
    expect(ensureWindowBench).not.toHaveBeenCalled();
  });

  it("opens the window bench for ensure commands with the command hint", async () => {
    const { transport, calls, ensureWindowBench } = setup(() => ok({ id: "r1" }));
    await transport.invoke("start_agent_run", { request: { goal: "g", agentId: "codex", cwd: "/w" }, panelId: "main" });
    expect(ensureWindowBench).toHaveBeenCalledWith(true, "/w");
    expect(calls[0].input).toEqual({ benchId: "bench-1", request: { goal: "g", agentId: "codex", cwd: "/w" }, panelId: "main" });
  });

  it("returns today's results and errors when a lookup command has no bench", async () => {
    const { transport, calls } = setup(() => ok("unused"), null);
    expect(await transport.invoke("list_agent_exchanges")).toEqual([]);
    expect(await transport.invoke("get_orchestration_workspace")).toBeNull();
    expect(await transport.invoke("collect_orchestration_reports")).toEqual([]);
    expect(await transport.invoke("cancel_agent_run", { runId: "r1" })).toBeNull();
    expect(await transport.invoke("replay_orchestration_runtime_events", { input: { runId: "r1", afterSequence: 2 } })).toEqual({
      runId: "r1",
      events: [],
      lastSequence: 0,
      terminal: false,
      gapDetected: true,
    });
    await expect(transport.invoke("send_prompt_to_run", { runId: "r1", prompt: "p" })).rejects.toBe("agent run is not active");
    await expect(transport.invoke("respond_agent_permission", { runId: "r9", permissionId: "p", optionId: "o" })).rejects.toBe(
      "unknown or finished run: r9",
    );
    await expect(transport.invoke("send_agent_exchange", { request: {} })).rejects.toBe(
      '{"code":"unknownWorkspace","message":"Agent workspace is not registered."}',
    );
    await expect(transport.invoke("acknowledge_agent_exchange", { request: {} })).rejects.toBe(
      '{"code":"unknownExchange","message":"Exchange was not found."}',
    );
    expect(calls).toHaveLength(0);
  });

  it("marks bound orchestration sessions with the window label and drops the stream id", async () => {
    const { transport } = setup(() => ok({ id: "ws1", revision: 3, eventStreamId: "orchestration:b1" }));
    expect(await transport.invoke("bootstrap_orchestration_workspace", { input: { worktreePath: "/w" } })).toEqual({
      id: "ws1",
      revision: 3,
      boundWindowLabel: "session-7",
    });
    const empty = setup(() => ok(null));
    expect(await empty.transport.invoke("get_orchestration_workspace")).toBeNull();
  });

  it("uses the compat fault strings per command flavor", async () => {
    const exchange = setup(() => fault("Target panel is closing.", { exchangeCode: "targetClosing" }));
    await expect(exchange.transport.invoke("send_agent_exchange", { request: {} })).rejects.toBe(
      '{"code":"targetClosing","message":"Target panel is closing."}',
    );
    const orchestration = setup(() =>
      fault("wrapped", { orchestrationError: { code: "notFound", message: "Orchestration workspace is not bootstrapped." } }),
    );
    await expect(orchestration.transport.invoke("recover_orchestration_workspace")).rejects.toBe(
      '{"code":"notFound","message":"Orchestration workspace is not bootstrapped."}',
    );
    const plain = setup(() => fault("Project name is taken."));
    await expect(plain.transport.invoke("delete_project", { id: "p1" })).rejects.toBe("Project name is taken.");
  });

  it("keeps commands that never fail today and replay's Missing fallback", async () => {
    const agents = setup(() => fault("catalog failed"));
    expect(await agents.transport.invoke("list_agents")).toEqual([]);
    const replay = setup(() => fault("run is owned by another bench."));
    expect(await replay.transport.invoke("replay_orchestration_runtime_events", { input: { runId: "r1", afterSequence: 0 } })).toEqual({
      runId: "r1",
      events: [],
      lastSequence: 0,
      terminal: false,
      gapDetected: false,
    });
  });

  it("reports offline and unknown outcomes with their own messages", async () => {
    const offline = setup(() => ({ kind: "notApplied", reason: "offline" }));
    await expect(offline.transport.invoke("delete_project", { id: "p1" })).rejects.toBe(MESSAGE_NOT_APPLIED);
    const unknown = setup(() => ({ kind: "unknown", reason: "epochChanged" }));
    await expect(unknown.transport.invoke("delete_project", { id: "p1" })).rejects.toBe(MESSAGE_RESULT_UNKNOWN);
  });

  it("passes a caller idempotency key through to the client", async () => {
    const { transport, calls } = setup(() => ok(null));
    await transport.invoke("send_prompt_to_run", { runId: "r1", prompt: "hi" }, { idempotencyKey: "exchange-delivery:x1" });
    expect(calls[0].key).toBe("exchange-delivery:x1");
  });

  it("rejects commands that are not server-owned instead of guessing", async () => {
    const { transport } = setup(() => ok(null));
    await expect(transport.invoke("get_appearance_preferences")).rejects.toThrow(/not a server-owned command/);
  });
});
