import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

export function listAgentExchanges() {
  return invoke<AgentExchange[]>("list_agent_exchanges");
}

function listenWithFallback<T>(
  eventName: string,
  callback: (payload: T) => void,
): Promise<UnlistenFn> {
  let disposed = false;
  const fallbackName = `${eventName}-fallback`;
  const handleFallback = (event: Event) => {
    if (!disposed) {
      callback((event as CustomEvent<T>).detail);
    }
  };
  window.addEventListener(fallbackName, handleFallback);
  return listen<T>(eventName, (event) => {
    if (!disposed) {
      callback(event.payload);
    }
  }).then((unlistenTauri) => () => {
    disposed = true;
    unlistenTauri();
    window.removeEventListener(fallbackName, handleFallback);
  });
}

export function listenAgentExchangeRequests(
  callback: (event: AgentExchangeRequestedEvent) => void,
) {
  return listenWithFallback(AGENT_EXCHANGE_REQUESTED_EVENT, callback);
}

export function listenAgentExchangeStatus(callback: (event: AgentExchange) => void) {
  return listenWithFallback(AGENT_EXCHANGE_STATUS_EVENT, callback);
}
