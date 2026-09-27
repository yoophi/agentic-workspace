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
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import { worktreeFileQueryKeys } from "@/entities/worktree-file/api/query-keys";
import { worktreeGitQueryKeys } from "@/entities/worktree-git/api/query-keys";

import {
  createNetworkEvents,
  EXCHANGE_REQUESTED_EVENT,
  EXCHANGE_STATUS_EVENT,
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
    vi.useFakeTimers();
    const hub = new FakeEventHub(3);
    const snapshot = deferred<unknown>();
    const listCalls: unknown[] = [];
    const client: WorkbenchClient = {
      call: vi.fn(async (operation: string, input: unknown) => {
        if (operation === "exchange.list") {
          listCalls.push(input);
          return { kind: "ok", output: await snapshot.promise, revision: undefined } as CallOutcome<unknown>;
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
    await vi.advanceTimersByTimeAsync(0);
    await until(() => statuses.length === 1, "first status");

    // 끊긴 동안 보관 한도(3)를 넘긴다 → 다시 붙으면 보관 gap → gap의 lastSequence로 live 확보 후 스냅샷.
    hub.ticketsDown = true;
    hub.sockets[0].drop();
    for (let i = 0; i < 5; i += 1) {
      hub.publish(stream, exchange(`filler-${i}`, "delivered", "2026-09-28T00:00:02Z"), STATUS);
    }
    hub.ticketsDown = false;
    await vi.advanceTimersByTimeAsync(10_000);
    await until(() => listCalls.length === 1, "snapshot requested after live is secured");

    // 스냅샷을 붙잡아 둔 사이 live로 온(버퍼에 쌓이는) 이벤트: x1의 옛 요청·옛 accepted, 그 뒤 delivered.
    hub.publish(stream, exchange("x1", "accepted", "2026-09-28T00:00:03Z"), REQUESTED); // 옛 요청
    hub.publish(stream, exchange("x1", "accepted", "2026-09-28T00:00:03Z"), STATUS); // 옛 accepted
    hub.publish(stream, exchange("x1", "delivered", "2026-09-28T00:00:04Z"), STATUS);
    hub.publish(stream, exchange("x2", "accepted", "2026-09-28T00:00:05Z"), REQUESTED);
    hub.publish(stream, exchange("x2", "accepted", "2026-09-28T00:00:05Z"), STATUS);
    snapshot.resolve([exchange("x1", "delivered", "2026-09-28T00:00:04Z"), exchange("x2", "accepted", "2026-09-28T00:00:05Z")]);
    await vi.advanceTimersByTimeAsync(0);
    await until(() => statuses.some((item) => item.requestId === "x2"), "snapshot applied");
    await vi.advanceTimersByTimeAsync(0);

    const afterSnapshot = statuses.slice(statuses.findIndex((item) => item.requestId === "x1"));
    // x1은 스냅샷의 delivered로 끝나고, 버퍼의 옛 accepted로 되돌아가지 않는다.
    expect(afterSnapshot.filter((item) => item.requestId === "x1").map((item) => item.status)).toEqual(["delivered"]);
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
