// 043 T041(SC-004c, Codex 설계 리뷰 H1): 실제 042 서버(시험 host 프로세스: 운영 router·런타임·hub, journal 보관 한도 4)에
// 실제 fetch와 WebSocket으로 TS 클라이언트를 붙여, run·교환·orchestration 스트림의 보관 한도 초과 복구를 확인한다.
// 끊긴(구독 해제) 동안 한도를 넘게 변경하고, 마지막으로 반영한 cursor로 다시 구독하면 실제 hub가 `retentionExceeded`를 보낸다.
// 클라이언트는 gap의 lastSequence로 live를 먼저 확보하고 스냅샷으로 재설정한 뒤, 이후 변경을 빠짐·중복 없이 받는다.
import { afterEach, describe, expect, it, vi } from "vitest";

import { createWorkbenchClient } from "../../call-client";
import type { Connection } from "../../connection";
import { createEventClient, type EventClient, type SnapshotSource } from "../../event-client";
import { OPERATION_KINDS } from "../../operation-kinds";
import type { EventEnvelope, OperationId } from "../../operation-map";

import { connectTo, startHost, type Host } from "./host";

let cleanup: Array<() => Promise<void> | void> = [];

afterEach(async () => {
  for (const step of cleanup.reverse()) {
    await step();
  }
  cleanup = [];
});

async function setup() {
  const host = await startHost({ HOST_JOURNAL_CAPACITY: "4" });
  const target = { current: host as Host };
  const connection: Connection = await connectTo(target);
  const client = createWorkbenchClient({ connection });
  const events: EventClient = createEventClient({ connection, graceMs: 0 });
  cleanup.push(() => host.stop(), () => connection.close(), () => events.close());
  const call = async <T = Record<string, unknown>>(operation: string, input: unknown): Promise<T> => {
    const outcome = await client.call(operation as OperationId, input as never);
    if (outcome.kind !== "ok") {
      throw new Error(`${operation}: ${JSON.stringify(outcome)}`);
    }
    return outcome.output as T;
  };
  const bench = (await call<{ benchId: string }>("bench.open", { workingDirectory: host.workDir })).benchId;
  return { host, client, events, call, bench };
}

/** 구독 → 받기 → 해제 → (한도 초과 변경) → 반영 cursor로 다시 구독 → 재설정 → live. */
async function recover(options: {
  events: EventClient;
  streamId: string;
  snapshot: SnapshotSource;
  change: (index: number) => Promise<void>;
}) {
  const first: EventEnvelope[] = [];
  const unsubscribe = options.events.subscribe(options.streamId, { onEvent: (event) => void first.push(event) }, { snapshot: options.snapshot });
  await options.change(0);
  await vi.waitFor(() => expect(first.length).toBeGreaterThan(0), { timeout: 10_000 });
  const applied = first[first.length - 1].sequence;
  unsubscribe();
  for (let index = 1; index <= 6; index += 1) {
    await options.change(index); // 한도 4를 넘긴다
  }
  const resets: unknown[] = [];
  const second: EventEnvelope[] = [];
  options.events.subscribe(
    options.streamId,
    { onEvent: (event) => void second.push(event), onReset: (snapshot) => void resets.push(snapshot) },
    { after: applied, snapshot: options.snapshot },
  );
  await vi.waitFor(() => expect(resets).toHaveLength(1), { timeout: 10_000 });
  const beforeLive = second.length;
  await options.change(100);
  await vi.waitFor(() => expect(second.length).toBeGreaterThan(beforeLive), { timeout: 10_000 });
  return { applied, resets, second, beforeLive };
}

describe("real 042 hub: retention recovery through the TS client", () => {
  it("keeps the operation kind table equal to the server's system.describe", async () => {
    const { call } = await setup();
    const describe = await call<{ operations: Array<{ id: OperationId; kind: string }> }>("system.describe", {});
    const server = Object.fromEntries(describe.operations.map((operation) => [operation.id, operation.kind]));
    // 044: describe는 호출자에게 보이는 operation만 싣는다. 창 주체에게는 소유자 전용 operation이 보이지 않으므로,
    // 보이는 것은 표와 같고 보이지 않는 것은 정확히 소유자 전용 집합이어야 한다.
    const ownerOnly = [
      "server.status",
      "server.stop",
      "lease.acquire",
      "lease.renew",
      "lease.release",
      "desktop.issueWindowToken",
      "desktop.retireWindow",
    ];
    const visible = Object.fromEntries(
      Object.entries(OPERATION_KINDS).filter(([id]) => !ownerOnly.includes(id)),
    );
    expect(visible).toEqual(server);
    expect(Object.keys(OPERATION_KINDS).filter((id) => !(id in server)).sort()).toEqual([...ownerOnly].sort());
  });

  it("recovers the run stream past retention and receives later events without loss or duplication", async () => {
    const { call, events, bench } = await setup();
    await call("run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId: "r1" } });
    const snapshot: SnapshotSource = {
      load: () => call("run.replay", { benchId: bench, runId: "r1", afterSequence: 0 }),
      passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence,
    };
    const { applied, resets, second, beforeLive } = await recover({
      events,
      streamId: "run:r1",
      snapshot,
      change: async (index) => void (await call("run.sendPrompt", { benchId: bench, runId: "r1", prompt: `p${index}` })),
    });
    const replay = resets[0] as { lastSequence: number; gapDetected: boolean };
    expect(replay.lastSequence).toBeGreaterThan(applied + 4); // 끊긴 동안 한도를 넘겼다
    // 스냅샷이 덮은 순번은 다시 받지 않고, 다음 live는 스냅샷 바로 다음 순번이다.
    expect(second.slice(0, beforeLive).every((event) => event.sequence > replay.lastSequence)).toBe(true);
    expect(second[beforeLive].sequence).toBe(replay.lastSequence + 1);
  });

  it("recovers the exchange stream past retention with the snapshot and merges later events", async () => {
    const { call, events, bench, host } = await setup();
    await call("run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId: "r1" } });
    await call("run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId: "r2" } });
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
    const list = () => call<Array<{ requestId: string; status: string; updatedAt: string }>>("exchange.list", { benchId: bench });
    const snapshot: SnapshotSource = {
      load: list,
      passes: (event, data) => {
        const body = event.body as { requestId?: string; updatedAt?: string };
        const current = (data as Array<{ requestId: string; updatedAt: string }>).find((item) => item.requestId === body.requestId);
        return !current || Date.parse(body.updatedAt ?? "") > Date.parse(current.updatedAt);
      },
    };
    const send = async (index: number) =>
      void (await call("exchange.send", {
        benchId: bench,
        request: { requestId: `x-${index}`, sourcePanelId: "main", targetPanelId: "p2", message: `m${index}`, delivery: "queue" },
      }));
    const { resets, second, beforeLive } = await recover({ events, streamId: `exchange:${bench}`, snapshot, change: send });
    const snapshotList = resets[0] as Array<{ requestId: string; status: string }>;
    // 스냅샷이 끊긴 동안의 교환을 모두 담고(확인 전 = accepted, 재조정 대상), 이후 live는 새 교환만이다.
    expect(snapshotList.map((item) => item.requestId).sort()).toEqual(["x-0", "x-1", "x-2", "x-3", "x-4", "x-5", "x-6"]);
    expect(snapshotList.every((item) => item.status === "accepted")).toBe(true);
    const live = second.slice(beforeLive).map((event) => (event.body as { requestId: string }).requestId);
    expect(new Set(live)).toEqual(new Set(["x-100"]));
  });

  it("recovers the orchestration stream past retention and continues after the snapshot revision", async () => {
    const { call, events, bench, host } = await setup();
    const session = await call<{ eventStreamId: string; revision: number; mainNodeId: string }>("orchestration.bootstrap", {
      benchId: bench,
      worktreePath: host.workDir,
    });
    // setPresentation은 직접 자식 노드에만 적용된다: 수동 자식 하나를 둔다.
    const adopted = await call<{ revision: number; nodes: Array<{ id: string; kind: string }> }>(
      "orchestration.adoptManualChild",
      { benchId: bench, panelId: "p2", title: "Child" },
    );
    const child = adopted.nodes.find((node) => node.kind !== "main");
    if (!child) {
      throw new Error("adopted child node missing");
    }
    let revision = adopted.revision;
    const statuses = ["background", "panel"];
    const change = async (index: number) => {
      const next = await call<{ revision: number }>("orchestration.setPresentation", {
        benchId: bench,
        request: {
          requestId: `presentation-${index}-${revision}`,
          nodeId: child.id,
          presentationStatus: statuses[index % 2],
          expectedRevision: revision,
        },
      });
      revision = next.revision;
    };
    const snapshot: SnapshotSource = {
      load: () => call("orchestration.get", { benchId: bench }),
      passes: (event, data) => (event.body as { revision: number }).revision > (data as { revision: number }).revision,
    };
    const { resets, second, beforeLive } = await recover({ events, streamId: session.eventStreamId, snapshot, change });
    const snap = resets[0] as { revision: number };
    expect(snap.revision).toBeGreaterThanOrEqual(session.revision + 7);
    const liveRevisions = second.slice(beforeLive).map((event) => (event.body as { revision: number }).revision);
    expect(liveRevisions.every((value) => value > snap.revision)).toBe(true);
    expect(liveRevisions[liveRevisions.length - 1]).toBe(revision);
  });
});
