// 043 T040(Codex 설계 리뷰 H1, R8): 보관 범위 gap 복구는 gap의 lastSequence로 live를 먼저 확보하고, 그 사이 이벤트는 버퍼에
// 담았다가, hello 뒤 스냅샷을 불러 수신자를 재설정한 다음 스냅샷 기준으로 걸러 넘긴다. 복구 중 새 gap이면 처음부터
// (상한 3회, 넘으면 오류 알림). hello만으로 성공으로 보지 않는다. epochChanged는 알린 뒤 새 세대 처음부터. evicted는 재설정 뒤 끝.
import { describe, expect, it, vi } from "vitest";

import type { EventEnvelope } from "./operation-map";
import { clientFor } from "./testing/event-client-harness";
import { FakeEventHub, until } from "./testing/fake-event-hub";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => (resolve = res));
  return { promise, resolve };
}

describe("event client gap recovery", () => {
  it("secures live from the gap's lastSequence, buffers, then merges after the snapshot", async () => {
    const hub = new FakeEventHub(3);
    const snapshot = deferred<{ lastSequence: number }>();
    const loads: number[] = [];
    const client = clientFor(hub);
    const seen: number[] = [];
    const resets: unknown[] = [];
    client.subscribe(
      "run:r1",
      { onEvent: (event: EventEnvelope) => void seen.push(event.sequence), onReset: (data) => void resets.push(data) },
      {
        after: 1,
        snapshot: {
          load: () => {
            loads.push(hub.lastSequence("run:r1"));
            return snapshot.promise;
          },
          passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence,
        },
      },
    );
    for (let i = 0; i < 6; i += 1) {
      hub.publish("run:r1"); // 1..6, 보관 4..6 → cursor 1은 gap(last 6)
    }
    await until(() => loads.length === 1, "snapshot requested after the live ticket");
    // 스냅샷보다 먼저 확보된 live 표의 cursor는 gap의 lastSequence다.
    expect(hub.ticketRequests.map((request) => request[0].afterSequence)).toEqual([1, 6]);
    expect(seen).toEqual([]); // 재설정 전에는 넘기지 않는다
    hub.publish("run:r1"); // 7: 버퍼
    hub.publish("run:r1"); // 8: 버퍼
    snapshot.resolve({ lastSequence: 7 }); // 스냅샷이 7까지 덮음
    await until(() => seen.length === 1, "buffer merged");
    expect(resets).toEqual([{ lastSequence: 7 }]);
    expect(seen).toEqual([8]); // 7은 스냅샷에 포함, 8만
    hub.publish("run:r1");
    await until(() => seen.length === 2, "live");
    expect(seen).toEqual([8, 9]);
    client.close();
  });

  it("restarts recovery on a new gap and gives up after three attempts", async () => {
    const hub = new FakeEventHub(2);
    const errors: string[] = [];
    const client = clientFor(hub, { onStreamError: (_stream, error) => errors.push(error) });
    let loads = 0;
    client.subscribe(
      "run:r1",
      { onEvent: () => undefined },
      {
        after: 1,
        snapshot: {
          load: async () => {
            loads += 1;
            // 스냅샷을 불러오는 동안 보관 범위가 계속 밀려 새 표도 gap이 된다.
            for (let i = 0; i < 5; i += 1) {
              hub.publish("run:r1");
            }
            hub.sockets[hub.sockets.length - 1].onmessage?.({
              data: JSON.stringify({ type: "gap", streamId: "run:r1", epoch: hub.epoch, reason: "retentionExceeded", lastSequence: hub.lastSequence("run:r1") }),
            });
            return { lastSequence: 0 };
          },
          passes: () => true,
        },
      },
    );
    for (let i = 0; i < 5; i += 1) {
      hub.publish("run:r1");
    }
    await until(() => errors.length === 1, "gives up");
    expect(errors[0]).toMatch(/recovery failed/);
    expect(loads).toBe(3);
    client.close();
  });

  it("does not treat hello as recovery success: a gap right after hello restarts the procedure", async () => {
    const hub = new FakeEventHub(2);
    const client = clientFor(hub);
    const load = vi.fn(async () => ({ lastSequence: hub.lastSequence("run:r1") }));
    const seen: number[] = [];
    client.subscribe("run:r1", { onEvent: (event) => void seen.push(event.sequence) }, {
      after: 1,
      snapshot: { load, passes: (event, data) => event.sequence > (data as { lastSequence: number }).lastSequence },
    });
    for (let i = 0; i < 4; i += 1) {
      hub.publish("run:r1"); // cursor 1 → gap
    }
    await until(() => load.mock.calls.length === 1, "recovered once");
    hub.publish("run:r1");
    await until(() => seen.length === 1, "live after recovery");
    expect(seen).toEqual([5]);
    client.close();
  });

  it("notifies an epoch change and recovers from the start of the new epoch", async () => {
    const hub = new FakeEventHub();
    const epochs: string[] = [];
    const client = clientFor(hub, { onEpochChanged: (epoch) => epochs.push(epoch) });
    const seen: number[] = [];
    const resets: unknown[] = [];
    client.subscribe(
      "run:r1",
      { onEvent: (event) => void seen.push(event.sequence), onReset: (data) => void resets.push(data) },
      { snapshot: { load: async () => ({ lastSequence: 0 }), passes: () => true } },
    );
    hub.publish("run:r1");
    hub.publish("run:r1");
    await until(() => seen.length === 2, "old epoch");
    hub.restart("epoch-2"); // 서버 재기동: 소켓 끊김, 스트림 초기화
    await until(() => epochs.length === 1, "epoch change noticed");
    expect(epochs).toEqual(["epoch-2"]);
    await until(() => resets.length === 1, "reset for the new epoch");
    hub.publish("run:r1");
    await until(() => seen.length === 3, "new epoch live");
    expect(seen).toEqual([1, 2, 1]);
    const last = hub.ticketRequests[hub.ticketRequests.length - 1][0];
    expect(last).toEqual({ streamId: "run:r1", epoch: "epoch-2", afterSequence: 0 });
    client.close();
  });

  it("resets and stops an evicted stream", async () => {
    const hub = new FakeEventHub();
    const client = clientFor(hub);
    const resets: unknown[] = [];
    client.subscribe("run:r1", { onEvent: () => undefined, onReset: (data) => void resets.push(data) }, {
      snapshot: { load: async () => ({ lastSequence: 3, gapDetected: true }), passes: () => true },
    });
    hub.publish("run:r1");
    await until(() => hub.sockets.length === 1, "open");
    hub.evict("run:r1");
    hub.sockets[0].drop();
    await until(() => resets.length === 1, "evicted reset");
    const socketsAfter = hub.sockets.length;
    hub.publish("run:r1");
    for (let i = 0; i < 100; i += 1) {
      await Promise.resolve();
    }
    expect(hub.sockets.length).toBe(socketsAfter); // 끝난 스트림은 다시 연결하지 않는다
    client.close();
  });
});
