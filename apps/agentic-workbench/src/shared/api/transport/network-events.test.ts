// 043 T034·T040(사용자 검토): 네트워크 이벤트 계층.
// (1) 교환 live-first 복구: 스냅샷(`exchange.list`)이 이미 종결을 보여 주면, 버퍼에 남은 그보다 오래된 상태·요청 이벤트는
//     다시 넘기지 않는다(requestId + updatedAt 병합) — 화면 상태가 역행하지 않고 끝난 교환이 다시 라우팅되지 않는다.
// (2) Worktree 알림 스트림: 재연결되면 전체 다시 읽기 신호를 보내고, 그 신호가 화면 무효화 경로에서 파일 목록·변경·
//     Git 이력을 모두 무효화한다.
import { QueryClient } from "@tanstack/react-query";
import { createEventClient, type CallOutcome, type EventConnectionPort, type WorkbenchClient } from "@yoophi/workbench-client";
import { FakeEventHub, until } from "@yoophi/workbench-client/testing";
import { afterEach, describe, expect, it, vi } from "vitest";

import { invalidateForWorktreeChange } from "@/features/worktree-workspace/model/worktree-change-invalidation";
import { createExchangeReconciler } from "@/features/agent-run/model/exchange-reconciler";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import { worktreeFileQueryKeys } from "@/entities/worktree-file/api/query-keys";
import { worktreeGitQueryKeys } from "@/entities/worktree-git/api/query-keys";

import {
  createNetworkEvents,
  EXCHANGE_REQUESTED_EVENT,
  EXCHANGE_STATUS_EVENT,
  ORCHESTRATION_WORKSPACE_UPDATED_EVENT,
  WORKTREE_CHANGED_EVENT,
} from "./network-events";

function connection(hub: FakeEventHub): EventConnectionPort {
  return {
    credentials: () => ({ baseUrl: "http://127.0.0.1:9", token: "t" }),
    epoch: () => hub.epoch,
    reportLost: vi.fn(),
    whenConnected: vi.fn(async () => ({ serverEpoch: hub.epoch })),
    refreshCredentials: vi.fn(async () => undefined),
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

const STATUS = "exchange.status.v1";
const REQUESTED = "exchange.requested.v1";

const exchange = (requestId: string, status: string, updatedAt: string) => ({
  requestId,
  source: { panelId: "main", title: "Main", runId: "r1" },
  target: { panelId: "p2", title: "Panel", runId: "r2" },
  message: `m ${requestId}`,
  delivery: "queue",
  status,
  createdAt: "2026-09-28T00:00:00Z",
  updatedAt,
});

afterEach(() => {
  vi.useRealTimers();
});

describe("network events — exchange live-first recovery merges by requestId and updatedAt", () => {
  it("does not replay requested or status events older than the snapshot", async () => {
    const hub = new FakeEventHub(3);
    const snapshot = deferred<unknown>();
    const listCalls: unknown[] = [];
    const client: WorkbenchClient = {
      call: vi.fn(async (operation: string, input: unknown) => {
        if (operation === "exchange.list") {
          listCalls.push(input);
          // 처음 두 호출은 구독 시작 재조정(수신자 둘 — 요청·상태 — 이 각자 재설정, 아직 교환 없음), 이후는 보관 gap
          // 복구 스냅샷(붙잡아 둔다).
          const output = listCalls.length <= 2 ? [] : await snapshot.promise;
          return { kind: "ok", output, revision: undefined } as CallOutcome<unknown>;
        }
        throw new Error(`unexpected ${operation}`);
      }) as never,
    };
    const events = createEventClient({ connection: connection(hub), fetch: hub.fetch, openSocket: hub.openSocket as never, random: () => 0.5 });
    const network = createNetworkEvents({ events, client });
    const statuses: Array<{ requestId: string; status: string }> = [];
    const requests: string[] = [];
    await network.listen(EXCHANGE_STATUS_EVENT, (payload) => void statuses.push(payload as never));
    await network.listen(EXCHANGE_REQUESTED_EVENT, (payload) => void requests.push((payload as { requestId: string }).requestId));
    network.noteBench("b1");
    const stream = "exchange:b1";

    hub.publish(stream, exchange("x0", "accepted", "2026-09-28T00:00:01Z"), STATUS);
    await vi.waitFor(() => expect(statuses).toHaveLength(1));

    // 끊긴 동안 보관 한도(3)를 넘긴다 → 다시 붙으면 보관 gap → gap의 lastSequence로 live 확보 후 스냅샷.
    hub.ticketsDown = true;
    hub.sockets[0].drop();
    for (let i = 0; i < 5; i += 1) {
      hub.publish(stream, exchange(`filler-${i}`, "delivered", "2026-09-28T00:00:02Z"), STATUS);
    }
    hub.ticketsDown = false;
    // 재연결 → hello(재연결 재조정이 스냅샷을 요청할 수 있음) → 보관 gap → lastSequence로 live 확보 → 복구 스냅샷 요청.
    await vi.waitFor(() => expect(listCalls.length).toBeGreaterThanOrEqual(3), { timeout: 5_000 });
    await vi.waitFor(() => expect(hub.ticketRequests.some((request) => request[0].afterSequence > 1)).toBe(true));

    // 스냅샷을 붙잡아 둔 사이 live로 온(버퍼에 쌓이는) 이벤트: x1의 옛 요청·옛 accepted, 그 뒤 delivered.
    hub.publish(stream, exchange("x1", "accepted", "2026-09-28T00:00:03Z"), REQUESTED); // 옛 요청
    hub.publish(stream, exchange("x1", "accepted", "2026-09-28T00:00:03Z"), STATUS); // 옛 accepted
    hub.publish(stream, exchange("x1", "delivered", "2026-09-28T00:00:04Z"), STATUS);
    hub.publish(stream, exchange("x2", "accepted", "2026-09-28T00:00:05Z"), REQUESTED);
    hub.publish(stream, exchange("x2", "accepted", "2026-09-28T00:00:05Z"), STATUS);
    snapshot.resolve([exchange("x1", "delivered", "2026-09-28T00:00:04Z"), exchange("x2", "accepted", "2026-09-28T00:00:05Z")]);
    await vi.waitFor(() => expect(statuses.some((item) => item.requestId === "x2")).toBe(true));
    await new Promise((resolve) => setTimeout(resolve, 0));

    const afterSnapshot = statuses.slice(statuses.findIndex((item) => item.requestId === "x1"));
    // x1은 스냅샷의 delivered로 끝나고, 버퍼의 옛 accepted로 되돌아가지 않는다. (재연결 hello의 재조정과 뒤이은 gap 복구가
    // 같은 스냅샷을 두 번 적용할 수 있다 — requestId 덮어쓰기라 결과는 같고, 역행은 없다.)
    const x1States = afterSnapshot.filter((item) => item.requestId === "x1").map((item) => item.status);
    expect(x1States.length).toBeGreaterThan(0);
    expect(new Set(x1States)).toEqual(new Set(["delivered"]));
    // 스냅샷에서 이미 종결된 x1의 요청은 다시 넘기지 않고, 확인 전인 x2만 넘긴다.
    expect(requests).not.toContain("x1");
    expect(requests).toContain("x2");
    events.close();
  });
});

describe("network events — worktree resync reaches the screen invalidation", () => {
  it("sends a full re-read signal after reconnecting and it invalidates files, changes and git history", async () => {
    const hub = new FakeEventHub();
    const client: WorkbenchClient = { call: vi.fn() as never };
    const events = createEventClient({ connection: connection(hub), fetch: hub.fetch, openSocket: hub.openSocket as never, random: () => 0.5 });
    const network = createNetworkEvents({ events, client });
    const received: Array<{ workingDirectory: string; kind?: string; reason?: string }> = [];
    const queryClient = new QueryClient();
    const path = "/work/wt";
    await network.listen(WORKTREE_CHANGED_EVENT, (payload) => {
      const change = payload as { workingDirectory: string; kind?: string };
      received.push(change as never);
      invalidateForWorktreeChange(queryClient, change.workingDirectory, "files", change.kind);
    });
    network.watchWorktree(path);
    await until(() => hub.sockets.length === 1 && hub.sockets[0].readyState === 1, "worktree subscribed");
    hub.publish(`worktree:${path}`, { kind: "file", paths: ["a.txt"] });
    await until(() => received.length === 1, "live change");
    expect(received[0]).toMatchObject({ workingDirectory: path, kind: "file" });

    const keys = [
      worktreeFileQueryKeys.list(path),
      projectQueryKeys.worktreeChanges(path),
      worktreeGitQueryKeys.history(path),
      worktreeGitQueryKeys.graph(path),
    ];
    for (const key of keys) {
      queryClient.setQueryData(key, "cached");
    }
    hub.sockets[0].drop(); // 끊긴 동안의 알림은 보관되지 않는다
    await until(() => received.length === 2, "resync after reconnect");
    expect(received[1]).toEqual({ workingDirectory: path, kind: "git", reason: "resync" });
    for (const key of keys) {
      expect(queryClient.getQueryState(key)?.isInvalidated, JSON.stringify(key)).toBe(true);
    }
    events.close();
  });
});

describe("network events — exchange reconciliation triggers (T047)", () => {
  function reconcilerSetup(acknowledge: (ack: unknown) => Promise<void>) {
    const route = vi.fn();
    const reconciler = createExchangeReconciler({
      route: (request) => {
        route(request.requestId);
        return { routed: true };
      },
      acknowledge: acknowledge as never,
    });
    return { reconciler, route };
  }

  async function wire(hub: FakeEventHub, list: () => unknown, reconciler: ReturnType<typeof createExchangeReconciler>) {
    const listCalls: number[] = [];
    const client: WorkbenchClient = {
      call: vi.fn(async (operation: string) => {
        if (operation !== "exchange.list") {
          throw new Error(`unexpected ${operation}`);
        }
        listCalls.push(Date.now());
        return { kind: "ok", output: list(), revision: undefined } as CallOutcome<unknown>;
      }) as never,
    };
    const events = createEventClient({ connection: connection(hub), fetch: hub.fetch, openSocket: hub.openSocket as never, random: () => 0.5 });
    const network = createNetworkEvents({ events, client });
    await network.listen(EXCHANGE_REQUESTED_EVENT, (payload) => reconciler.handleRequested(payload as never));
    await network.listen(EXCHANGE_STATUS_EVENT, (payload) => reconciler.observeStatus(payload as never));
    network.noteBench("b1");
    return { events, listCalls };
  }

  it("retries only the acknowledgement after a same-epoch reconnect with no new events and no retention gap", async () => {
    const hub = new FakeEventHub();
    const acknowledge = vi.fn<(ack: unknown) => Promise<void>>().mockRejectedValueOnce("network").mockResolvedValue(undefined);
    const { reconciler, route } = reconcilerSetup(acknowledge);
    let serverState: unknown[] = [];
    const { events, listCalls } = await wire(hub, () => serverState, reconciler);
    // 구독 시작 재조정: 수신자(요청·상태)마다 스냅샷을 읽는다(아직 교환 없음).
    await vi.waitFor(() => expect(listCalls).toHaveLength(2));
    // 교환 요청: 서버는 accepted로 저장하고 요청 이벤트를 낸다. 화면은 라우팅하고 확인하지만 확인이 한 번 실패한다.
    serverState = [exchange("x1", "accepted", "2026-09-28T00:00:01Z")];
    hub.publish("exchange:b1", exchange("x1", "accepted", "2026-09-28T00:00:01Z"), REQUESTED);
    await vi.waitFor(() => expect(acknowledge).toHaveBeenCalledTimes(1));
    expect(route).toHaveBeenCalledTimes(1);
    const socketsBefore = hub.sockets.length;
    const listsBefore = listCalls.length;

    // 새 이벤트도 보관 gap도 없이 소켓만 다시 연결된다(같은 세대).
    hub.sockets[hub.sockets.length - 1].drop();
    await vi.waitFor(() => {
      expect(hub.sockets).toHaveLength(socketsBefore + 1);
      expect(listCalls).toHaveLength(listsBefore + 2); // 재연결 재조정(수신자 둘)
    });
    await vi.waitFor(() => expect(acknowledge).toHaveBeenCalledTimes(2));
    expect(route).toHaveBeenCalledTimes(1); // 다시 라우팅하지 않는다
    expect(acknowledge.mock.calls[1][0]).toEqual({ requestId: "x1", targetPanelId: "p2", outcome: "delivered", reason: null });

    // 서버가 확인을 반영한 뒤의 재연결: 더 하지 않는다.
    serverState = [exchange("x1", "delivered", "2026-09-28T00:00:02Z")];
    hub.sockets[hub.sockets.length - 1].drop();
    await vi.waitFor(() => expect(listCalls).toHaveLength(listsBefore + 4));
    await new Promise((resolve) => setTimeout(resolve, 0)); // 재설정 처리 한 차례 양보(추가 호출이 있으면 여기서 드러난다)
    expect(acknowledge).toHaveBeenCalledTimes(2);
    expect(route).toHaveBeenCalledTimes(1);
    events.close();
  });

  it("reconciles at subscription start: an accepted exchange whose request event is not in the stream is delivered once", async () => {
    const hub = new FakeEventHub();
    const acknowledge = vi.fn(async () => undefined);
    const { reconciler, route } = reconcilerSetup(acknowledge);
    const { events, listCalls } = await wire(hub, () => [exchange("x0", "accepted", "2026-09-28T00:00:01Z")], reconciler);
    await vi.waitFor(() => expect(listCalls).toHaveLength(2)); // 구독 시작 재조정이 수신자마다 스냅샷을 부른다
    await vi.waitFor(() => expect(acknowledge).toHaveBeenCalledTimes(1));
    expect(route.mock.calls).toEqual([["x0"]]);
    events.close();
  });
});

describe("network events — orchestration listeners that refetch asynchronously (T038)", () => {
  it("resyncs only the failing listener through a revision signal while the other keeps receiving", async () => {
    const hub = new FakeEventHub();
    let revision = 1;
    const client: WorkbenchClient = {
      call: vi.fn(async (operation: string) => {
        if (operation === "orchestration.get") {
          return { kind: "ok", output: { id: "ws1", revision }, revision: undefined } as CallOutcome<unknown>;
        }
        throw new Error(`unexpected ${operation}`);
      }) as never,
    };
    const events = createEventClient({ connection: connection(hub), fetch: hub.fetch, openSocket: hub.openSocket as never, random: () => 0.5 });
    const network = createNetworkEvents({ events, client });
    // 화면 수신자(worktree-agent-run-area)와 같은 모양: 이벤트를 받으면 작업 영역을 await로 다시 읽는다.
    let refetchFailures = 1;
    const refetched: number[] = [];
    const flakySignals: Array<{ revision: number; reason?: string }> = [];
    await network.listen(ORCHESTRATION_WORKSPACE_UPDATED_EVENT, async (payload) => {
      const signal = payload as { revision: number; reason?: string };
      flakySignals.push(signal);
      if (refetchFailures > 0) {
        refetchFailures -= 1;
        throw new Error("getOrchestrationWorkspace failed");
      }
      const session = await client.call("orchestration.get" as never, {} as never);
      refetched.push(((session as { output: { revision: number } }).output).revision);
    });
    const other: number[] = [];
    await network.listen(ORCHESTRATION_WORKSPACE_UPDATED_EVENT, (payload) => {
      other.push((payload as { revision: number }).revision);
      if (other.length === 1) {
        throw new Error("synchronous failure in the second listener");
      }
    });
    network.noteBench("b1");
    network.noteOrchestrationStream("orchestration:bind-1");
    await vi.waitFor(() => expect(hub.sockets.some((socket) => socket.readyState === 1)).toBe(true));

    revision = 2;
    hub.publish("orchestration:bind-1", { workspaceId: "ws1", revision: 2, reason: "bootstrap" }, "orchestration.workspaceUpdated.v1");
    revision = 3;
    hub.publish("orchestration:bind-1", { workspaceId: "ws1", revision: 3, reason: "child" }, "orchestration.workspaceUpdated.v1");
    // 첫 수신자: 재조회 실패 → 스냅샷(revision 3)의 재설정 신호 → 다시 읽어 revision 3 반영. 스냅샷 이하 이벤트는 다시 받지 않는다.
    await vi.waitFor(() => expect(refetched).toContain(3));
    expect(flakySignals.some((signal) => signal.reason === "resync" && signal.revision === 3)).toBe(true);
    // 두 번째 수신자(동기 예외): 재설정 신호로 revision 3을 받는다. 첫 수신자의 실패에 막히지 않았다.
    await vi.waitFor(() => expect(other).toContain(3));
    revision = 4;
    hub.publish("orchestration:bind-1", { workspaceId: "ws1", revision: 4, reason: "child" }, "orchestration.workspaceUpdated.v1");
    await vi.waitFor(() => {
      expect(refetched[refetched.length - 1]).toBe(4);
      expect(other[other.length - 1]).toBe(4);
    });
    events.close();
  });
});
