import { invoke, listen, type EventCallback } from "@/shared/api/transport";

import type {
  AgentExchange,
  AgentExchangeRequestedEvent,
  AgentExchangeStatus,
  AgentPromptDelivery,
  AgentWorkspaceSnapshotInput,
} from "@/entities/agent-run/model/agent-exchange";

export const AGENT_EXCHANGE_REQUESTED_EVENT = "agent-exchange-requested";
export const AGENT_EXCHANGE_STATUS_EVENT = "agent-exchange-status";

export type SendAgentExchangeInput = {
  requestId: string;
  sourcePanelId: string;
  sourceRunId: string | null;
  targetPanelId: string;
  targetRunId: string | null;
  message: string;
  delivery: AgentPromptDelivery;
};

export type AcknowledgeAgentExchangeInput = {
  requestId: string;
  targetPanelId: string;
  outcome: Extract<
    AgentExchangeStatus,
    "delivered" | "rejected" | "failed" | "cancelled"
  >;
  reason?: string | null;
};

export function syncAgentWorkspace(request: AgentWorkspaceSnapshotInput) {
  return invoke<{ revision: number; acceptedPanels: number }>("sync_agent_workspace", {
    request,
  });
}

export function sendAgentExchange(request: SendAgentExchangeInput) {
  return invoke<AgentExchange>("send_agent_exchange", { request });
}

export function acknowledgeAgentExchange(request: AcknowledgeAgentExchangeInput) {
  return invoke<AgentExchange>("acknowledge_agent_exchange", { request });
}

/** 화면 대기열에서 지운 교환 prompt의 전달 포기(044 Codex r7). 이미 확인한 교환을 서버에서 끝낸다(멱등). */
export function discardAgentExchangeDelivery(requestId: string) {
  return invoke<null>("discard_agent_exchange_delivery", { requestId });
}

export function listAgentExchanges() {
  return invoke<AgentExchange[]>("list_agent_exchanges");
}

export function listenAgentExchangeRequests(callback: EventCallback<AgentExchangeRequestedEvent>) {
  return listen(AGENT_EXCHANGE_REQUESTED_EVENT, callback);
}

export function listenAgentExchangeStatus(callback: EventCallback<AgentExchange>) {
  return listen(AGENT_EXCHANGE_STATUS_EVENT, callback);
}
