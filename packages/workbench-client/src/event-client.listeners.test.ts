// 043 T030(Codex 설계 리뷰 H2): 수신자 계약. Promise는 settle까지 기다리고 수신자마다 하나씩 처리한다. 거절·동기 예외는 그
// 수신자만 실패로 두고 스냅샷으로 재동기한 뒤 이어 받는다 — 다른 수신자는 막히지 않는다. 재연결 cursor는 수신자들의 반영 완료
// 순번 최솟값이고, 이미 반영한 수신자에게는 다시 넘기지 않는다. 수신자가 바뀌는 중에 도착한 이벤트도 빠지지 않는다.
import { describe, expect, it } from "vitest";

import { clientFor } from "./testing/event-client-harness";
import type { EventEnvelope } from "./operation-map";
import { FakeEventHub, until } from "./testing/fake-event-hub";

function deferred() {
  let resolve!: () => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<void>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/** run 스트림 스냅샷: 지금까지 발행된 순번 전부(= run.replay), 기준점 이후만 통과. */
function runSnapshot(hub: FakeEventHub, streamId: string) {
  return {
    load: async () => ({ lastSequence: hub.lastSequence(streamId) }),
    passes: (event: EventEnvelope, data: unknown) => event.sequence > (data as { lastSequence: number }).lastSequence,
  };
}

describe("event client listener contract", () => {
  it("advances a listener only after its promise resolves and delivers one event at a time", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const gates = [deferred(), deferred(), deferred()];
    const started: number[] = [];
    const applied: number[] = [];
    client.subscribe("run:r1", {
      onEvent: async (event) => {
        started.push(event.sequence);
        await gates[event.sequence - 1].promise;
        applied.push(event.sequence);
      },
    });
    hub.publish("run:r1");
    hub.publish("run:r1");
    hub.publish("run:r1");
    await until(() => started.length === 1, "first delivery");
    expect(started).toEqual([1]); // 1이 끝나기 전에는 2를 넘기지 않는다
    expect(client.debugCursor("run:r1")).toBe(0);
    gates[0].resolve();
    await until(() => started.length === 2, "second delivery");
    expect(client.debugCursor("run:r1")).toBe(1);
    gates[1].resolve();
    gates[2].resolve();
    await until(() => applied.length === 3, "all applied");
    expect(client.debugCursor("run:r1")).toBe(3);
    client.close();
  });

  it("resyncs only the rejecting listener from the snapshot while the other keeps receiving", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const healthy: number[] = [];
    const flaky: number[] = [];
    const resets: unknown[] = [];
    let failOn = 2;
    client.subscribe("run:r1", { onEvent: (event) => void healthy.push(event.sequence) }, { snapshot: runSnapshot(hub, "run:r1") });
    client.subscribe(
      "run:r1",
      {
        onEvent: async (event) => {
          if (event.sequence === failOn) {
            failOn = -1;
            throw new Error("refetch failed");
          }
          flaky.push(event.sequence);
        },
        onReset: async (data) => void resets.push(data),
      },
      { snapshot: runSnapshot(hub, "run:r1") },
    );
    hub.publish("run:r1");
    hub.publish("run:r1"); // flaky 실패 → 스냅샷(lastSequence 2 또는 3)으로 재동기
    hub.publish("run:r1");
    await until(() => healthy.length === 3 && resets.length === 1, "healthy continues, flaky resets");
    hub.publish("run:r1");
    await until(() => flaky.at(-1) === 4, "flaky resumes after reset");
    expect(healthy).toEqual([1, 2, 3, 4]);
    const baseline = (resets[0] as { lastSequence: number }).lastSequence;
    // 스냅샷이 포함한 순번은 다시 받지 않고, 그 뒤만 받는다(중복·누락 없음).
    expect(flaky).toEqual([1, ...[3, 4].filter((sequence) => sequence > baseline)]);
    client.close();
  });

  it("treats a synchronous throw like a rejection", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const resets: unknown[] = [];
    const seen: number[] = [];
    client.subscribe(
      "run:r1",
      {
        onEvent: (event) => {
          if (event.sequence === 1) {
            throw new Error("sync failure");
          }
          seen.push(event.sequence);
        },
        onReset: (data) => void resets.push(data),
      },
      { snapshot: runSnapshot(hub, "run:r1") },
    );
    hub.publish("run:r1");
    await until(() => resets.length === 1, "reset after a sync throw");
    hub.publish("run:r1");
    await until(() => seen.length === 1, "continues");
    expect(seen).toEqual([2]);
    client.close();
  });

  it("reconnects from the minimum applied cursor without redelivering to listeners that already applied", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const fast: number[] = [];
    const slow: number[] = [];
    const gate = deferred();
    client.subscribe("run:r1", { onEvent: (event) => void fast.push(event.sequence) });
    client.subscribe("run:r1", {
      onEvent: async (event) => {
        if (event.sequence === 2) {
          await gate.promise;
        }
        slow.push(event.sequence);
      },
    });
    hub.publish("run:r1");
    hub.publish("run:r1");
    hub.publish("run:r1");
    await until(() => fast.length === 3 && slow.length === 1, "slow blocked on 2");
    hub.sockets[0].drop(); // 받았지만 반영 전(slow는 2 처리 중) 끊김
    await until(() => hub.sockets.length === 2 && hub.sockets[1].readyState === 1, "reconnected");
    expect(hub.ticketRequests.at(-1)).toEqual([{ streamId: "run:r1", epoch: "epoch-1", afterSequence: 1 }]);
    gate.resolve();
    hub.publish("run:r1");
    await until(() => slow.length === 4 && fast.length === 4, "both caught up");
    expect(fast).toEqual([1, 2, 3, 4]);
    expect(slow).toEqual([1, 2, 3, 4]);
    client.close();
  });

  it("hands events that arrive during a listener swap to the new listener", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub, { graceMs: 60_000 });
    const old: number[] = [];
    const next: number[] = [];
    const unsubscribe = client.subscribe("run:r1", { onEvent: (event) => void old.push(event.sequence) });
    hub.publish("run:r1");
    await until(() => old.length === 1, "old listener");
    unsubscribe();
    hub.publish("run:r1"); // 교체 사이에 도착
    client.subscribe("run:r1", { onEvent: (event) => void next.push(event.sequence) });
    hub.publish("run:r1");
    await until(() => next.length === 2, "new listener");
    expect(old).toEqual([1]);
    expect(next).toEqual([2, 3]);
    client.close();
  });
});
