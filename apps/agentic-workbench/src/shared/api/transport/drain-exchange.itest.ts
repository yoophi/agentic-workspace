// 044 T043(research R7 검증 3(b), Codex 설계 리뷰 C2·E2): wait-stop 중 교환 전달을 **043 화면 코드**로 끝낸다. 실제 시험 host
// (042 router + 실제 런타임 + 감시 루프)에 네트워크 이벤트 계층(`createNetworkEvents`)과 교환 원장(`createExchangeReconciler`)을
// 붙이고, 패널 대기열의 전송은 앱 transport(`createHttpTransport` → command 표) 위의 `sendPromptToRun`으로 보낸다.
//
// 흐름: 대상 run이 바쁜 동안(권한 대기) 교환 요청 → 원장이 라우팅하고 **전송보다 먼저 확인** → 소유자가 wait-stop → 비우는
// 중에는 이어 가기 표지 없는 prompt가 거절된다(새 작업) → 권한 응답(C)으로 turn이 끝나면 패널 대기열이
// `continuation.exchangeRequestId`를 실어 보낸다(K) → 서버가 교환마다 한 번 받아 전달·소비 → 활동 작업 0 → 멈춤.
// 데스크톱 임대가 있어 미소비 교환은 전달될 때까지 활동 작업이다.
import { createEventClient, createWorkbenchClient, type OperationId } from "@yoophi/workbench-client";
import { connectTo, startHost, type HostInfo } from "@yoophi/workbench-client/test-host";
import { afterEach, describe, expect, it, vi } from "vitest";

import { sendPromptToRun } from "@/entities/agent-run/api/agent-run-repository";
import {
  createExchangeReconciler,
  exchangeContinuation,
  exchangeDeliveryKey,
} from "@/features/agent-run/model/exchange-reconciler";

import { createHttpTransport } from "./http-transport";
import { compatTransport, setTransport } from "./index";
import { createNetworkEvents, EXCHANGE_REQUESTED_EVENT, EXCHANGE_STATUS_EVENT } from "./network-events";

let cleanup: Array<() => Promise<void> | void> = [];
afterEach(async () => {
  for (const step of cleanup.reverse()) {
    await step();
  }
  cleanup = [];
});

interface QueuedDelivery {
  runId: string;
  text: string;
  idempotencyKey: string;
  exchangeRequestId: string;
}

describe("real server: exchange delivery during a wait-stop through the 043 consumer code", () => {
  it("acknowledges before sending, delivers with a continuation after the turn, and then the server stops", async () => {
    const host = await startHost({ HOST_PERMISSION_ID: "p1" });
    cleanup.push(() => host.stop());
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
    const respond = (bench: string, runId: string) =>
      call("run.respondPermission", { benchId: bench, runId, permissionId: "p1", optionId: "allow" });

    const bench = (await call<{ benchId: string }>("bench.open", { workingDirectory: host.workDir })).benchId;
    for (const runId of ["r1", "r2"]) {
      await call("run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId } });
    }
    await respond(bench, "r1"); // r1은 쉬고, r2는 첫 turn이 권한을 기다린다(바쁨).
    await owner("lease.acquire", { clientKind: "desktop", clientId: "app" });
    const panelRuns: Record<string, string> = { main: "r1", p2: "r2" };
    await call("exchange.syncWorkspace", {
      benchId: bench,
      request: {
        worktreePath: host.workDir,
        revision: 1,
        focusedPanelId: "main",
        panels: [
          { panelId: "main", title: "Main", runId: "r1", status: "running" },
          { panelId: "p2", title: "Panel", runId: "r2", status: "running" },
        ],
      },
    });

    // 화면: 원장이 대상 패널 대기열에 넣고(라우팅) 확인한다. 패널 대기열은 run이 쉴 때 보낸다.
    const order: string[] = [];
    const panelQueue: QueuedDelivery[] = [];
    const reconciler = createExchangeReconciler({
      route: (request) => {
        panelQueue.push({
          runId: panelRuns[request.target.panelId],
          text: request.message,
          idempotencyKey: exchangeDeliveryKey(request.requestId),
          exchangeRequestId: request.requestId,
        });
        return { routed: true };
      },
      acknowledge: async (ack) => {
        await call("exchange.acknowledge", { benchId: bench, request: ack });
        order.push(`ack:${ack.requestId}`);
      },
    });
    const events = createEventClient({ connection: windowConnection, graceMs: 0 });
    cleanup.push(() => events.close());
    const network = createNetworkEvents({ events, client });
    await network.listen(EXCHANGE_REQUESTED_EVENT, (payload) => reconciler.handleRequested(payload as never));
    await network.listen(EXCHANGE_STATUS_EVENT, (payload) => reconciler.observeStatus(payload as never));
    network.noteBench(bench);
    setTransport(
      createHttpTransport({ client, ensureWindowBench: async () => bench, windowLabel: "session-a", events: network }),
    );
    cleanup.push(() => setTransport(compatTransport));

    await call("exchange.send", {
      benchId: bench,
      request: { requestId: "x-1", sourcePanelId: "main", targetPanelId: "p2", message: "hello peer", delivery: "queue" },
    });
    await vi.waitFor(() => expect(order).toEqual(["ack:x-1"]), { timeout: 15_000, interval: 20 });
    expect(panelQueue).toHaveLength(1);

    const draining = await owner<{ state: string }>("server.stop", { mode: "wait" });
    expect(draining.state).toBe("drainingWait");
    const status = await owner<{ state: string; activeWork: Record<string, number> }>("server.status", {});
    expect(status.state).toBe("drainingWait");
    expect(status.activeWork.busyRuns).toBe(1);
    expect(status.activeWork.pendingExchanges).toBe(1);

    // 이어 가기 표지가 없는 prompt는 비우는 중 새 작업이다(043 이전 전송 모양).
    await expect(sendPromptToRun("r2", "a new prompt")).rejects.toBeDefined();

    await respond(bench, "r2");
    await vi.waitFor(
      async () => {
        const current = await owner<{ activeWork: Record<string, number> }>("server.status", {});
        expect(current.activeWork.busyRuns).toBe(0);
      },
      { timeout: 15_000, interval: 20 },
    );
    // 패널 대기열 전송: 앱 저장소 → transport → command 표(`continuation` 포함).
    for (const delivery of panelQueue.splice(0)) {
      await sendPromptToRun(
        delivery.runId,
        delivery.text,
        { idempotencyKey: delivery.idempotencyKey },
        exchangeContinuation(delivery.exchangeRequestId),
      );
      order.push(`send:${delivery.exchangeRequestId}`);
    }
    expect(order).toEqual(["ack:x-1", "send:x-1"]);

    // 멈추면(stopping) 새 호출은 `unavailable`이다.
    await vi.waitFor(
      async () => {
        const outcome = await ownerClient.call("server.status" as OperationId, {} as never);
        expect(outcome.kind === "fault" ? outcome.fault.code : outcome.kind).toBe("unavailable");
      },
      { timeout: 15_000, interval: 50 },
    );
  });
});
