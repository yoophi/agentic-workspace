// 043 T057 OCR 구현 리뷰: 재동기와 진행 중인 전달이 겹칠 때의 경합.
// - 재동기(onReset)가 끝난 뒤, 재동기 전에 시작된 onEvent가 늦게 끝나도 새 대기열의 다른 이벤트를 지우거나 cursor를 올리지 않는다.
// - 스냅샷이 이미 반영한 이벤트의 프레임이 재동기 뒤에 도착해도 다시 넘기지 않는다(스냅샷 기준 걸러내기는 다음 재동기까지 유지).
// - 내려간 수신자의 재동기 재시도는 멈춘다.
import { afterEach, describe, expect, it, vi } from "vitest";

import { clientFor } from "./testing/event-client-harness";
import type { EventEnvelope } from "./operation-map";
import { FakeEventHub, until } from "./testing/fake-event-hub";

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

function coveredUpTo(lastSequence: number) {
  return {
    load: async () => ({ lastSequence }),
    passes: (event: EventEnvelope, data: unknown) => event.sequence > (data as { lastSequence: number }).lastSequence,
  };
}

afterEach(() => {
  vi.useRealTimers();
});

describe("event client resync races", () => {
  it("does not drop a queued event when an onEvent that started before a resync settles after it", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const gate = deferred();
    const started: number[] = [];
    const applied: number[] = [];
    const resets: number[] = [];
    client.subscribe(
      "s",
      {
        onEvent: async (event) => {
          started.push(event.sequence);
          if (event.sequence === 1) {
            await gate.promise;
          }
          applied.push(event.sequence);
        },
        onReset: () => void resets.push(1),
      },
      // 스냅샷은 1까지 덮는다(1과 2 사이에 찍힘): 재동기 뒤 대기열은 [2]만 남는다.
      { snapshot: coveredUpTo(1), resyncOnReconnect: true },
    );
    hub.publish("s");
    hub.publish("s");
    await until(() => started.length === 1, "first delivery in flight");
    expect(client.debugDropSockets()).toBe(1);
    await until(() => resets.length === 1, "resync on reconnect");
    gate.resolve(); // 재동기 전에 시작된 1이 이제 끝난다
    await until(() => applied.includes(2), "event 2 delivered after the late settle");
    expect(started.filter((sequence) => sequence === 2)).toEqual([2]);
    expect(client.debugCursor("s")).toBe(2);
    client.close();
  });

  it("keeps filtering by the snapshot after a resync so frames it already covered are not delivered again", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const delivered: number[] = [];
    let failOnce = true;
    client.subscribe(
      "s",
      {
        onEvent: (event) => {
          if (failOnce) {
            failOnce = false;
            throw new Error("refetch failed");
          }
          delivered.push(event.sequence);
        },
        onReset: () => undefined,
      },
      // 서버 스냅샷은 2까지 담았지만 2의 프레임은 재동기 뒤에 도착한다.
      { snapshot: coveredUpTo(2) },
    );
    hub.publish("s"); // 1: 실패 → 재동기
    await until(() => !failOnce && client.debugCursor("s") === 1, "resync after the failure");
    hub.publish("s"); // 2: 스냅샷이 이미 덮음
    hub.publish("s"); // 3
    await until(() => delivered.includes(3), "event 3 delivered");
    expect(delivered).toEqual([3]);
    expect(client.debugCursor("s")).toBe(3);
    client.close();
  });

  it("stops retrying a failed resync once the listener is removed", async () => {
    vi.useFakeTimers();
    const hub = new FakeEventHub();
    const client = clientFor(hub, { maxRecoveryAttempts: 2 });
    let loads = 0;
    const unsubscribe = client.subscribe(
      "s",
      {
        onEvent: () => {
          throw new Error("always fails");
        },
      },
      {
        snapshot: {
          load: async () => {
            loads += 1;
            throw new Error("snapshot down");
          },
          passes: () => true,
        },
      },
    );
    hub.publish("s");
    await vi.advanceTimersByTimeAsync(0);
    expect(loads).toBe(1);
    unsubscribe();
    await vi.advanceTimersByTimeAsync(60_000);
    expect(loads).toBe(1);
    client.close();
  });
});

describe("event client listener join while a ticket is pending", () => {
  it("reopens from the joining listener's earlier cursor instead of the cursor the pending ticket used", async () => {
    const hub = new FakeEventHub();
    let releaseTicket!: () => void;
    const held = new Promise<void>((resolve) => {
      releaseTicket = resolve;
    });
    let holdNext = false;
    const fetchImpl = (async (url: string | URL | Request, init?: RequestInit) => {
      if (holdNext) {
        holdNext = false;
        await held;
      }
      return hub.fetch(url, init);
    }) as typeof fetch;
    const client = clientFor(hub, { fetch: fetchImpl });
    const first: number[] = [];
    client.subscribe("s", { onEvent: (event) => void first.push(event.sequence) });
    hub.publish("s");
    hub.publish("s");
    await until(() => first.length === 2, "first listener caught up");
    holdNext = true;
    expect(client.debugDropSockets()).toBe(1); // 재연결 표(cursor 2)가 붙잡힌다
    await until(() => !holdNext, "reconnect ticket requested");
    const late: number[] = [];
    client.subscribe("s", { onEvent: (event) => void late.push(event.sequence) }, { after: 0 });
    releaseTicket();
    await until(() => late.length === 2, "joining listener receives from its own cursor");
    expect(late).toEqual([1, 2]);
    expect(first).toEqual([1, 2]);
    client.close();
  });
});
