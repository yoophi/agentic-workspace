// 043 T029: 이벤트 클라이언트 기본(research R7, contracts §5). 스트림당 소켓 하나, 표 cursor, hello 뒤 전달, 수신자가 없는 동안의
// 대기열(상한 초과 시 cursor에서 다시 구독), 마지막 수신자가 떠나고 유예가 지나면 닫기.
import { afterEach, describe, expect, it, vi } from "vitest";

import type { EventEnvelope } from "./operation-map";
import { FakeEventHub, until } from "./testing/fake-event-hub";
import { clientFor } from "./testing/event-client-harness";

function recorder() {
  const seen: number[] = [];
  return { seen, listener: { onEvent: (event: EventEnvelope) => void seen.push(event.sequence) } };
}

afterEach(() => {
  vi.useRealTimers();
});

describe("createEventClient basics", () => {
  it("issues one ticket from cursor 0 in the current epoch and delivers events after hello in order", async () => {
    const hub = new FakeEventHub();
    hub.publish("run:r1");
    const client = clientFor(hub);
    const { seen, listener } = recorder();
    client.subscribe("run:r1", listener);
    await until(() => seen.length === 1, "replayed event");
    hub.publish("run:r1");
    hub.publish("run:r1");
    await until(() => seen.length === 3, "live events");
    expect(seen).toEqual([1, 2, 3]);
    expect(hub.ticketRequests).toEqual([[{ streamId: "run:r1", epoch: "epoch-1", afterSequence: 0 }]]);
    expect(hub.sockets).toHaveLength(1);
    client.close();
  });

  it("uses one socket per stream and a starting cursor when given", async () => {
    const hub = new FakeEventHub();
    for (let i = 0; i < 5; i += 1) {
      hub.publish("run:r1");
    }
    hub.publish("exchange:b1");
    const client = clientFor(hub);
    const run = recorder();
    const exchange = recorder();
    client.subscribe("run:r1", run.listener, { after: 3 });
    client.subscribe("exchange:b1", exchange.listener);
    await until(() => run.seen.length === 2 && exchange.seen.length === 1, "both streams");
    expect(run.seen).toEqual([4, 5]);
    expect(hub.sockets).toHaveLength(2);
    expect(hub.ticketRequests).toContainEqual([{ streamId: "run:r1", epoch: "epoch-1", afterSequence: 3 }]);
    client.close();
  });

  it("keeps frames that arrive with no listener and hands them to the next listener", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub, { graceMs: 60_000 });
    const first = recorder();
    const unsubscribe = client.subscribe("run:r1", first.listener);
    hub.publish("run:r1");
    await until(() => first.seen.length === 1, "first listener");
    unsubscribe();
    hub.publish("run:r1");
    hub.publish("run:r1");
    const second = recorder();
    client.subscribe("run:r1", second.listener);
    await until(() => second.seen.length === 2, "backlog to next listener");
    expect(second.seen).toEqual([2, 3]);
    expect(hub.sockets).toHaveLength(1); // 유예 안에 다시 붙어 같은 소켓
    client.close();
  });

  it("drops the backlog past its limit and resubscribes from the cursor when a listener returns", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub, { graceMs: 60_000, backlogLimit: 3 });
    const first = recorder();
    const unsubscribe = client.subscribe("run:r1", first.listener);
    hub.publish("run:r1");
    await until(() => first.seen.length === 1, "first");
    unsubscribe();
    for (let i = 0; i < 5; i += 1) {
      hub.publish("run:r1"); // 2..6, 상한 3 초과 → 소켓을 닫고 대기열을 버린다
    }
    await until(() => hub.sockets[0].readyState === 3, "backlog overflow closes the socket");
    const second = recorder();
    client.subscribe("run:r1", second.listener);
    await until(() => second.seen.length === 5, "resubscribed from cursor 1");
    expect(second.seen).toEqual([2, 3, 4, 5, 6]);
    expect(hub.ticketRequests[hub.ticketRequests.length - 1]).toEqual([{ streamId: "run:r1", epoch: "epoch-1", afterSequence: 1 }]);
    client.close();
  });

  it("closes the stream socket after the last listener leaves and the grace passes", async () => {
    vi.useFakeTimers();
    const hub = new FakeEventHub();
    const client = clientFor(hub, { graceMs: 1_000 });
    const unsubscribe = client.subscribe("run:r1", recorder().listener);
    await vi.advanceTimersByTimeAsync(0);
    expect(hub.sockets[0].readyState).toBe(1);
    unsubscribe();
    await vi.advanceTimersByTimeAsync(999);
    expect(hub.sockets[0].readyState).toBe(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(hub.sockets[0].readyState).toBe(3);
    client.close();
  });
});
