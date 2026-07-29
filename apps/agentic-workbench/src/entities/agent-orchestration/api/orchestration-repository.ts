import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  OrchestrationTask,
  PromptDispatch,
  PromptTargetMode,
  PresentationStatus,
  OrchestrationEvent,
  OrchestrationSession,
  TaskCommand,
  TaskReport,
} from "../model/types";

export const ORCHESTRATION_WORKSPACE_UPDATED_EVENT =
  "orchestration-workspace-updated";
export const ORCHESTRATION_COMMAND_UPDATED_EVENT =
  "orchestration-command-updated";
export const ORCHESTRATION_NOTIFICATION_UPDATED_EVENT =
  "orchestration-coordinator-notification-updated";

export type BootstrapOrchestrationInput = {
  worktreePath: string;
  resumeWorkspaceId?: string | null;
};

export type BindMainCoordinatorRunInput = {
  requestId: string;
  panelId: "main-agent-run";
  runId: string;
  state: "active" | "ended";
  expectedRevision: number;
};

export type DelegateOrchestrationGoalInput = {
  requestId: string;
  goal: string;
  expectedRevision: number;
};

export type DelegateOrchestrationGoalOutcome = {
  rootTaskId: string;
  generationId: string;
  dispatchId: string;
  status: "accepted";
};

export type OrchestrationApiError = {
  code: string;
  message: string;
  retryable: boolean;
};

export function parseOrchestrationError(error: unknown): OrchestrationApiError {
  const raw = error instanceof Error ? error.message : String(error);
  try {
    const parsed = JSON.parse(raw) as Partial<OrchestrationApiError>;
    if (
      typeof parsed.code === "string" &&
      typeof parsed.message === "string"
    ) {
      return {
        code: parsed.code,
        message: parsed.message,
        retryable: parsed.retryable === true,
      };
    }
  } catch {
    // Non-domain failures are normalized below.
  }
  return { code: "unknown", message: raw, retryable: false };
}

export function bootstrapOrchestrationWorkspace(
  input: BootstrapOrchestrationInput,
) {
  return invoke<OrchestrationSession>("bootstrap_orchestration_workspace", {
    input,
  });
}

export function getOrchestrationWorkspace() {
  return invoke<OrchestrationSession | null>("get_orchestration_workspace");
}

export function listRecoverableOrchestrationWorkspaces(worktreePath: string) {
  return invoke<OrchestrationSession[]>(
    "list_recoverable_orchestration_workspaces",
    { input: { worktreePath } },
  );
}

export function bindMainCoordinatorRun(
  input: BindMainCoordinatorRunInput,
) {
  return invoke<OrchestrationSession>("bind_main_coordinator_run", { input });
}

export function delegateOrchestrationGoal(
  input: DelegateOrchestrationGoalInput,
) {
  return invoke<DelegateOrchestrationGoalOutcome>(
    "delegate_orchestration_goal",
    { input },
  );
}

export function adoptManualOrchestrationChild(input: {
  panelId: string;
  title: string;
}) {
  return invoke<OrchestrationSession>("adopt_manual_orchestration_child", {
    input,
  });
}

export function listOrchestrationTasks(generationId: string) {
  return invoke<OrchestrationTask[]>("list_orchestration_tasks", {
    input: { generationId },
  });
}

export function collectOrchestrationReports() {
  return invoke<TaskReport[]>("collect_orchestration_reports");
}

export function setOrchestrationPresentation(input: {
  requestId: string;
  nodeId: string;
  presentationStatus: Extract<
    PresentationStatus,
    "panel" | "background" | "detached"
  >;
  expectedRevision: number;
}) {
  return invoke<OrchestrationSession>("set_orchestration_presentation", {
    input,
  });
}

export type OrchestrationTaskActionInput = {
  requestId: string;
  taskId: string;
  expectedRevision: number;
  message?: string | null;
  targetNodeId?: string | null;
};

function invokeTaskAction(command: string, input: OrchestrationTaskActionInput) {
  return invoke<OrchestrationSession>(command, { input });
}

export const respondOrchestrationInput = (input: OrchestrationTaskActionInput) =>
  invoke<TaskCommand>("respond_orchestration_input", { input });
export const cancelOrchestrationTask = (input: OrchestrationTaskActionInput) =>
  invokeTaskAction("cancel_orchestration_task", input);
export const retryOrchestrationTask = (input: OrchestrationTaskActionInput) =>
  invokeTaskAction("retry_orchestration_task", input);
export const reassignOrchestrationTask = (input: OrchestrationTaskActionInput) =>
  invokeTaskAction("reassign_orchestration_task", input);

export function sendOrchestrationChildCommand(input: {
  requestId: string;
  taskId: string;
  kind: TaskCommand["kind"];
  message?: string | null;
  inputReportId?: string | null;
  delivery?: "send" | "queue" | "draft";
  expectedTaskRevision?: number | null;
}) {
  return invoke<TaskCommand>("send_orchestration_child_command", {
    input: {
      ...input,
      delivery: input.delivery ?? "queue",
    },
  });
}

export function handoffOrchestrationCoordinator(input: {
  requestId: string;
  successorRunId: string;
  summary: string;
  confirmed: boolean;
  expectedRevision: number;
}) {
  return invoke<OrchestrationSession>("handoff_orchestration_coordinator", {
    input,
  });
}

export type RuntimeEventSnapshot = {
  runId: string;
  events: Array<{
    runId: string;
    sequence: number;
    event: unknown;
    terminal: boolean;
  }>;
  lastSequence: number;
  terminal: boolean;
  gapDetected: boolean;
};

export function replayOrchestrationRuntimeEvents(
  runId: string,
  afterSequence: number,
) {
  return invoke<RuntimeEventSnapshot>("replay_orchestration_runtime_events", {
    input: { runId, afterSequence },
  });
}

export function dispatchOrchestrationPrompt(input: {
  requestId: string;
  intent: "direct" | "delegate";
  targetMode: PromptTargetMode;
  message: string;
  delivery: "send" | "queue" | "draft";
  panelIds: string[];
  expectedRevision: number;
}) {
  return invoke<PromptDispatch>("dispatch_orchestration_prompt", { input });
}

export function recoverOrchestrationWorkspace() {
  return invoke<OrchestrationSession>("recover_orchestration_workspace");
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

export function listenOrchestrationWorkspaceUpdated(
  callback: (event: OrchestrationEvent) => void,
) {
  return listenWithFallback(ORCHESTRATION_WORKSPACE_UPDATED_EVENT, callback);
}

export function listenOrchestrationCommandUpdated(
  callback: (event: OrchestrationEvent) => void,
) {
  return listenWithFallback(ORCHESTRATION_COMMAND_UPDATED_EVENT, callback);
}

export function listenCoordinatorNotificationUpdated(
  callback: (event: OrchestrationEvent) => void,
) {
  return listenWithFallback(
    ORCHESTRATION_NOTIFICATION_UPDATED_EVENT,
    callback,
  );
}
