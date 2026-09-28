// @vitest-environment happy-dom
// 044 Codex r7(apps medium): **실제 AgentRunPanel**을 실제 시험 host(042 router + 실제 런타임 + 감시 루프)에 앱 transport
// (`createHttpTransport` → command 표)와 네트워크 이벤트 계층으로 붙인다. 교환은 043 원장(`createExchangeReconciler`)이 라우팅·
// 확인하고, 라우팅은 작업 영역 화면과 같이 패널의 `externalPromptRequest`로 넣는다(worktree-agent-run-area의 `route`). 패널
// 액션(대기 prompt 제거·steer 시도)은 사용자가 누르는 버튼을 그대로 누른다.
//
// 흐름: 패널 run의 첫 turn이 권한을 기다리는 동안(바쁨) 교환이 온다 → 원장이 패널 대기열에 넣고 확인 → 소유자가 wait-stop →
// (a) 제거: 패널의 제거 버튼 → 서버 전달 포기(C) → `pendingExchanges` 0 → 권한 응답으로 turn 끝 → 서버 정지. 지운 교환은 run에
// 가지 않는다. (b) steer: 교환 항목의 즉시 전송은 막혀 있다 → turn이 끝나면 패널 자동 전송이 이어 가기 표지로 보낸다(K) →
// 서버 정지.
import { createEventClient, createWorkbenchClient, type OperationId } from "@yoophi/workbench-client";
import { connectTo, startHost, type HostInfo } from "@yoophi/workbench-client/test-host";
import { act } from "react";
import { afterEach, describe, expect, it, onTestFailed, vi } from "vitest";

import { createExchangeReconciler } from "@/features/agent-run/model/exchange-reconciler";
import { createHttpTransport } from "@/shared/api/transport/http-transport";
import { compatTransport, setTransport } from "@/shared/api/transport";
import type { Transport } from "@/shared/api/transport/transport";
import {
  createNetworkEvents,
  EXCHANGE_REQUESTED_EVENT,
  EXCHANGE_STATUS_EVENT,
} from "@/shared/api/transport/network-events";

import {
  cleanupAgentRunPanelTests,
  renderAgentRunPanel,
  waitForAgentRunPanel,
} from "./agent-run-panel.test-harness";

let cleanup: Array<() => Promise<void> | void> = [];
afterEach(async () => {
  await cleanupAgentRunPanelTests();
  for (const step of cleanup.reverse()) {
    await step();
  }
  cleanup = [];
});

type Recorded = { command: string; args: Record<string, unknown> };

function queuedPromptButton(position: number, action: "제거" | "즉시 전송") {
  return document.querySelector<HTMLButtonElement>(`button[aria-label='${position}번 대기 prompt ${action}']`);
}

async function scenario() {
  // 화면은 브라우저 환경(happy-dom)의 fetch·WebSocket으로 붙는다 — 데스크톱 webview처럼 출처를 싣고, 서버 허용 출처에 있다.
  // turn 경계 lifecycle(`promptSent`·`promptCompleted`)은 실제 ACP runner처럼 낸다 — 시험 host의 엔진은 scripted engine이다.
  const host = await startHost({
    HOST_PERMISSION_ID: "p1",
    HOST_ALLOWED_ORIGINS: window.location.origin,
    HOST_PROMPT_LIFECYCLE: "1",
  });
  cleanup.push(() => host.stop());
  // 실패 진단: host의 접근 기록(stderr — 거절된 출처·WebSocket 업그레이드 등)을 실패한 시험에만 보인다.
  const hostLog: string[] = [];
  host.process.stderr.on("data", (chunk: Buffer) => hostLog.push(chunk.toString("utf8")));
  onTestFailed(() => {
    console.error(`--- host stderr ---\n${hostLog.join("")}`);
  });
  const target = { current: host as HostInfo };
  const windowConnection = await connectTo(target);
  cleanup.push(() => windowConnection.close());
  const ownerConnection = await connectTo(target, "owner");
  cleanup.push(() => ownerConnection.close());
  const client = createWorkbenchClient({ connection: windowConnection });
  const ownerClient = createWorkbenchClient({ connection: ownerConnection });
  const callWith =
    (via: typeof client) =>
    async <T = Record<string, unknown>>(operation: string, input: unknown): Promise<T> => {
      const outcome = await via.call(operation as OperationId, input as never);
      if (outcome.kind !== "ok") {
        throw new Error(`${operation}: ${JSON.stringify(outcome)}`);
      }
      return outcome.output as T;
    };
  const call = callWith(client);
  const owner = callWith(ownerClient);
  const status = () => owner<{ state: string; activeWork: Record<string, number> }>("server.status", {});

  const bench = (await call<{ benchId: string }>("bench.open", { workingDirectory: host.workDir })).benchId;
  const events = createEventClient({ connection: windowConnection, graceMs: 0 });
  cleanup.push(() => events.close());
  const network = createNetworkEvents({ events, client });
  network.noteBench(bench);
  const http = createHttpTransport({ client, ensureWindowBench: async () => bench, windowLabel: "session-a", events: network });
  // 패널이 보낸 command를 기록한다(전송은 그대로 실제 서버로).
  const recorded: Recorded[] = [];
  const recording: Transport = {
    kind: http.kind,
    invoke: (command, args, options) => {
      recorded.push({ command, args: args ?? {} });
      return http.invoke(command, args, options);
    },
    listen: (event, callback) => http.listen(event, callback),
  };
  setTransport(recording);
  cleanup.push(() => setTransport(compatTransport));

  // 교환을 보내는 쪽 run(쉬는 상태).
  await call("run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId: "source" } });
  await call("run.respondPermission", { benchId: bench, runId: "source", permissionId: "p1", optionId: "allow" });

  // 실제 패널: 첫 prompt로 run을 시작한다. 첫 turn은 권한을 기다린다(바쁨).
  const panel = await renderAgentRunPanel({
    panelId: "p2",
    workingDirectory: host.workDir,
    externalPromptRequest: { id: "start-1", text: "Work on the task", delivery: "send" },
  });
  await waitForAgentRunPanel(() => recorded.some((item) => item.command === "start_agent_run"), 15_000);
  const start = recorded.find((item) => item.command === "start_agent_run");
  const panelRun = (start?.args.request as { runId: string }).runId;
  await vi.waitFor(async () => expect((await status()).activeWork.busyRuns).toBe(1), { timeout: 15_000, interval: 20 });

  await owner("lease.acquire", { clientKind: "desktop", clientId: "app" });
  await call("exchange.syncWorkspace", {
    benchId: bench,
    request: {
      worktreePath: host.workDir,
      revision: 1,
      focusedPanelId: "main",
      panels: [
        { panelId: "main", title: "Main", runId: "source", status: "running" },
        { panelId: "p2", title: "Panel", runId: panelRun, status: "running" },
      ],
    },
  });

  // 작업 영역 화면의 라우팅과 같다: 원장이 대상 패널의 `externalPromptRequest`로 넣고 확인한다.
  const acked: string[] = [];
  const reconciler = createExchangeReconciler({
    route: (request) => {
      void panel.rerender({
        externalPromptRequest: {
          id: request.requestId,
          text: request.message,
          delivery: request.delivery,
          exchangeRequestId: request.requestId,
        },
      });
      return { routed: true };
    },
    acknowledge: async (ack) => {
      await call("exchange.acknowledge", { benchId: bench, request: ack });
      acked.push(ack.requestId);
    },
  });
  await network.listen(EXCHANGE_REQUESTED_EVENT, (payload) => reconciler.handleRequested(payload as never));
  await network.listen(EXCHANGE_STATUS_EVENT, (payload) => reconciler.observeStatus(payload as never));

  await call("exchange.send", {
    benchId: bench,
    request: { requestId: "x-1", sourcePanelId: "main", targetPanelId: "p2", message: "hello peer", delivery: "queue" },
  });
  await vi.waitFor(() => expect(acked).toEqual(["x-1"]), { timeout: 15_000, interval: 20 });
  await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") !== null, 15_000);

  const draining = await owner<{ state: string }>("server.stop", { mode: "wait" });
  expect(draining.state).toBe("drainingWait");
  const before = await status();
  expect(before.activeWork.busyRuns).toBe(1);
  expect(before.activeWork.pendingExchanges).toBe(1);

  const finishTurn = () =>
    call("run.respondPermission", { benchId: bench, runId: panelRun, permissionId: "p1", optionId: "allow" });
  const stopped = () =>
    vi.waitFor(
      async () => {
        const outcome = await ownerClient.call("server.status" as OperationId, {} as never);
        expect(outcome.kind === "fault" ? outcome.fault.code : outcome.kind).toBe("unavailable");
      },
      { timeout: 15_000, interval: 50 },
    );
  const exchangeSends = () =>
    recorded.filter(
      (item) =>
        item.command === "send_prompt_to_run" &&
        (item.args.continuation as { exchangeRequestId?: string } | undefined)?.exchangeRequestId === "x-1",
    );
  return { panelRun, recorded, status, finishTurn, stopped, exchangeSends };
}

describe("real server: AgentRunPanel queue actions on an acknowledged exchange during a wait-stop", () => {
  it("removing the queued exchange discards it on the server, so the wait-stop completes after the turn", async () => {
    const s = await scenario();
    await act(async () => {
      queuedPromptButton(1, "제거")?.click();
    });
    await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") === null, 15_000);
    await vi.waitFor(async () => expect((await s.status()).activeWork.pendingExchanges).toBe(0), {
      timeout: 15_000,
      interval: 20,
    });
    expect(s.recorded.filter((item) => item.command === "discard_agent_exchange_delivery")).toEqual([
      { command: "discard_agent_exchange_delivery", args: { requestId: "x-1" } },
    ]);

    await s.finishTurn();
    await s.stopped();
    expect(s.exchangeSends(), "the discarded exchange never reached the run").toEqual([]);
  });

  it("does not steer a queued exchange; the panel sends it with its continuation after the turn and the server stops", async () => {
    const s = await scenario();
    const steer = queuedPromptButton(1, "즉시 전송");
    expect(steer?.disabled, "an exchange item cannot be steered").toBe(true);
    await act(async () => {
      steer?.click();
    });
    expect(s.recorded.filter((item) => item.command === "steer_prompt_to_run")).toEqual([]);
    expect((await s.status()).activeWork.pendingExchanges).toBe(1);

    await s.finishTurn();
    await s.stopped();
    expect(s.exchangeSends(), "the panel queue delivered the exchange once with its continuation").toHaveLength(1);
  });
});
