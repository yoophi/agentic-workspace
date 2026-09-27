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

describe("event client resync ordering (Codex implementation review)", () => {
  it("never applies an older resync snapshot after a newer one, even when its load resolves last", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const firstLoad = deferred();
    let loads = 0;
    const applied: number[] = [];
    let failOnce = true;
    client.subscribe(
      "s",
      {
        onEvent: () => {
          if (failOnce) {
            failOnce = false;
            throw new Error("refetch failed");
          }
        },
        onReset: (data) => void applied.push((data as { version: number }).version),
      },
      {
        snapshot: {
          load: async () => {
            loads += 1;
            if (loads === 1) {
              await firstLoad.promise; // 첫 재동기(수신자 실패)의 응답이 늦는다
              return { version: 1 };
            }
            return { version: 2 };
          },
          passes: () => true,
        },
        resyncOnReconnect: true,
      },
    );
    hub.publish("s"); // 실패 → 재동기 1(보류)
    await until(() => loads === 1, "first resync loading");
    expect(client.debugDropSockets()).toBe(1); // 재연결 → 재동기 2
    // 재연결 hello가 두 번째 재동기를 시작한다(직렬화 뒤에는 첫 적재가 끝나야 두 번째 적재가 돈다).
    await until(() => hub.sockets.length === 2 && hub.sockets[1].readyState === 1, "reconnected");
    for (let i = 0; i < 20; i += 1) {
      await Promise.resolve();
    }
    firstLoad.resolve();
    await until(() => applied.length > 0 && loads >= 2, "a snapshot applied");
    for (let i = 0; i < 50; i += 1) {
      await Promise.resolve();
    }
    expect(applied[applied.length - 1]).toBe(2);
    expect(applied).not.toContain(1);
    client.close();
  });

  it("gives a listener that joins during gap recovery its own snapshot before live events", async () => {
    const hub = new FakeEventHub(2);
    for (let i = 0; i < 5; i += 1) {
      hub.publish("s"); // 보관 한도 2: 1–3은 journal에서 빠진다
    }
    const client = clientFor(hub);
    const snapshot = {
      load: async () => ({ lastSequence: hub.lastSequence("s") }),
      passes: (event: EventEnvelope, data: unknown) => event.sequence > (data as { lastSequence: number }).lastSequence,
    };
    const hold = deferred();
    let aResets = 0;
    client.subscribe(
      "s",
      {
        onEvent: () => undefined,
        onReset: async () => {
          aResets += 1;
          await hold.promise; // 복구의 onReset이 끝나기 전에 B가 합류한다
        },
      },
      { snapshot },
    );
    await until(() => aResets === 1, "recovery resetting A");
    const bResets: unknown[] = [];
    const bEvents: number[] = [];
    client.subscribe(
      "s",
      { onEvent: (event) => void bEvents.push(event.sequence), onReset: (data) => void bResets.push(data) },
      { after: 0, snapshot },
    );
    hold.resolve();
    await until(() => bResets.length === 1, "B gets its own snapshot");
    hub.publish("s"); // 6
    await until(() => bEvents.includes(6), "B receives live 6");
    expect(bResets).toEqual([{ lastSequence: 5 }]);
    expect(bEvents).toEqual([6]);
    client.close();
  });
});

describe("event client resync liveness (Codex follow-up review)", () => {
  it("lets a newer resync proceed while a superseded snapshot load never settles", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    let loads = 0;
    const applied: number[] = [];
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
        onReset: (data) => void applied.push((data as { version: number }).version),
      },
      {
        snapshot: {
          load: () => {
            loads += 1;
            // 첫 적재(수신자 실패 재동기)는 끝나지 않는다. 두 번째(재연결 재동기)는 1까지 덮는 스냅샷.
            return loads === 1 ? new Promise<never>(() => undefined) : Promise.resolve({ version: 2, lastSequence: 1 });
          },
          passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence,
        },
        resyncOnReconnect: true,
      },
    );
    hub.publish("s"); // 1: 실패 → 재동기 1(영영 보류)
    await until(() => loads === 1, "first resync loading");
    expect(client.debugDropSockets()).toBe(1); // 재연결 → 재동기 2
    await until(() => applied.length === 1, "the newer resync applies despite the pending load");
    expect(applied).toEqual([2]);
    hub.publish("s"); // 2
    await until(() => delivered.includes(2), "delivery resumes");
    expect(client.debugCursor("s")).toBe(2);
    client.close();
  });

  it("resets a listener that joins while terminal (evicted) recovery is resetting others", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const snapshot = { load: async () => ({ final: true }), passes: () => false };
    const hold = deferred();
    let aResets = 0;
    client.subscribe(
      "s",
      {
        onEvent: () => undefined,
        onReset: async () => {
          aResets += 1;
          await hold.promise;
        },
      },
      { snapshot },
    );
    hub.publish("s");
    await until(() => client.debugCursor("s") === 1, "first event applied");
    hub.evict("s");
    expect(client.debugDropSockets()).toBe(1); // 재연결 → evicted → 종결 복구
    await until(() => aResets === 1, "terminal recovery resetting A");
    const bResets: unknown[] = [];
    client.subscribe("s", { onEvent: () => undefined, onReset: (data) => void bResets.push(data) }, { snapshot });
    hold.resolve();
    await until(() => bResets.length === 1, "B gets the final snapshot");
    expect(bResets).toEqual([{ final: true }]);
    client.close();
  });
});

describe("event client recovery isolation (Codex follow-up review 2)", () => {
  it("keeps delivering to other listeners when a joiner's recovery reset never settles and the joiner leaves", async () => {
    const hub = new FakeEventHub(2);
    for (let i = 0; i < 5; i += 1) {
      hub.publish("s");
    }
    const client = clientFor(hub);
    const snapshot = {
      load: async () => ({ lastSequence: hub.lastSequence("s") }),
      passes: (event: EventEnvelope, data: unknown) => event.sequence > (data as { lastSequence: number }).lastSequence,
    };
    const holdLoad = deferred();
    let loads = 0;
    const aEvents: number[] = [];
    client.subscribe(
      "s",
      { onEvent: (event) => void aEvents.push(event.sequence), onReset: () => undefined },
      {
        snapshot: {
          load: async () => {
            loads += 1;
            if (loads === 1) {
              await holdLoad.promise; // 복구 적재 중에 B가 합류한다
            }
            return snapshot.load();
          },
          passes: snapshot.passes,
        },
      },
    );
    await until(() => loads === 1, "recovery loading");
    const unsubscribeB = client.subscribe(
      "s",
      { onEvent: () => undefined, onReset: () => new Promise<void>(() => undefined) }, // 끝나지 않는 재설정
      { snapshot },
    );
    holdLoad.resolve();
    for (let i = 0; i < 50; i += 1) {
      await Promise.resolve();
    }
    unsubscribeB();
    hub.publish("s"); // 6
    await until(() => aEvents.includes(6), "A keeps receiving live events");
    expect(client.debugCursor("s")).toBe(6);
    client.close();
  });
});

describe("event client recovery cursors (Codex follow-up review 3)", () => {
  it("passes the listener's actual applied cursor to the recovery reset, not the gap boundary", async () => {
    const hub = new FakeEventHub(2);
    for (let i = 0; i < 5; i += 1) {
      hub.publish("s");
    }
    const client = clientFor(hub);
    const contexts: number[] = [];
    client.subscribe(
      "s",
      { onEvent: () => undefined, onReset: (_data, context) => void contexts.push(context.delivered) },
      {
        snapshot: {
          load: async () => ({ lastSequence: hub.lastSequence("s") }),
          passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence,
        },
      },
    );
    await until(() => contexts.length === 1, "recovery reset");
    expect(contexts).toEqual([0]); // run 소비자는 이 값 뒤의 스냅샷 이벤트(1–5)를 다시 반영한다
    await until(() => client.debugCursor("s") === 5, "cursor moves to the gap boundary after the reset");
    client.close();
  });

  it("delivers buffered recovery events once and in order when the socket reconnects during the snapshot load", async () => {
    const hub = new FakeEventHub(2);
    for (let i = 0; i < 5; i += 1) {
      hub.publish("s");
    }
    const client = clientFor(hub);
    const holdLoad = deferred();
    let loads = 0;
    const events: number[] = [];
    client.subscribe(
      "s",
      { onEvent: (event) => void events.push(event.sequence), onReset: () => undefined },
      {
        snapshot: {
          load: async () => {
            loads += 1;
            await holdLoad.promise;
            return { lastSequence: 5 };
          },
          passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence,
        },
      },
    );
    await until(() => loads === 1, "recovery snapshot loading");
    hub.publish("s"); // 6 (버퍼)
    hub.publish("s"); // 7 (버퍼)
    expect(client.debugDropSockets()).toBe(1); // 복구 기준점 5에서 다시 연결 → 6·7 재생
    await until(() => hub.sockets.length === 3 && hub.sockets[2].readyState === 1, "reconnected during recovery");
    for (let i = 0; i < 20; i += 1) {
      await Promise.resolve();
    }
    holdLoad.resolve();
    await until(() => events.length >= 2, "buffered events delivered");
    for (let i = 0; i < 20; i += 1) {
      await Promise.resolve();
    }
    expect(events).toEqual([6, 7]);
    client.close();
  });
});

describe("event client epoch change followed by a gap (Codex follow-up review 4)", () => {
  it("resets with applied 0 and reconnects in the new epoch when a retention gap follows an epoch change", async () => {
    const hub = new FakeEventHub(2);
    const client = clientFor(hub);
    const contexts: number[] = [];
    const events: number[] = [];
    client.subscribe(
      "s",
      { onEvent: (event) => void events.push(event.sequence), onReset: (_data, context) => void contexts.push(context.delivered) },
      {
        snapshot: {
          load: async () => ({ lastSequence: hub.lastSequence("s") }),
          passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence,
        },
      },
    );
    for (let i = 0; i < 3; i += 1) {
      hub.publish("s");
    }
    await until(() => client.debugCursor("s") === 3, "old epoch applied");
    hub.ticketsDown = true;
    hub.restart("epoch-2");
    for (let i = 0; i < 5; i += 1) {
      hub.publish("s"); // 새 세대 1–5, 보관 한도 2
    }
    hub.ticketsDown = false;
    await until(() => contexts.length >= 1 && client.debugCursor("s") === 5, "recovered in the new epoch", 100_000);
    expect(contexts[contexts.length - 1]).toBe(0);
    hub.publish("s"); // 새 세대 6
    await until(() => events.includes(6), "live 6 in the new epoch", 100_000);
    expect(client.debugCursor("s")).toBe(6);
    client.close();
  });
});

describe("event client recovery reconnect cursor (Codex follow-up review 5)", () => {
  it("reconnects from the gap boundary while a recovery reset is still running", async () => {
    const hub = new FakeEventHub(2);
    for (let i = 0; i < 5; i += 1) {
      hub.publish("s");
    }
    const client = clientFor(hub);
    const hold = deferred();
    let resets = 0;
    client.subscribe(
      "s",
      {
        onEvent: () => undefined,
        onReset: async () => {
          resets += 1;
          await hold.promise;
        },
      },
      {
        snapshot: {
          load: async () => ({ lastSequence: hub.lastSequence("s") }),
          passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence,
        },
      },
    );
    await until(() => resets === 1, "recovery reset running");
    expect(client.debugCursor("s")).toBe(5); // 복구 스냅샷이 5까지 덮는다
    const before = hub.ticketRequests.length;
    expect(client.debugDropSockets()).toBe(1);
    await until(() => hub.ticketRequests.length > before, "reconnect ticket");
    expect(hub.ticketRequests[hub.ticketRequests.length - 1][0].afterSequence).toBe(5);
    hold.resolve();
    client.close();
  });
});

describe("event client terminal retry budget (Codex follow-up review 9)", () => {
  it("stops retrying a failing final snapshot on a terminal stream after the retry budget and reports it", async () => {
    vi.useFakeTimers();
    const hub = new FakeEventHub();
    const errors: string[] = [];
    const client = clientFor(hub, { maxRecoveryAttempts: 2, onStreamError: (_stream, error) => void errors.push(error) });
    let resets = 0;
    let loads = 0;
    client.subscribe(
      "s",
      {
        onEvent: () => undefined,
        onReset: () => {
          resets += 1;
          throw new Error("always fails");
        },
      },
      {
        snapshot: {
          load: async () => {
            loads += 1;
            return {};
          },
          passes: () => true,
        },
      },
    );
    hub.publish("s");
    await vi.advanceTimersByTimeAsync(0);
    hub.evict("s");
    expect(client.debugDropSockets()).toBe(1); // 재연결 → evicted → 종결 복구
    await vi.advanceTimersByTimeAsync(60_000);
    const settledResets = resets;
    const settledLoads = loads;
    await vi.advanceTimersByTimeAsync(600_000);
    expect(resets).toBe(settledResets);
    expect(loads).toBe(settledLoads);
    expect(resets).toBeLessThanOrEqual(1 + 2); // 복구 적용 1회 + 재시도 예산 2회
    expect(errors.some((error) => error.startsWith("listener resync failed"))).toBe(true);
    client.close();
  });
});
