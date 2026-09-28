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
import {
  compatTransport,
  MESSAGE_NOT_APPLIED,
  MESSAGE_RESULT_UNKNOWN,
  setTransport,
} from "@/shared/api/transport";
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

/** 화면의 다음 `run.cancel` 한 번의 호출 결과(Codex r8): `notApplied`는 보내지 않고 "보내지 않음", `unknownNotApplied`는
 *  보내지 않고 "결과 모름", `unknownApplied`는 실제 서버에 보낸 뒤 답을 버리고 "결과 모름"(응답 유실). 그 밖의 호출과
 *  이벤트 스트림은 그대로 실제 서버로 간다. */
type CancelInjection = "notApplied" | "unknownNotApplied" | "unknownApplied";
/** Codex r9: 주입과 함께, 답을 돌려주기 전에 문(`gate`)이 열릴 때까지 붙잡는다. `appliedHeld`는 실제 서버에 보내 적용되게 한 뒤
 *  문이 열리면 실제 답(성공)을 돌려준다 — 끝 이벤트가 답보다 먼저 오는 순서. `held`는 답을 붙잡았다는 표지. */
type HeldCancel = { mode: CancelInjection | "appliedHeld"; gate: Promise<void>; held: () => void };

async function scenario({
  drainFirst = true,
  rejectSteerFirst = false,
}: { drainFirst?: boolean; rejectSteerFirst?: boolean } = {}) {
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
  let nextCancel: CancelInjection | HeldCancel | undefined;
  const cancelsReachingServer: unknown[] = [];
  const screenClient: typeof client = {
    call: async (operation, input, options) => {
      if (operation === ("run.cancel" as OperationId) && typeof nextCancel === "object") {
        const held = nextCancel;
        nextCancel = undefined;
        if (held.mode === "appliedHeld") {
          const outcome = await client.call(operation, input, options);
          cancelsReachingServer.push(input);
          held.held();
          await held.gate;
          return outcome;
        }
        if (held.mode === "unknownApplied") {
          cancelsReachingServer.push(await client.call(operation, input, options));
        }
        held.held();
        await held.gate;
        return held.mode === "notApplied" ? { kind: "notApplied", reason: "offline" } : { kind: "unknown", reason: "lost" };
      }
      if (operation === ("run.cancel" as OperationId) && nextCancel) {
        const mode = nextCancel;
        nextCancel = undefined;
        if (mode === "notApplied") {
          return { kind: "notApplied", reason: "offline" };
        }
        if (mode === "unknownApplied") {
          cancelsReachingServer.push(await client.call(operation, input, options));
        }
        return { kind: "unknown", reason: "lost" };
      }
      if (operation === ("run.cancel" as OperationId)) {
        cancelsReachingServer.push(input);
      }
      return client.call(operation, input, options);
    },
  };
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
  const http = createHttpTransport({ client: screenClient, ensureWindowBench: async () => bench, windowLabel: "session-a", events: network });
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

  if (rejectSteerFirst) {
    // 일반 대기 prompt를 steer로 보내 거절되게 한다(시험 host 엔진은 active-turn steer를 지원하지 않는다) — 비우기 전이라
    // 거절 이유는 "steer 미지원"이다(비우는 중이면 새 작업으로 거절된다).
    await panel.rerender({ externalPromptRequest: { id: "manual-2", text: "Change direction", delivery: "queue" } });
    await waitForAgentRunPanel(() => queuedPromptButton(2, "즉시 전송") !== null, 15_000);
    await act(async () => {
      queuedPromptButton(2, "즉시 전송")?.click();
    });
    await waitForAgentRunPanel(() => panel.container.textContent?.includes("Steer rejected #1") ?? false, 15_000);
    await waitForAgentRunPanel(() => findButton("Full restart") !== null, 15_000);
    expect(panel.container.textContent).toContain("steer unsupported");
  }
  const beginWaitStop = async () => {
    const draining = await owner<{ state: string }>("server.stop", { mode: "wait" });
    expect(draining.state).toBe("drainingWait");
  };
  if (drainFirst) {
    await beginWaitStop();
    const before = await status();
    expect(before.activeWork.busyRuns).toBe(1);
    expect(before.activeWork.pendingExchanges).toBe(1);
  }

  const respondPermission = (runId: string) =>
    call("run.respondPermission", { benchId: bench, runId, permissionId: "p1", optionId: "allow" });
  const finishTurn = () => respondPermission(panelRun);
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
  const injectNextCancel = (mode: CancelInjection) => {
    nextCancel = mode;
  };
  /** 다음 취소 한 번을 붙잡는다(`release`로 답을 돌려준다). `held`는 화면이 취소를 보냈고 답을 기다리는 중일 때 풀린다. */
  const holdNextCancel = (mode: HeldCancel["mode"]) => {
    let release!: () => void;
    let markHeld!: () => void;
    const gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    const held = new Promise<void>((resolve) => {
      markHeld = resolve;
    });
    nextCancel = { mode, gate, held: markHeld };
    return { release, held };
  };
  const sendExchange = async (requestId: string, message: string) => {
    await call("exchange.send", {
      benchId: bench,
      request: { requestId, sourcePanelId: "main", targetPanelId: "p2", message, delivery: "queue" },
    });
    await vi.waitFor(() => expect(acked).toContain(requestId), { timeout: 15_000, interval: 20 });
  };
  const exchangeSendsOf = (requestId: string) =>
    recorded.filter(
      (item) =>
        item.command === "send_prompt_to_run" &&
        (item.args.continuation as { exchangeRequestId?: string } | undefined)?.exchangeRequestId === requestId,
    );
  const benchRuns = async () =>
    (await owner<Array<{ benchId: string; runs?: Array<{ runId: string }> }>>("bench.list", {}))
      .filter((item) => item.benchId === bench)
      .flatMap((item) => (item.runs ?? []).map((run) => run.runId));
  return {
    panel,
    panelRun,
    recorded,
    status,
    finishTurn,
    stopped,
    exchangeSends,
    injectNextCancel,
    holdNextCancel,
    sendExchange,
    exchangeSendsOf,
    benchRuns,
    cancelsReachingServer,
    respondPermission,
    beginWaitStop,
  };
}

function findButton(name: string) {
  return [...document.querySelectorAll("button")].find((button) => button.textContent?.trim() === name) ?? null;
}

async function press(name: string) {
  await act(async () => {
    findButton(name)?.click();
  });
}

describe("real server: AgentRunPanel queue actions on an acknowledged exchange during a wait-stop", () => {
  // Codex r8(apps medium): 거절된 steer의 "Full restart"와 "Cancel"이 취소를 서버에 닿게 하지 못했거나(notApplied) 결과를 모를 때
  // (unknown) 패널은 교환 항목을 버리지도, 새 run을 시작하지도 않는다. run이 살아 있으면 turn 끝에 교환이 이어 가기 표지로
  // 전달되고 wait-stop이 끝난다. 실제로 취소가 적용됐으면(응답만 유실) run 끝 이벤트가 오고 서버는 대상 run이 없는 교환을
  // 세지 않아 역시 wait-stop이 끝난다.
  it.each(["notApplied", "unknownNotApplied"] as const)(
    "full restart whose cancel is %s keeps the exchange, which is delivered after the turn, and the server stops",
    async (mode) => {
      const s = await scenario({ rejectSteerFirst: true });
      s.injectNextCancel(mode);
      await press("Full restart");
      const shown = mode === "notApplied" ? MESSAGE_NOT_APPLIED : MESSAGE_RESULT_UNKNOWN;
      await waitForAgentRunPanel(() => s.panel.container.textContent?.includes(shown) ?? false, 15_000);

      expect(s.cancelsReachingServer, "the injected cancel never reached the server").toEqual([]);
      expect(s.recorded.filter((item) => item.command === "start_agent_run"), "no replacement run").toHaveLength(1);
      expect(s.recorded.filter((item) => item.command === "discard_agent_exchange_delivery")).toEqual([]);
      expect(queuedPromptButton(1, "제거"), "the exchange item is still queued").not.toBeNull();
      const held = await s.status();
      expect(held.activeWork.busyRuns).toBe(1);
      expect(held.activeWork.pendingExchanges).toBe(1);

      // turn이 끝나면 패널이 교환을 이어 가기 표지로 보낸다 → 교환 소비(pendingExchanges 0)·바쁜 run 없음 → 서버 정지.
      await s.finishTurn();
      await s.stopped();
      expect(s.exchangeSends(), "the exchange was delivered once with its continuation").toHaveLength(1);
    },
  );

  // (B) 취소는 적용됐고 응답만 유실됐다. 이미 wait-stop 중이면 취소로 대상 run이 끝나는 순간 활동이 0이 되어 서버가 멈추므로
  // (새 run 시작은 비우는 중 거절되는 새 작업이다), 재시작이 새 run을 정확히 하나 만드는지는 wait-stop을 재시작 뒤에 시작해
  // 확인한다. wait-stop이 먼저인 경우는 아래 다음 시험이다.
  it("full restart whose cancel applied but whose reply was lost restarts exactly once after the recovered run end", async () => {
    const s = await scenario({ drainFirst: false, rejectSteerFirst: true });
    s.injectNextCancel("unknownApplied");
    await press("Full restart");
    await waitForAgentRunPanel(
      () =>
        (s.panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false) ||
        s.recorded.filter((item) => item.command === "start_agent_run").length === 2,
      15_000,
    );
    expect(s.cancelsReachingServer, "the cancel reached the server").toHaveLength(1);

    // 복구된 run 이벤트(취소 끝)가 재시작을 한 번 잇는다: 새 run 정확히 1개, 교환 항목은 버리고 서버에서도 끝낸다.
    await waitForAgentRunPanel(
      () => s.recorded.filter((item) => item.command === "start_agent_run").length === 2,
      15_000,
    );
    await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") === null, 15_000);
    const starts = s.recorded.filter((item) => item.command === "start_agent_run");
    const replacement = starts[1].args.request as { runId: string; goal: string };
    expect(replacement.goal).toContain("Change direction");
    await vi.waitFor(
      () =>
        expect(s.recorded.filter((item) => item.command === "discard_agent_exchange_delivery")).toEqual([
          { command: "discard_agent_exchange_delivery", args: { requestId: "x-1" } },
        ]),
      { timeout: 15_000, interval: 20 },
    );
    expect(s.exchangeSends(), "nothing was sent to the cancelled run").toEqual([]);

    // 새 run의 첫 turn도 권한을 기다린다(시험 host 규칙). wait-stop을 시작하면 교환은 이미 세지 않고, 응답해 turn이 끝나면
    // 바쁜 run이 없어 서버가 멈춘다.
    await vi.waitFor(async () => expect((await s.status()).activeWork.busyRuns).toBe(1), { timeout: 15_000, interval: 20 });
    await s.beginWaitStop();
    expect((await s.status()).activeWork.pendingExchanges).toBe(0);
    await s.respondPermission(replacement.runId);
    await s.stopped();
    expect(s.recorded.filter((item) => item.command === "start_agent_run"), "still exactly one replacement").toHaveLength(2);
  });

  it("full restart whose cancel applied during a wait-stop lets the server stop without a duplicate run", async () => {
    const s = await scenario({ rejectSteerFirst: true });
    s.injectNextCancel("unknownApplied");
    await press("Full restart");
    await vi.waitFor(() => expect(s.cancelsReachingServer, "the cancel reached the server").toHaveLength(1), {
      timeout: 15_000,
      interval: 20,
    });
    // 취소로 대상 run이 끝나 교환은 세지 않고 바쁜 run도 없다: 서버가 멈춘다. 재시작의 새 run은 비우는 중 새 작업이라 서버가
    // 만들지 않는다(정상 — 우회하지 않는다). 패널: 끝 이벤트가 대기열(교환 항목)을 정리하고, 재시작 시도는 많아야 한 번이며
    // 서버가 거절하면 오류와 거절된 steer가 남는다.
    await s.stopped();
    await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") === null, 15_000);
    expect(s.exchangeSends(), "nothing was sent to the cancelled run").toEqual([]);
    const starts = s.recorded.filter((item) => item.command === "start_agent_run");
    expect(starts.length, "at most one restart attempt").toBeLessThanOrEqual(2);
    if (starts.length === 2) {
      await waitForAgentRunPanel(() => s.panel.container.textContent?.includes("Steer rejected #1") ?? false, 15_000);
    }
  });

  // Codex r9(apps medium): 재시작 취소의 답을 기다리는 동안 새 교환이 도착해 패널 대기열에 들어가고 서버에 확인됐다. 취소가
  // 적용되지 않고 결과를 모름으로 끝나면, 패널은 호출 전 대기열로 덮어써 그 교환을 잃지 않는다 — turn이 끝나면 두 교환 모두
  // 이어 가기 표지로 전달되고 wait-stop이 끝난다.
  it("an exchange that arrived while a full restart's cancel was pending survives an unknown result and is delivered", async () => {
    const s = await scenario({ drainFirst: false, rejectSteerFirst: true });
    const hold = s.holdNextCancel("unknownNotApplied");
    await press("Full restart");
    await hold.held;
    await s.sendExchange("x-2", "second peer message");
    await waitForAgentRunPanel(() => s.panel.container.textContent?.includes("second peer message") ?? false, 15_000);
    await act(async () => {
      hold.release();
    });
    await waitForAgentRunPanel(() => s.panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false, 15_000);

    expect(s.cancelsReachingServer, "the injected cancel never reached the server").toEqual([]);
    expect(s.panel.container.textContent, "the exchange that arrived during the cancel is still queued").toContain(
      "second peer message",
    );
    expect(s.recorded.filter((item) => item.command === "discard_agent_exchange_delivery")).toEqual([]);
    await s.beginWaitStop();
    const held = await s.status();
    expect(held.activeWork.pendingExchanges).toBe(2);

    await s.finishTurn();
    await s.stopped();
    expect(s.exchangeSendsOf("x-1"), "the first exchange was delivered once").toHaveLength(1);
    expect(s.exchangeSendsOf("x-2"), "the exchange that arrived during the cancel was delivered once").toHaveLength(1);
    expect(s.recorded.filter((item) => item.command === "start_agent_run"), "no replacement run").toHaveLength(1);
  });

  // Codex r9(apps medium): 결과를 몰랐던(적용되지 않은) 재시작 뒤 다시 누른 재시작은 앞 보류를 대체한다. 두 번째 취소는 실제로
  // 적용되고, 그 run의 끝 이벤트가 성공 답보다 먼저 온다. 대체 run은 정확히 하나다(서버의 run 목록으로 확인).
  it("a retried full restart after an unknown one starts exactly one replacement run when the cancel end precedes the reply", async () => {
    const s = await scenario({ drainFirst: false, rejectSteerFirst: true });
    s.injectNextCancel("unknownNotApplied");
    await press("Full restart");
    await waitForAgentRunPanel(() => s.panel.container.textContent?.includes(MESSAGE_RESULT_UNKNOWN) ?? false, 15_000);
    expect(s.recorded.filter((item) => item.command === "start_agent_run")).toHaveLength(1);

    const hold = s.holdNextCancel("appliedHeld");
    await press("Full restart");
    await hold.held;
    // 서버가 취소를 적용했다: 끝 이벤트가 패널에 와서 대기열을 비울 때까지 답을 붙잡는다.
    await waitForAgentRunPanel(() => queuedPromptButton(1, "제거") === null, 15_000);
    await act(async () => {
      hold.release();
    });
    await waitForAgentRunPanel(
      () => s.recorded.filter((item) => item.command === "start_agent_run").length >= 2,
      15_000,
    );
    const replacement = s.recorded.filter((item) => item.command === "start_agent_run")[1].args.request as {
      runId: string;
    };
    await vi.waitFor(async () => expect((await s.status()).activeWork.busyRuns).toBe(1), { timeout: 15_000, interval: 20 });
    await vi.waitFor(
      async () => expect((await s.benchRuns()).sort()).toEqual(["source", replacement.runId].sort()),
      { timeout: 15_000, interval: 20 },
    );
    expect(s.recorded.filter((item) => item.command === "start_agent_run"), "exactly one replacement").toHaveLength(2);

    await s.beginWaitStop();
    expect((await s.status()).activeWork.pendingExchanges).toBe(0);
    await s.respondPermission(replacement.runId);
    await s.stopped();
    expect(s.recorded.filter((item) => item.command === "start_agent_run"), "still exactly one replacement").toHaveLength(2);
    expect(s.exchangeSends(), "nothing was sent to the cancelled run").toEqual([]);
  });

  it("cancel whose request was not applied keeps the run and its exchange; the exchange is delivered and the server stops", async () => {
    const s = await scenario();
    s.injectNextCancel("notApplied");
    await press("Cancel");
    await waitForAgentRunPanel(() => s.panel.container.textContent?.includes(MESSAGE_NOT_APPLIED) ?? false, 15_000);
    expect(s.cancelsReachingServer).toEqual([]);
    expect(s.recorded.filter((item) => item.command === "discard_agent_exchange_delivery")).toEqual([]);
    expect(queuedPromptButton(1, "제거")).not.toBeNull();
    expect((await s.status()).activeWork.pendingExchanges).toBe(1);

    await s.finishTurn();
    await s.stopped();
    expect(s.exchangeSends()).toHaveLength(1);
  });

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
