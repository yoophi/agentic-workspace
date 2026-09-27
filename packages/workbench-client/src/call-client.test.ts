// 043 T017: 호출 클라이언트의 결과 판정(research R6, contracts §4).
// - notApplied: 클라이언트가 스스로 보내지 않은 경우만(연결이 이미 끊김). fetch를 부르지 않는다.
// - unknown: 보내기를 시도한 뒤의 모든 실패(fetch 거절, 응답 파싱 실패). 변경은 재연결 handshake의 세대가 보낼 때와
//   같으면 같은 멱등성 키로 한 번 재시도하고, 세대가 바뀌었으면 다시 보내지 않는다. 조회는 새 요청으로 다시 보낸다.
// - 401은 자격 증명을 한 번 갱신하고 같은 요청(같은 키)으로 한 번 더.
import { describe, expect, it, vi } from "vitest";

import { createWorkbenchClient, type ConnectionPort } from "./call-client";
import { createConnection, type ConnectionInfo } from "./connection";

type Sent = { url: string; body: Record<string, unknown>; token: string };

function fakeConnection(overrides: Partial<ConnectionPort> = {}) {
  let epoch = "epoch-1";
  let token = "token-1";
  let state: ReturnType<ConnectionPort["state"]> = "connected";
  const port: ConnectionPort & {
    setEpoch(value: string): void;
    setState(value: ReturnType<ConnectionPort["state"]>): void;
    lost: number;
  } = {
    lost: 0,
    state: () => state,
    epoch: () => epoch,
    credentials: () => ({ baseUrl: "http://127.0.0.1:9", token }),
    refreshCredentials: vi.fn(async () => {
      token = `token-${Number(token.split("-")[1]) + 1}`;
    }),
    reportLost() {
      port.lost += 1;
    },
    whenConnected: vi.fn(async () => ({ serverEpoch: epoch })),
    setEpoch(value) {
      epoch = value;
    },
    setState(value) {
      state = value;
    },
    ...overrides,
  };
  return port;
}

function json(status: number, body: unknown, contentType = "application/json") {
  return new Response(JSON.stringify(body), { status, headers: { "content-type": contentType } });
}

function problem(status: number, code: string, message: string, extra: Record<string, unknown> = {}) {
  return json(
    status,
    { code, message, retryable: false, outcome: "notApplied", requestId: "srv", type: `urn:aw:fault:${code}`, title: code, status, ...extra },
    "application/problem+json",
  );
}

function recordingFetch(respond: (sent: Sent, index: number) => Response | Promise<Response>) {
  const sent: Sent[] = [];
  const fetchImpl = vi.fn(async (url: string | URL | Request, init?: RequestInit) => {
    const item: Sent = {
      url: String(url),
      body: JSON.parse(String(init?.body)),
      token: String((init?.headers as Record<string, string>).authorization).replace("Bearer ", ""),
    };
    sent.push(item);
    return respond(item, sent.length - 1);
  });
  return { sent, fetchImpl: fetchImpl as unknown as typeof fetch };
}

const complete = (output: unknown) => json(200, { kind: "complete", output });

describe("createWorkbenchClient.call", () => {
  it("does not send and reports notApplied while the connection is already down", async () => {
    const connection = fakeConnection();
    connection.setState("reconnecting");
    const { sent, fetchImpl } = recordingFetch(() => complete([]));
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.list", {})).toEqual({ kind: "notApplied", reason: "offline" });
    expect(await client.call("project.delete", { id: "p1" })).toEqual({ kind: "notApplied", reason: "offline" });
    expect(sent).toHaveLength(0);
  });

  it("returns ok for a complete reply and fault for a problem reply", async () => {
    const connection = fakeConnection();
    const { sent, fetchImpl } = recordingFetch((item) =>
      item.body.operation === "project.list" ? complete([{ id: "p1" }]) : problem(409, "conflict", "Project name is taken."),
    );
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.list", {})).toEqual({ kind: "ok", output: [{ id: "p1" }], revision: undefined });
    const failed = await client.call("project.delete", { id: "p1" });
    expect(failed.kind).toBe("fault");
    expect(failed.kind === "fault" && failed.fault.message).toBe("Project name is taken.");
    expect(sent[0].url).toBe("http://127.0.0.1:9/v1/calls");
    expect(sent[0].body.idempotencyKey).toBeUndefined();
    expect(typeof sent[1].body.idempotencyKey).toBe("string");
    expect(connection.lost).toBe(0);
  });

  it("retries a lost command once with the same idempotency key in the same epoch", async () => {
    const connection = fakeConnection();
    const { sent, fetchImpl } = recordingFetch((_item, index) => {
      if (index === 0) {
        throw new TypeError("fetch failed");
      }
      return complete({ id: "p1" });
    });
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.delete", { id: "p1" })).toEqual({ kind: "ok", output: { id: "p1" }, revision: undefined });
    expect(sent).toHaveLength(2);
    expect(sent[1].body.idempotencyKey).toBe(sent[0].body.idempotencyKey);
    expect(connection.lost).toBe(1);
    expect(connection.whenConnected).toHaveBeenCalledTimes(1);
  });

  it("never resends a lost command after the server epoch changed", async () => {
    const connection = fakeConnection({
      whenConnected: vi.fn(async () => ({ serverEpoch: "epoch-2" })),
    });
    const { sent, fetchImpl } = recordingFetch(() => {
      throw new TypeError("fetch failed");
    });
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.delete", { id: "p1" })).toEqual({ kind: "unknown", reason: "epochChanged" });
    expect(sent).toHaveLength(1);
  });

  it("treats an unreadable reply after sending as a lost response, not as notApplied", async () => {
    const connection = fakeConnection({
      whenConnected: vi.fn(async () => ({ serverEpoch: "epoch-2" })),
    });
    const { sent, fetchImpl } = recordingFetch(() => new Response("<html>proxy", { status: 200 }));
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.delete", { id: "p1" })).toEqual({ kind: "unknown", reason: "epochChanged" });
    expect(sent).toHaveLength(1);
  });

  it("reports unknown when the single same-epoch retry is also lost", async () => {
    const connection = fakeConnection();
    const { sent, fetchImpl } = recordingFetch(() => {
      throw new TypeError("fetch failed");
    });
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.delete", { id: "p1" })).toEqual({ kind: "unknown", reason: "lost" });
    expect(sent).toHaveLength(2);
    expect(sent[1].body.idempotencyKey).toBe(sent[0].body.idempotencyKey);
  });

  it("resends a lost query as a new request after reconnecting, even across epochs", async () => {
    const connection = fakeConnection({
      whenConnected: vi.fn(async () => ({ serverEpoch: "epoch-2" })),
    });
    const { sent, fetchImpl } = recordingFetch((_item, index) => {
      if (index === 0) {
        throw new TypeError("fetch failed");
      }
      return complete([]);
    });
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.list", {})).toEqual({ kind: "ok", output: [], revision: undefined });
    expect(sent).toHaveLength(2);
    expect(sent[1].body.requestId).not.toBe(sent[0].body.requestId);
  });

  it("refreshes credentials once on 401 and retries with the same key", async () => {
    const connection = fakeConnection();
    const { sent, fetchImpl } = recordingFetch((_item, index) =>
      index === 0 ? problem(401, "unauthenticated", "credentials are missing or invalid.") : complete(null),
    );
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    expect(await client.call("project.delete", { id: "p1" })).toEqual({ kind: "ok", output: null, revision: undefined });
    expect(connection.refreshCredentials).toHaveBeenCalledTimes(1);
    expect(sent.map((item) => item.token)).toEqual(["token-1", "token-2"]);
    expect(sent[1].body.idempotencyKey).toBe(sent[0].body.idempotencyKey);
  });

  it("returns the second 401 as a fault instead of refreshing forever", async () => {
    const connection = fakeConnection();
    const { sent, fetchImpl } = recordingFetch(() => problem(401, "unauthenticated", "credentials are missing or invalid."));
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    const outcome = await client.call("project.list", {});
    expect(outcome.kind).toBe("fault");
    expect(sent).toHaveLength(2);
    expect(connection.refreshCredentials).toHaveBeenCalledTimes(1);
  });

  it("uses a caller-provided idempotency key for a user action", async () => {
    const connection = fakeConnection();
    const { sent, fetchImpl } = recordingFetch(() => complete(null));
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    await client.call("run.sendPrompt", { benchId: "b", runId: "r", prompt: "hi" }, { idempotencyKey: "exchange-delivery:x1" });
    expect(sent[0].body.idempotencyKey).toBe("exchange-delivery:x1");
  });
});

// 사용자 검토(043 T022): 실제 연결 수명과 함께. 서버가 다른 포트·새 세대로 다시 뜨면 연결은 새 끝점을 찾아 성공하지만,
// 응답을 잃은 변경은 새 서버로 다시 보내지 않는다. 401 갱신으로 끝점이 바뀌는 경로에서도 같다.
describe("call client with a real connection across a server restart", () => {
  function handshake(epoch: string) {
    return json(200, {
      selectedProtocolVersion: 1, supportedProtocolVersions: [1], serverVersion: "t", apiMajor: 1,
      contractHash: "h", instanceId: "i", serverEpoch: epoch, storageSchemaVersion: 2, features: [],
    });
  }

  function servers() {
    const log: Array<{ url: string; operation?: string; key?: string }> = [];
    let issued = 0;
    let restarted = false;
    const fetchConnection = async (): Promise<ConnectionInfo> => {
      issued += 1;
      return {
        baseUrl: restarted ? "http://127.0.0.1:2222" : "http://127.0.0.1:1111",
        token: `token-${issued}`,
        expiresAt: new Date(Date.now() + 15 * 60 * 1000).toISOString(),
      };
    };
    const behavior = { oldCall: "lose" as "lose" | "unauthorized" | "complete" };
    const fetchImpl = (async (url: string | URL | Request, init?: RequestInit) => {
      const target = String(url);
      const body = init?.body ? JSON.parse(String(init.body)) : {};
      log.push({ url: target, operation: body.operation, key: body.idempotencyKey });
      const old = target.startsWith("http://127.0.0.1:1111");
      if (old && restarted) {
        throw new TypeError("fetch failed: connect ECONNREFUSED");
      }
      if (target.endsWith("/v1/system/handshake")) {
        return handshake(old ? "epoch-1" : "epoch-2");
      }
      if (old && behavior.oldCall === "lose") {
        restarted = true; // 요청은 도착했을 수 있고, 답하기 전에 서버가 죽었다
        throw new TypeError("fetch failed: socket hang up");
      }
      if (old && behavior.oldCall === "unauthorized") {
        restarted = true;
        return problem(401, "unauthenticated", "credentials are missing or invalid.");
      }
      return complete(null);
    }) as unknown as typeof fetch;
    const connection = createConnection({ fetchConnection, fetch: fetchImpl, random: () => 0.5 });
    return { connection, fetchImpl, log, behavior, lose: () => { restarted = true; } };
  }

  it("finds the new endpoint but never resends the lost mutation to the new epoch", async () => {
    const { connection, fetchImpl, log } = servers();
    await connection.start();
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    const outcome = await client.call("project.delete", { id: "p1" });
    expect(outcome).toEqual({ kind: "unknown", reason: "epochChanged" });
    expect(connection.epoch()).toBe("epoch-2");
    expect(connection.credentials().baseUrl).toBe("http://127.0.0.1:2222");
    const mutations = log.filter((entry) => entry.operation === "project.delete");
    expect(mutations.map((entry) => entry.url)).toEqual(["http://127.0.0.1:1111/v1/calls"]);
    // 새 세대에서의 새 호출은 보낸다.
    expect((await client.call("project.list", {})).kind).toBe("ok");
  });

  it("does not resend an uncertain mutation when a 401 refresh moves it to a new epoch", async () => {
    const { connection, fetchImpl, log, behavior } = servers();
    await connection.start();
    const client = createWorkbenchClient({ connection, fetch: fetchImpl });
    // 첫 시도는 응답 유실(같은 서버로 재연결되는 것처럼 보이게 handshake는 옛 세대), 재시도는 401 → 갱신이 새 끝점·새 세대로.
    let attempt = 0;
    const wrapped = (async (url: string | URL | Request, init?: RequestInit) => {
      const target = String(url);
      if (target.endsWith("/v1/calls") && target.startsWith("http://127.0.0.1:1111")) {
        attempt += 1;
        log.push({ url: target, operation: JSON.parse(String(init?.body)).operation });
        if (attempt === 1) {
          throw new TypeError("fetch failed: socket hang up");
        }
        behavior.oldCall = "unauthorized";
        return (fetchImpl as unknown as (u: string, i?: RequestInit) => Promise<Response>)(target, init);
      }
      return (fetchImpl as unknown as (u: string, i?: RequestInit) => Promise<Response>)(target, init);
    }) as unknown as typeof fetch;
    const retrying = createWorkbenchClient({ connection, fetch: wrapped });
    // 재연결 handshake가 옛 세대로 성공하도록 연결을 끊지 않은 채 두고(reportLost는 재연결 루프를 돈다) 시작한다.
    const outcome = await retrying.call("project.delete", { id: "p1" });
    expect(outcome).toEqual({ kind: "unknown", reason: "epochChanged" });
    const newServerMutations = log.filter(
      (entry) => entry.operation === "project.delete" && entry.url.startsWith("http://127.0.0.1:2222"),
    );
    expect(newServerMutations).toHaveLength(0);
    expect(connection.epoch()).toBe("epoch-2");
    void client;
  });
});
