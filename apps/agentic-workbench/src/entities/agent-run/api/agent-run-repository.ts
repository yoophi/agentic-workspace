import { invoke, listen } from "@/shared/api/transport";

import type {
  AgentDescriptor,
  AgentRun,
  AgentRunRequest,
  AgentRunSettings,
  AgentToolCommandCandidateQuery,
  AgentToolCommandCandidateResponse,
  PermissionMode,
  DeliveredRunEvent,
  ProviderSession,
} from "@/entities/agent-run/model/types";

export async function listAgents() {
  return invoke<AgentDescriptor[]>("list_agents");
}

export async function listProviderSessions(agentId: string, cwd?: string) {
  return invoke<ProviderSession[]>("list_provider_sessions", { agentId, cwd });
}

export async function listAgentToolCommandCandidates(
  input: AgentToolCommandCandidateQuery,
) {
  return invoke<AgentToolCommandCandidateResponse>("list_agent_tool_command_candidates", {
    input,
  });
}

export async function getAgentRunSettings(workingDirectory: string) {
  return invoke<AgentRunSettings | null>("get_agent_run_settings", {
    workingDirectory,
  });
}

export async function saveAgentRunSettings(settings: AgentRunSettings) {
  return invoke<AgentRunSettings>("save_agent_run_settings", { settings });
}

export async function startAgentRun(request: AgentRunRequest, panelId?: string) {
  return invoke<AgentRun>("start_agent_run", { request, panelId });
}

export async function sendPromptToRun(runId: string, prompt: string) {
  return invoke<void>("send_prompt_to_run", { runId, prompt });
}

export async function steerPromptToRun(runId: string, prompt: string) {
  return invoke<void>("steer_prompt_to_run", { runId, prompt });
}

export async function cancelCurrentPromptAndSendToRun(runId: string, prompt: string) {
  return invoke<void>("cancel_current_prompt_and_send_to_run", { runId, prompt });
}

export async function setRunPermissionMode(runId: string, permissionMode: PermissionMode) {
  return invoke<void>("set_run_permission_mode", { runId, permissionMode });
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

/** 창의 run 이벤트(호환 경로: 창 삽입, 네트워크 경로: 창이 아는 run의 `run:<id>` 구독). */
export const AGENT_RUN_EVENT = "agent-run-event";

export function listenRunEvents(callback: (event: DeliveredRunEvent) => void) {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  void listen<DeliveredRunEvent>(AGENT_RUN_EVENT, (envelope) => {
    if (!disposed) {
      callback(envelope);
    }
  }).then((dispose) => {
    if (disposed) {
      dispose();
    } else {
      unlisten = dispose;
    }
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}
