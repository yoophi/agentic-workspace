// 교환 원장·재조정(043 T035, research R8 교환 전달 재조정, Codex 설계 리뷰 H3). 서버는 교환 요청을 agent에 직접 보내지 않는다 —
// 화면이 requested 이벤트를 받아 대상 패널로 라우팅하고 확인(ack)해야 전달이 끝난다. 이벤트를 잃으면(보관 한도·끊김) 스냅샷의
// `accepted` 교환으로 다시 맞춘다. 원장은 창 메모리에 둔다: 라우팅은 한 번, 확인은 성공할 때까지(서버 확인은 requestId 멱등).
import type {
  AgentExchange,
  AgentExchangeRequestedEvent,
} from "@/entities/agent-run/model/agent-exchange";

export interface ExchangeAck {
  requestId: string;
  targetPanelId: string;
  outcome: "delivered" | "rejected";
  reason: string | null;
}

export type RouteResult = { routed: true } | { routed: false; reason: string };

export interface ExchangeReconcilerDeps {
  route(request: AgentExchangeRequestedEvent): RouteResult;
  acknowledge(ack: ExchangeAck): Promise<void>;
}

interface Entry {
  ack?: ExchangeAck;
  acked: boolean;
  running?: Promise<void>;
}

const TERMINAL: ReadonlySet<string> = new Set(["delivered", "rejected", "failed", "cancelled"]);

/** 교환 prompt를 run에 보낼 때의 멱등성 키 — 원장이 없는 새 창에서 다시 라우팅돼도 같은 세대에서는 agent에 한 번만 간다. */
export function exchangeDeliveryKey(requestId: string) {
  return `exchange-delivery:${requestId}`;
}

function requestOf(exchange: AgentExchange): AgentExchangeRequestedEvent {
  const { requestId, source, target, message, delivery, createdAt } = exchange;
  return { requestId, source, target, message, delivery, createdAt };
}

export function createExchangeReconciler(deps: ExchangeReconcilerDeps) {
  const ledger = new Map<string, Entry>();

  async function run(request: AgentExchangeRequestedEvent, entry: Entry) {
    if (!entry.ack) {
      const result = deps.route(request);
      entry.ack = {
        requestId: request.requestId,
        targetPanelId: request.target.panelId,
        outcome: result.routed ? "delivered" : "rejected",
        reason: result.routed ? null : result.reason,
      };
    }
    try {
      await deps.acknowledge(entry.ack);
      entry.acked = true;
    } catch {
      // 확인 실패: 다음 재조정이 확인만 다시 한다(라우팅은 다시 하지 않는다).
    }
  }

  async function handleRequested(request: AgentExchangeRequestedEvent): Promise<void> {
    let entry = ledger.get(request.requestId);
    if (!entry) {
      entry = { acked: false };
      ledger.set(request.requestId, entry);
    }
    while (entry.running) {
      await entry.running;
    }
    if (entry.acked) {
      return;
    }
    const current = entry;
    current.running = run(request, current).finally(() => {
      current.running = undefined;
    });
    await current.running;
  }

  return {
    handleRequested,
    /** 스냅샷(`exchange.list`)의 확인 전 교환을 맞춘다. */
    async reconcile(exchanges: AgentExchange[]) {
      for (const exchange of exchanges) {
        if (TERMINAL.has(exchange.status)) {
          this.observeStatus(exchange);
          continue;
        }
        if (exchange.status === "accepted") {
          await handleRequested(requestOf(exchange));
        }
      }
    },
    /** 서버가 알린 종결 상태 = 확인됨. */
    observeStatus(exchange: AgentExchange) {
      if (!TERMINAL.has(exchange.status)) {
        return;
      }
      const entry = ledger.get(exchange.requestId) ?? { acked: false };
      entry.acked = true;
      ledger.set(exchange.requestId, entry);
    },
  };
}

export type ExchangeReconciler = ReturnType<typeof createExchangeReconciler>;
