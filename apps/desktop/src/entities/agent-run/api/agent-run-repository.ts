import { invoke } from "@tauri-apps/api/core";

import type {
  AgentDescriptor,
  AgentRun,
  AgentRunRequest,
  ProviderSession,
  RunEventEnvelope,
} from "@/entities/agent-run/model/types";

export async function listAgents() {
  return invoke<AgentDescriptor[]>("list_agents");
}

export async function listProviderSessions(agentId: string, cwd?: string) {
  return invoke<ProviderSession[]>("list_provider_sessions", { agentId, cwd });
}

export async function startAgentRun(request: AgentRunRequest) {
  return invoke<AgentRun>("start_agent_run", { request });
}

export async function sendPromptToRun(runId: string, prompt: string) {
  return invoke<void>("send_prompt_to_run", { runId, prompt });
}

export async function cancelAgentRun(runId: string) {
  return invoke<void>("cancel_agent_run", { runId });
}

export async function respondAgentPermission(
  runId: string,
  permissionId: string,
  optionId: string,
) {
  return invoke<void>("respond_agent_permission", { runId, permissionId, optionId });
}

export function listenRunEvents(callback: (event: RunEventEnvelope) => void) {
  let disposed = false;
  const handleEnvelope = (envelope: RunEventEnvelope) => {
    if (disposed) {
      return;
    }
    callback(envelope);
  };
  const handleFallback = (event: Event) => {
    handleEnvelope((event as CustomEvent<RunEventEnvelope>).detail);
  };
  window.addEventListener("agent-run-event-fallback", handleFallback);

  return () => {
    disposed = true;
    window.removeEventListener("agent-run-event-fallback", handleFallback);
  };
}
