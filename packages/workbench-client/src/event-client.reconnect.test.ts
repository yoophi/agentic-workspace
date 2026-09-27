// 043 T039(SC-004): 강제 끊김 100회 이상 — 받았지만 반영 전 끊김, 수신자 교체 중 끊김을 섞어도 수신자마다 순번이 1..N으로
// 빠짐·중복 없이 반영된다. 재연결 cursor는 반영 완료 최솟값이고, subscriberLagged gap은 같은 cursor로 다시 연결한다.
import { describe, expect, it } from "vitest";

import type { EventEnvelope } from "./operation-map";
import { clientFor } from "./testing/event-client-harness";
import { FakeEventHub, until } from "./testing/fake-event-hub";

/** 결정적 의사 난수(시험 재현성). */
function rng(seed: number) {
  let state = seed;
  return () => {
    state = (state * 1103515245 + 12345) % 2 ** 31;
    return state / 2 ** 31;
  };
}

describe("event client reconnects without loss or duplication", () => {
  it("survives 120 forced disconnects with slow async listeners and listener swaps", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub, { graceMs: 60_000 });
    const random = rng(7);
    const stable: number[] = [];
    const slow: number[] = [];
    let swapped: number[] = [];
    const swappedHistory: number[][] = [];
    client.subscribe("run:r1", { onEvent: (event: EventEnvelope) => void stable.push(event.sequence) });
    client.subscribe("run:r1", {
      onEvent: async (event: EventEnvelope) => {
        // 몇 개는 마이크로태스크 여러 번 뒤에 끝난다(받았지만 반영 전 구간을 만든다).
        for (let i = 0; i < Math.floor(random() * 4); i += 1) {
          await Promise.resolve();
        }
        slow.push(event.sequence);
      },
    });
    let unsubscribeSwapped = client.subscribe("run:r1", { onEvent: (event) => void swapped.push(event.sequence) });

    let published = 0;
    let drops = 0;
    for (let round = 0; round < 120; round += 1) {
      const burst = 1 + Math.floor(random() * 3);
      for (let i = 0; i < burst; i += 1) {
        hub.publish("run:r1");
        published += 1;
      }
      if (round % 10 === 5) {
        // 교체 중 끊김: 이 수신자가 반영한 지점을 기록하고 새 수신자로 바꾼 뒤 곧바로 끊는다.
        unsubscribeSwapped();
        swappedHistory.push(swapped);
        const resumeAt = swapped.length === 0 ? 0 : swapped[swapped.length - 1];
        swapped = [];
        unsubscribeSwapped = client.subscribe(
          "run:r1",
          { onEvent: (event) => void swapped.push(event.sequence) },
          { after: resumeAt },
        );
      }
      await until(() => hub.sockets.some((socket) => socket.readyState === 1), `open before drop ${round}`);
      const open = hub.sockets.filter((socket) => socket.readyState === 1);
      const dropped = open[open.length - 1];
      dropped.drop();
      drops += 1;
      await until(
        () => hub.sockets.some((socket) => socket !== dropped && socket.readyState === 1),
        `reconnect ${round}`,
      );
    }
    await until(() => stable.length === published && slow.length === published, "all delivered");
    await until(() => (swapped[swapped.length - 1] ?? 0) === published, "swapped listener caught up");

    const expected = Array.from({ length: published }, (_, index) => index + 1);
    expect(stable).toEqual(expected);
    expect(slow).toEqual(expected);
    // 교체된 수신자들: 각자 이어 받은 구간이 겹치지도 비지도 않고 1..N을 덮는다.
    const joined = [...swappedHistory, swapped].flat();
    expect(joined).toEqual(expected);
    expect(drops).toBe(120);
    client.close();
  });

  it("reconnects from the same cursor after a subscriberLagged gap", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const seen: number[] = [];
    client.subscribe("run:r1", { onEvent: (event) => void seen.push(event.sequence) });
    hub.publish("run:r1");
    hub.publish("run:r1");
    await until(() => seen.length === 2, "two");
    const socket = hub.sockets[0];
    socket.onmessage?.({ data: JSON.stringify({ type: "gap", streamId: "run:r1", epoch: hub.epoch, reason: "subscriberLagged" }) });
    await until(() => hub.sockets.length === 2 && hub.sockets[1].readyState === 1, "reconnected");
    expect(hub.ticketRequests[hub.ticketRequests.length - 1]).toEqual([{ streamId: "run:r1", epoch: "epoch-1", afterSequence: 2 }]);
    hub.publish("run:r1");
    await until(() => seen.length === 3, "continues");
    expect(seen).toEqual([1, 2, 3]);
    client.close();
  });
});
