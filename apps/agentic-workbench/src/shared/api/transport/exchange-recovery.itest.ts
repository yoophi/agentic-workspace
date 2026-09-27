// 043 T041(사용자 검토): 요청 이벤트를 잃은 교환이 실제 042 서버에서 **화면의 재조정 경로**(네트워크 이벤트 계층 + 교환
// 원장)를 거쳐 라우팅 1회·확인 1회로 전달된다. 스냅샷 내용만 보는 것이 아니라 실제 `exchange.acknowledge`로 서버 상태를
// `delivered`로 바꾼다. 구독 전에 교환 3건(요청·상태 이벤트 6건)을 보내 보관 한도(4)를 넘기므로 첫 요청 이벤트는 journal에
// 없다 — 구독이 보관 gap 복구와 구독 시작 재조정을 거쳐 맞춘다. 다시 연결해도 추가 라우팅·확인은 없다.
import { createEventClient, createWorkbenchClient, type OperationId } from "@yoophi/workbench-client";
import { connectTo, startHost, type HostInfo } from "@yoophi/workbench-client/test-host";
import { afterEach, describe, expect, it, vi } from "vitest";

import { createExchangeReconciler } from "@/features/agent-run/model/exchange-reconciler";

import { createNetworkEvents, EXCHANGE_REQUESTED_EVENT, EXCHANGE_STATUS_EVENT } from "./network-events";

let cleanup: Array<() => Promise<void> | void> = [];
afterEach(async () => {
  for (const step of cleanup.reverse()) {
    await step();
  }
  cleanup = [];
});

describe("real 042 server: lost exchange requests through the screen reconciliation path", () => {
  it("routes and acknowledges each lost request exactly once and not again after a reconnect", async () => {
    const host = await startHost({ HOST_JOURNAL_CAPACITY: "4" });
    cleanup.push(() => host.stop());
    const connection = await connectTo({ current: host as HostInfo });
    cleanup.push(() => connection.close());
    const client = createWorkbenchClient({ connection });
    const call = async <T = Record<string, unknown>>(operation: string, input: unknown): Promise<T> => {
      const outcome = await client.call(operation as OperationId, input as never);
      if (outcome.kind !== "ok") {
        throw new Error(`${operation}: ${JSON.stringify(outcome)}`);
      }
      return outcome.output as T;
    };
    const bench = (await call<{ benchId: string }>("bench.open", { workingDirectory: host.workDir })).benchId;
    for (const runId of ["r1", "r2"]) {
      await call("run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId } });
    }
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
    // 화면이 구독하기 전에(요청 이벤트를 받을 수 없는 동안) 교환 3건.
    for (const index of [1, 2, 3]) {
      await call("exchange.send", {
        benchId: bench,
        request: { requestId: `x-${index}`, sourcePanelId: "main", targetPanelId: "p2", message: `m${index}`, delivery: "queue" },
      });
    }

    const routed: string[] = [];
    const acknowledged: string[] = [];
    const reconciler = createExchangeReconciler({
      route: (request) => {
        routed.push(request.requestId);
        return { routed: true };
      },
      acknowledge: async (ack) => {
        await call("exchange.acknowledge", { benchId: bench, request: ack });
        acknowledged.push(ack.requestId);
      },
    });
    const events = createEventClient({ connection, graceMs: 0 });
    cleanup.push(() => events.close());
    // 재조정 적용 표식: 스냅샷 조회 응답이 **해결된** 횟수(요청이 아니라 응답 도착 뒤에 센다).
    let snapshotsResolved = 0;
    const countingClient = {
      call: (async (operation: OperationId, input: never, options?: never) => {
        const outcome = await client.call(operation, input, options);
        if (operation === ("exchange.list" as OperationId)) {
          snapshotsResolved += 1;
        }
        return outcome;
      }) as typeof client.call,
    };
    const network = createNetworkEvents({ events, client: countingClient });
    await network.listen(EXCHANGE_REQUESTED_EVENT, (payload) => reconciler.handleRequested(payload as never));
    // 상태 수신자의 적용 기록(재설정은 스냅샷의 교환마다 이 콜백을 await한다).
    const statusApplied: string[] = [];
    await network.listen(EXCHANGE_STATUS_EVENT, (payload) => {
      reconciler.observeStatus(payload as never);
      statusApplied.push((payload as { requestId: string }).requestId);
    });
    network.noteBench(bench);

    await vi.waitFor(async () => {
      const list = await call<Array<{ requestId: string; status: string }>>("exchange.list", { benchId: bench });
      expect(list.map((item) => item.status)).toEqual(["delivered", "delivered", "delivered"]);
    }, { timeout: 15_000, interval: 50 });
    expect(routed.slice().sort()).toEqual(["x-1", "x-2", "x-3"]);
    expect(acknowledged.slice().sort()).toEqual(["x-1", "x-2", "x-3"]);

    // 같은 세대에서 다시 연결: 재연결 재조정(수신자 둘 → 스냅샷 조회 2회)이 끝난 뒤에도 추가 라우팅·확인이 없다
    // (서버가 이미 delivered).
    const resolvedBefore = snapshotsResolved;
    const appliedBefore = statusApplied.length;
    expect(events.debugDropSockets()).toBeGreaterThan(0);
    // (1) 두 수신자의 재설정 스냅샷 응답이 도착했고 (2) 상태 수신자의 재설정이 세 교환 모두의 콜백을 끝냈다.
    await vi.waitFor(() => {
      expect(snapshotsResolved).toBeGreaterThanOrEqual(resolvedBefore + 2);
      expect(new Set(statusApplied.slice(appliedBefore))).toEqual(new Set(["x-1", "x-2", "x-3"]));
    }, { timeout: 10_000 });
    // 요청 수신자의 재설정은 스냅샷 응답 뒤 동기 루프로 끝난다(accepted가 없으면 콜백 없음) — 한 차례 양보한다.
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(routed).toHaveLength(3);
    expect(acknowledged).toHaveLength(3);
  });
});
