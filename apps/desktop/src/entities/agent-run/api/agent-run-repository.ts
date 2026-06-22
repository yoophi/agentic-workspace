import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  AgentDescriptor,
  AgentRun,
  AgentRunRequest,
  RunEventEnvelope,
} from "@/entities/agent-run/model/types";

export async function listAgents() {
  return invoke<AgentDescriptor[]>("list_agents");
}

export async function startAgentRun(request: AgentRunRequest) {
  return invoke<AgentRun>("start_agent_run", { request });
}

export async function cancelAgentRun(runId: string) {
  return invoke<void>("cancel_agent_run", { runId });
}

export function listenRunEvents(callback: (event: RunEventEnvelope) => void) {
  return listen<RunEventEnvelope>("agent-run-event", (event) => callback(event.payload));
}
