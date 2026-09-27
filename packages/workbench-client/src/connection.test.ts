// 043 T018: 연결 수명(research R5·R9·R10). 자격 증명은 수명의 80%에서 갱신하고, 8시간 동안 어느 시점에도 만료된 토큰으로
// 호출하지 않는다(SC-006). 끊김을 알리면 backoff로 handshake를 다시 하고, 세대가 바뀌면 알린다.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { createConnection, type ConnectionInfo } from "./connection";

const FIFTEEN_MINUTES = 15 * 60 * 1000;

function handshakeResponse(epoch: string) {
  return new Response(
    JSON.stringify({
      selectedProtocolVersion: 1,
      supportedProtocolVersions: [1],
      serverVersion: "t",
      apiMajor: 1,
      contractHash: "h",
      instanceId: "i",
      serverEpoch: epoch,
      storageSchemaVersion: 2,
      features: [],
    }),
    { status: 200, headers: { "content-type": "application/json" } },
  );
}

describe("createConnection", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-09-28T00:00:00Z"));
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  function setup(epochs: string[] = ["epoch-1"]) {
    let issued = 0;
    const fetchConnection = vi.fn(async (): Promise<ConnectionInfo> => {
      issued += 1;
      return {
        baseUrl: "http://127.0.0.1:9",
        token: `token-${issued}`,
        expiresAt: new Date(Date.now() + FIFTEEN_MINUTES).toISOString(),
        incarnation: "inc-1",
      };
    });
    let handshakes = 0;
    const fetchImpl = vi.fn(async () => {
      const epoch = epochs[Math.min(handshakes, epochs.length - 1)];
      handshakes += 1;
      return handshakeResponse(epoch);
    }) as unknown as typeof fetch;
    const connection = createConnection({ fetchConnection, fetch: fetchImpl, random: () => 0.5 });
    return { connection, fetchConnection, fetchImpl };
  }

  it("bootstraps with a handshake and exposes epoch and credentials", async () => {
    const { connection, fetchImpl } = setup();
    const handshake = await connection.start();
    expect(handshake.serverEpoch).toBe("epoch-1");
    expect(connection.state()).toBe("connected");
    expect(connection.epoch()).toBe("epoch-1");
    expect(connection.credentials()).toEqual({ baseUrl: "http://127.0.0.1:9", token: "token-1" });
    expect(connection.incarnation()).toBe("inc-1");
    expect(vi.mocked(fetchImpl).mock.calls[0][0]).toBe("http://127.0.0.1:9/v1/system/handshake");
  });

  it("refreshes at 80% of the lifetime and never holds an expired token over 8 hours", async () => {
    const { connection, fetchConnection } = setup();
    await connection.start();
    const eightHours = 8 * 60 * 60 * 1000;
    const step = 60 * 1000;
    for (let elapsed = 0; elapsed < eightHours; elapsed += step) {
      await vi.advanceTimersByTimeAsync(step);
      const token = connection.credentials().token;
      const expiresAt = connection.expiresAt();
      expect(expiresAt, `token ${token} at ${elapsed / step} min`).toBeGreaterThan(Date.now());
    }
    // 15분 토큰을 12분마다 갱신: 8시간 / 12분 = 40회 + 처음 1회.
    expect(fetchConnection).toHaveBeenCalledTimes(41);
  });

  it("refreshCredentials fetches a new token immediately", async () => {
    const { connection } = setup();
    await connection.start();
    await connection.refreshCredentials();
    expect(connection.credentials().token).toBe("token-2");
  });

  it("reconnects with backoff after a reported loss and notifies an epoch change", async () => {
    const { connection, fetchImpl, fetchConnection } = setup(["epoch-1", "down", "down", "epoch-2"]);
    vi.mocked(fetchImpl).mockImplementation((async () => {
      const call = vi.mocked(fetchImpl).mock.calls.length;
      if (call === 2 || call === 3) {
        throw new TypeError("fetch failed");
      }
      return handshakeResponse(call === 1 ? "epoch-1" : "epoch-2");
    }) as unknown as typeof fetch);
    const states: string[] = [];
    const epochs: string[] = [];
    connection.onState((state) => states.push(state));
    connection.onEpochChanged((epoch) => epochs.push(epoch));
    await connection.start();

    connection.reportLost();
    connection.reportLost(); // 중복 보고는 재연결 루프를 하나만 돌린다
    expect(connection.state()).toBe("reconnecting");
    const reconnected = connection.whenConnected();
    // 첫 시도: handshake 실패(2번째 fetch) → 연결 정보 재발견 → handshake 실패(3번째) → 250ms 대기.
    await vi.advanceTimersByTimeAsync(0);
    expect(connection.state()).toBe("reconnecting");
    // 두 번째 시도: handshake 성공(4번째, 새 세대).
    await vi.advanceTimersByTimeAsync(250);
    expect((await reconnected).serverEpoch).toBe("epoch-2");
    expect(fetchConnection).toHaveBeenCalledTimes(2); // start 1 + 실패 뒤 재발견 1
    expect(connection.state()).toBe("connected");
    expect(epochs).toEqual(["epoch-2"]);
    // onState는 구독 즉시 현재 상태(start 전 `connecting`)를 한 번 알린 뒤 변화를 알린다.
    expect(states).toEqual(["connecting", "connected", "reconnecting", "connected"]);
    expect(vi.mocked(fetchImpl)).toHaveBeenCalledTimes(4);
  });

  it("caps the backoff at 10 seconds and reports disconnected after repeated failures", async () => {
    const { connection, fetchImpl } = setup();
    await connection.start();
    vi.mocked(fetchImpl).mockImplementation((async () => {
      throw new TypeError("fetch failed");
    }) as unknown as typeof fetch);
    connection.reportLost();
    const delays: number[] = [];
    let last = vi.mocked(fetchImpl).mock.calls.length;
    for (let i = 0; i < 12; i += 1) {
      let waited = 0;
      while (vi.mocked(fetchImpl).mock.calls.length === last) {
        await vi.advanceTimersByTimeAsync(50);
        waited += 50;
      }
      last = vi.mocked(fetchImpl).mock.calls.length;
      delays.push(waited);
    }
    expect(Math.max(...delays)).toBeLessThanOrEqual(10_000);
    expect(delays[delays.length - 1]).toBeGreaterThanOrEqual(10_000 * 0.8);
    expect(connection.state()).toBe("disconnected");
  });

  it("rediscovers the endpoint when the old port refuses connections after a server restart", async () => {
    let issued = 0;
    const fetchConnection = vi.fn(async (): Promise<ConnectionInfo> => {
      issued += 1;
      return {
        baseUrl: issued === 1 ? "http://127.0.0.1:1111" : "http://127.0.0.1:2222",
        token: `token-${issued}`,
        expiresAt: new Date(Date.now() + FIFTEEN_MINUTES).toISOString(),
        incarnation: "inc-1",
      };
    });
    const fetchImpl = vi.fn(async (url: string | URL | Request) => {
      if (String(url).startsWith("http://127.0.0.1:1111") && issued > 1) {
        throw new TypeError("fetch failed: connect ECONNREFUSED");
      }
      return handshakeResponse(String(url).startsWith("http://127.0.0.1:2222") ? "epoch-2" : "epoch-1");
    }) as unknown as typeof fetch;
    const connection = createConnection({ fetchConnection, fetch: fetchImpl, random: () => 0.5 });
    const epochs: string[] = [];
    connection.onEpochChanged((epoch) => epochs.push(epoch));
    await connection.start();
    expect(connection.credentials().baseUrl).toBe("http://127.0.0.1:1111");

    // 서버가 다른 포트로 다시 떴다: 옛 포트는 연결 거부(401이 오지 않는다).
    issued = 1;
    vi.mocked(fetchImpl).mockImplementation((async (url: string | URL | Request) => {
      if (String(url).startsWith("http://127.0.0.1:1111")) {
        throw new TypeError("fetch failed: connect ECONNREFUSED");
      }
      return handshakeResponse("epoch-2");
    }) as unknown as typeof fetch);
    connection.reportLost();
    const reconnected = connection.whenConnected();
    await vi.advanceTimersByTimeAsync(0);
    expect((await reconnected).serverEpoch).toBe("epoch-2");
    expect(connection.credentials().baseUrl).toBe("http://127.0.0.1:2222");
    expect(epochs).toEqual(["epoch-2"]);
  });

  it("start fails when the connection info or the handshake is unavailable", async () => {
    const failing = createConnection({
      fetchConnection: async () => {
        throw new Error("Workbench HTTP server is not running: address in use");
      },
      fetch: vi.fn() as unknown as typeof fetch,
    });
    await expect(failing.start()).rejects.toThrow("address in use");
  });
});
