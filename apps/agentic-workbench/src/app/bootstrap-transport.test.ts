// 043 T051(US5)·T026: 경로는 창이 뜰 때 한 번 정한다(research R3, contracts §3). 연결 정보·handshake·전달 선언 중 하나라도
// 실패하면 그 창은 처음부터 호환 경로다(진단 기록). 네트워크 경로로 시작한 창은 나중에 끊겨도 호환 경로로 바뀌지 않는다.
import { describe, expect, it, vi } from "vitest";

import { getTransport, setTransport, compatTransport } from "@/shared/api/transport";

import { bootstrapTransport, type BootstrapDeps } from "./bootstrap-transport";

function handshake(epoch = "epoch-1") {
  return new Response(
    JSON.stringify({
      selectedProtocolVersion: 1, supportedProtocolVersions: [1], serverVersion: "t", apiMajor: 1,
      contractHash: "h", instanceId: "i", serverEpoch: epoch, storageSchemaVersion: 2, features: [],
    }),
    { status: 200, headers: { "content-type": "application/json" } },
  );
}

function deps(overrides: Partial<BootstrapDeps> = {}): BootstrapDeps & { logs: string[] } {
  const logs: string[] = [];
  return {
    logs,
    getConnection: vi.fn(async () => ({
      baseUrl: "http://127.0.0.1:9",
      token: "t",
      expiresAt: new Date(Date.now() + 900_000).toISOString(),
      incarnation: "inc-1",
    })),
    ensureWindowBench: vi.fn(async () => "bench-1"),
    declareNetworkDelivery: vi.fn(async () => undefined),
    windowLabel: () => "session-1",
    fetch: vi.fn(async () => handshake()) as unknown as typeof fetch,
    log: (line: string) => logs.push(line),
    ...overrides,
  };
}

describe("bootstrapTransport", () => {
  it("chooses the network path after connection, handshake and delivery declaration succeed", async () => {
    setTransport(compatTransport);
    const d = deps();
    const result = await bootstrapTransport(d);
    expect(result.kind).toBe("http");
    expect(getTransport().kind).toBe("http");
    expect(d.declareNetworkDelivery).toHaveBeenCalledWith("inc-1");
    expect(d.logs).toEqual([]);
    result.connection?.close();
  });

  it.each([
    ["connection info", { getConnection: vi.fn(async () => { throw "Workbench HTTP server is not running: address in use"; }) }],
    ["handshake", { fetch: vi.fn(async () => { throw new TypeError("fetch failed"); }) as unknown as typeof fetch }],
    ["delivery declaration", { declareNetworkDelivery: vi.fn(async () => { throw "Window is no longer available."; }) }],
    ["missing incarnation", {
      getConnection: vi.fn(async () => ({ baseUrl: "http://127.0.0.1:9", token: "t", expiresAt: new Date(Date.now() + 900_000).toISOString() })),
    }],
  ] as const)("stays on the compat path when the %s fails and records why", async (_label, override) => {
    setTransport(compatTransport);
    const d = deps(override as Partial<BootstrapDeps>);
    const result = await bootstrapTransport(d);
    expect(result.kind).toBe("compat");
    expect(getTransport()).toBe(compatTransport);
    expect(d.logs).toHaveLength(1);
    expect(d.logs[0]).toMatch(/^\[workbench-client\] using compat path: /);
  });

  it("does not declare network delivery when the handshake fails (events stay on the compat path)", async () => {
    setTransport(compatTransport);
    const d = deps({ fetch: vi.fn(async () => { throw new TypeError("fetch failed"); }) as unknown as typeof fetch });
    await bootstrapTransport(d);
    expect(d.declareNetworkDelivery).not.toHaveBeenCalled();
  });

  it("never switches a network window back to compat after a later disconnect", async () => {
    setTransport(compatTransport);
    let up = true;
    const d = deps({
      fetch: vi.fn(async () => {
        if (!up) {
          throw new TypeError("fetch failed");
        }
        return handshake();
      }) as unknown as typeof fetch,
    });
    const result = await bootstrapTransport(d);
    up = false;
    result.connection?.reportLost();
    expect(getTransport().kind).toBe("http");
    await expect(getTransport().invoke("list_projects")).rejects.toMatch(/연결되어 있지 않아/);
    expect(getTransport().kind).toBe("http");
    result.connection?.close();
  });
});
