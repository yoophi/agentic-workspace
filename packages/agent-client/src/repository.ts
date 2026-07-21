// ACP agent-run 실행 제어 래퍼. Tauri command를 호출하고 run 이벤트를 구독한다.
// 어느 Tauri 앱이든(예: hushline) 이 모듈을 소비해 agent run을 구동할 수 있다.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type {
  AgentRun,
  AgentRunRequest,
  PermissionMode,
  RunEventEnvelope,
} from "./types";

/** run 이벤트가 전달되는 Tauri 이벤트 채널 이름. 앱의 sink가 이 채널로 emit한다. */
export const AGENT_RUN_EVENT = "agent-run-event";

export async function startAgentRun(request: AgentRunRequest) {
  return invoke<AgentRun>("start_agent_run", { request });
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

/**
 * run 이벤트를 구독한다. Tauri `agent-run-event` 채널을 우선 사용하고,
 * 백엔드 sink가 함께 dispatch하는 `agent-run-event-fallback` window 이벤트도 처리한다.
 * 반환된 함수를 호출하면 구독을 해제한다.
 */
export function listenRunEvents(callback: (event: RunEventEnvelope) => void) {
  let disposed = false;
  const handleEnvelope = (envelope: RunEventEnvelope) => {
    if (disposed) {
      return;
    }
    callback(envelope);
  };

  const unlistenPromise = listen<RunEventEnvelope>(AGENT_RUN_EVENT, (event) => {
    handleEnvelope(event.payload);
  });

  const handleFallback = (event: Event) => {
    handleEnvelope((event as CustomEvent<RunEventEnvelope>).detail);
  };
  window.addEventListener("agent-run-event-fallback", handleFallback);

  return () => {
    disposed = true;
    window.removeEventListener("agent-run-event-fallback", handleFallback);
    void unlistenPromise.then((unlisten) => unlisten());
  };
}
