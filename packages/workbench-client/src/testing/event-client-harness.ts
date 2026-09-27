// 시험 전용: 가짜 hub에 붙은 이벤트 클라이언트.
import { vi } from "vitest";

import { createEventClient, type EventConnectionPort } from "../event-client";
import type { FakeEventHub } from "./fake-event-hub";

export function connectionFor(hub: FakeEventHub): EventConnectionPort {
  return {
    credentials: () => ({ baseUrl: "http://127.0.0.1:9", token: "token" }),
    epoch: () => hub.epoch,
    reportLost: vi.fn(),
    whenConnected: vi.fn(async () => ({ serverEpoch: hub.epoch })),
    refreshCredentials: vi.fn(async () => undefined),
  };
}

export function clientFor(hub: FakeEventHub, overrides: Partial<Parameters<typeof createEventClient>[0]> = {}) {
  return createEventClient({
    connection: connectionFor(hub),
    fetch: hub.fetch,
    openSocket: hub.openSocket as never,
    graceMs: 0,
    random: () => 0.5,
    ...overrides,
  });
}

