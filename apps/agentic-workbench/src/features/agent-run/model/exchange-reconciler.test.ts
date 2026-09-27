// 043 T031(Codex 설계 리뷰 H3): 교환 요청은 화면이 대상 패널로 라우팅하고 확인(ack)해야 agent에 전달된다. 창 원장
// (requestId → 라우팅·확인)으로 라이브 요청과 스냅샷 재조정을 합쳐, 라우팅과 확인이 각각 정확히 한 번 일어나게 한다:
// - 새 요청: 라우팅 + 확인 1회
// - 스냅샷의 `accepted`(요청 발행 뒤 확인 전, T003) 교환 중 라우팅 전인 것: 라우팅 + 확인
// - 라우팅 뒤 확인이 실패한 것: 확인만 다시(서버 확인은 requestId 멱등)
// - 확인이 끝난 것(원장 또는 서버 상태): 다시 하지 않음
// - 원장이 없는 새 창(새로고침): 다시 라우팅될 수 있다 — agent 중복 전달은 run 전송 키 `exchange-delivery:<requestId>`가 막는다.
import { describe, expect, it, vi } from "vitest";

import type { AgentExchange, AgentExchangeRequestedEvent } from "@/entities/agent-run/model/agent-exchange";

import { createExchangeReconciler, exchangeDeliveryKey } from "./exchange-reconciler";

function exchange(requestId: string, status: AgentExchange["status"], panelId = "p2"): AgentExchange {
  return {
    requestId,
    source: { panelId: "main", title: "Main", runId: "r1" },
    target: { panelId, title: "Panel", runId: "r2" },
    message: `message ${requestId}`,
    delivery: "queue",
    status,
    createdAt: "2026-09-28T00:00:00Z",
    updatedAt: "2026-09-28T00:00:00Z",
  } as AgentExchange;
}

function requested(item: AgentExchange): AgentExchangeRequestedEvent {
  const { requestId, source, target, message, delivery, createdAt } = item;
  return { requestId, source, target, message, delivery, createdAt };
}

function setup(acknowledge = vi.fn(async () => undefined)) {
  const route = vi.fn((request: AgentExchangeRequestedEvent) => ({ routed: true as const, requestId: request.requestId }));
  const reconciler = createExchangeReconciler({
    route: (request) => (route(request), { routed: true }),
    acknowledge,
  });
  return { reconciler, route, acknowledge };
}

describe("exchange reconciler", () => {
  it("routes and acknowledges a live request exactly once even if it is seen again", async () => {
    const { reconciler, route, acknowledge } = setup();
    const x1 = exchange("x1", "accepted");
    await reconciler.handleRequested(requested(x1));
    await reconciler.handleRequested(requested(x1));
    await reconciler.reconcile([x1]);
    expect(route).toHaveBeenCalledTimes(1);
    expect(acknowledge).toHaveBeenCalledTimes(1);
    expect(acknowledge).toHaveBeenCalledWith({ requestId: "x1", targetPanelId: "p2", outcome: "delivered", reason: null });
  });

  it("delivers accepted exchanges from a snapshot whose request event was lost", async () => {
    const { reconciler, route, acknowledge } = setup();
    await reconciler.reconcile([exchange("x1", "accepted"), exchange("x2", "delivered"), exchange("x3", "rejected")]);
    expect(route.mock.calls.map(([request]) => request.requestId)).toEqual(["x1"]);
    expect(acknowledge).toHaveBeenCalledTimes(1);
  });

  it("retries only the acknowledgement when it failed after routing", async () => {
    const acknowledge = vi
      .fn<(ack: unknown) => Promise<void>>()
      .mockRejectedValueOnce("network")
      .mockResolvedValue(undefined);
    const { reconciler, route } = setup(acknowledge as never);
    const x1 = exchange("x1", "accepted");
    await reconciler.handleRequested(requested(x1));
    expect(route).toHaveBeenCalledTimes(1);
    expect(acknowledge).toHaveBeenCalledTimes(1);
    await reconciler.reconcile([x1]); // 서버는 여전히 accepted(확인 실패)
    expect(route).toHaveBeenCalledTimes(1); // 다시 라우팅하지 않는다
    expect(acknowledge).toHaveBeenCalledTimes(2);
    await reconciler.reconcile([exchange("x1", "delivered")]);
    expect(acknowledge).toHaveBeenCalledTimes(2);
  });

  it("acknowledges a rejected routing with the rejection reason", async () => {
    const acknowledge = vi.fn(async () => undefined);
    const reconciler = createExchangeReconciler({
      route: () => ({ routed: false, reason: "missing-target" }),
      acknowledge,
    });
    await reconciler.handleRequested(requested(exchange("x9", "accepted", "gone")));
    expect(acknowledge).toHaveBeenCalledWith({ requestId: "x9", targetPanelId: "gone", outcome: "rejected", reason: "missing-target" });
  });

  it("treats a server-confirmed terminal status as acknowledged", async () => {
    const { reconciler, route, acknowledge } = setup();
    reconciler.observeStatus(exchange("x1", "delivered"));
    await reconciler.handleRequested(requested(exchange("x1", "accepted")));
    expect(route).not.toHaveBeenCalled();
    expect(acknowledge).not.toHaveBeenCalled();
  });

  it("serializes concurrent handling of the same request", async () => {
    let release!: () => void;
    const acknowledge = vi.fn(() => new Promise<void>((resolve) => (release = resolve)));
    const { reconciler, route } = setup(acknowledge as never);
    const x1 = exchange("x1", "accepted");
    const first = reconciler.handleRequested(requested(x1));
    const second = reconciler.reconcile([x1]);
    release();
    await Promise.all([first, second]);
    expect(route).toHaveBeenCalledTimes(1);
    expect(acknowledge).toHaveBeenCalledTimes(1);
  });

  it("derives the run delivery idempotency key from the request id", () => {
    expect(exchangeDeliveryKey("x1")).toBe("exchange-delivery:x1");
  });
});
