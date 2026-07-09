import {
  appendOneTimelineItem,
  readAgentThreadStatus,
  isSessionInfoUpdateEvent,
  normalizeSessionUpdatedAt,
  readAvailableCommandMetadata,
  readSessionInfoUpdateMetadata,
  toTimelineItem,
} from "@/entities/agent-run/model";
import type { RunEventEnvelope, TimelineItem } from "@/entities/agent-run/model";
import type {
  AgentThreadStatus,
  AgentCommandOverrides,
  AgentDescriptor,
  AgentProfile,
  AvailableCommandMetadata,
} from "@/entities/agent-run/model/types";
import { resolveAgentProfileLaunch } from "@/features/agent-command-override/model/command-overrides";

export type QueuedPrompt = {
  id: string;
  text: string;
  source?: QueuedPromptSource;
  dispatchAfterRunStart?: boolean;
};

export type QueuedPromptSource =
  | "first-run"
  | "manual-queue"
  | "saved-prompt"
  | "external-request"
  | "steer-fallback";

export type SteerInputStatus =
  | "pending"
  | "accepted"
  | "rejected"
  | "queuedFallback"
  | "cancelAndSendFallback"
  | "failed";

export type SteerInputSource = "manual-input" | "queued-prompt";

export type SteerInput = {
  id: string;
  targetRunId: string;
  text: string;
  status: SteerInputStatus;
  source: SteerInputSource;
  createdAtSequence: number;
  errorMessage?: string;
  originalQueueIndex?: number;
};

export type PromptDispatchPhase =
  | "idle"
  | "starting"
  | "awaitingPrompt"
  | "responding"
  | "steering"
  | "cancelling"
  | "cancelAndSending";

export type UsageContext = {
  used: number;
  size: number;
};

export type RunEventState = {
  items: TimelineItem[];
  usageContext: UsageContext | null;
  agentThreadStatus: AgentThreadStatus;
  sessionUpdatedAt: string | null;
  availableCommandMetadata: AvailableCommandMetadata | null;
  isAwaitingPromptResponse: boolean;
  isRunning: boolean;
  activeRunId: string | null;
  queuedPrompts: QueuedPrompt[];
  pendingSteers: SteerInput[];
  rejectedSteers: SteerInput[];
  promptDispatchPhase?: PromptDispatchPhase;
  transitionToken?: string | null;
  suppressQueueAutosend?: boolean;
};

export type PromptHistoryEntry = {
  text: string;
  sequence: number;
};

export type PromptHistoryCursor =
  | {
      status: "idle";
    }
  | {
      status: "viewing";
      index: number;
    };

export type PromptHistoryState = {
  entries: PromptHistoryEntry[];
  cursor: PromptHistoryCursor;
  preservedDraft: string | null;
};

export type PromptHistoryDirection = "previous" | "next";

export type PromptHistoryNavigationResult = {
  handled: boolean;
  nextInput: string;
  nextState: PromptHistoryState;
};

/**
 * 저장된 선택(profile id)이 enabled 프로필이 아니면 첫 enabled 프로필로
 * 폴백한다(specs/008 R6). enabled 프로필이 없으면 빈 문자열.
 */
export function resolveSelectedProfileId(
  enabledProfiles: AgentProfile[],
  candidateId: string,
): string {
  if (enabledProfiles.some((profile) => profile.id === candidateId)) {
    return candidateId;
  }
  return enabledProfiles[0]?.id ?? "";
}

/**
 * 세션 시작 request에 담을 실행 구성(specs/008). 선택된 프로필의 command/env를
 * 해석하되, catalog 기본 command는 request에 싣지 않는다(백엔드가 기본을 안다).
 */
export function resolveRequestAgentLaunch({
  profileId,
  agents,
  overrides,
}: {
  profileId: string;
  agents: AgentDescriptor[];
  overrides: AgentCommandOverrides | null | undefined;
}): {
  agentId: string;
  agentCommand?: string;
  agentEnv?: Record<string, string>;
} | null {
  const launch = resolveAgentProfileLaunch({ profileId, overrides, agents });
  if (!launch) {
    return null;
  }

  return {
    agentId: launch.agentId,
    ...(launch.source === "defaultCommand" ? {} : { agentCommand: launch.command }),
    ...(Object.keys(launch.env).length > 0 ? { agentEnv: launch.env } : {}),
  };
}

export function isOverrideCommandFailure(message: string) {
  const normalized = message.toLowerCase();
  return (
    normalized.includes("agent command") ||
    normalized.includes("spawning acp agent") ||
    normalized.includes("failed to spawn acp agent") ||
    normalized.includes("cannot be parsed")
  );
}

export const initialPromptHistoryState: PromptHistoryState = {
  entries: [],
  cursor: { status: "idle" },
  preservedDraft: null,
};

export function appendPromptHistory(
  state: PromptHistoryState,
  text: string,
): PromptHistoryState {
  const normalizedText = text.trim();
  if (!normalizedText) {
    return state;
  }

  const previousEntry = state.entries[state.entries.length - 1];
  return {
    entries: [
      ...state.entries,
      {
        text: normalizedText,
        sequence: previousEntry ? previousEntry.sequence + 1 : 1,
      },
    ],
    cursor: { status: "idle" },
    preservedDraft: null,
  };
}

export function resetPromptHistoryCursor(
  state: PromptHistoryState,
): PromptHistoryState {
  if (state.cursor.status === "idle" && state.preservedDraft === null) {
    return state;
  }

  return {
    ...state,
    cursor: { status: "idle" },
    preservedDraft: null,
  };
}

export function navigatePromptHistory({
  state,
  direction,
  currentInput,
  isEditableBoundary,
  hasModifierKey,
}: {
  state: PromptHistoryState;
  direction: PromptHistoryDirection;
  currentInput: string;
  isEditableBoundary: boolean;
  hasModifierKey: boolean;
}): PromptHistoryNavigationResult {
  if (hasModifierKey || !isEditableBoundary || state.entries.length === 0) {
    return { handled: false, nextInput: currentInput, nextState: state };
  }

  if (state.cursor.status === "idle") {
    if (direction === "next") {
      return { handled: false, nextInput: currentInput, nextState: state };
    }

    const nextIndex = state.entries.length - 1;
    return {
      handled: true,
      nextInput: state.entries[nextIndex].text,
      nextState: {
        ...state,
        cursor: { status: "viewing", index: nextIndex },
        preservedDraft: currentInput,
      },
    };
  }

  const currentEntry = state.entries[state.cursor.index];
  if (currentEntry && currentInput !== currentEntry.text) {
    return navigatePromptHistory({
      state: resetPromptHistoryCursor(state),
      direction,
      currentInput,
      isEditableBoundary,
      hasModifierKey,
    });
  }

  if (direction === "previous") {
    const nextIndex = Math.max(0, state.cursor.index - 1);
    return {
      handled: true,
      nextInput: state.entries[nextIndex].text,
      nextState: {
        ...state,
        cursor: { status: "viewing", index: nextIndex },
      },
    };
  }

  const nextIndex = state.cursor.index + 1;
  if (nextIndex >= state.entries.length) {
    return {
      handled: true,
      nextInput: state.preservedDraft ?? "",
      nextState: {
        ...state,
        cursor: { status: "idle" },
        preservedDraft: null,
      },
    };
  }

  return {
    handled: true,
    nextInput: state.entries[nextIndex].text,
    nextState: {
      ...state,
      cursor: { status: "viewing", index: nextIndex },
    },
  };
}

export function isPromptHistoryNavigationBoundary({
  value,
  selectionStart,
  selectionEnd,
  direction,
}: {
  value: string;
  selectionStart: number;
  selectionEnd: number;
  direction: PromptHistoryDirection;
}) {
  if (direction === "previous") {
    return !value.slice(0, selectionStart).includes("\n");
  }

  return !value.slice(selectionEnd).includes("\n");
}

export function applyRunEvent(
  state: RunEventState,
  envelope: RunEventEnvelope,
): RunEventState {
  if (envelope.runId !== state.activeRunId) {
    return state;
  }

  if (envelope.event.type === "usage") {
    return {
      ...state,
      usageContext: { used: envelope.event.used, size: envelope.event.size },
    };
  }

  if (isSessionInfoUpdateEvent(envelope.event)) {
    const agentThreadStatus = readAgentThreadStatus(envelope.event);
    const sessionUpdatedAt = normalizeSessionUpdatedAt(
      readSessionInfoUpdateMetadata(envelope.event)?.updatedAt,
    );
    return {
      ...state,
      ...(agentThreadStatus ? { agentThreadStatus } : {}),
      ...(sessionUpdatedAt ? { sessionUpdatedAt } : {}),
      ...(agentThreadStatus?.type === "idle" ? { isAwaitingPromptResponse: false } : {}),
    };
  }

  if (envelope.event.type === "raw") {
    const availableCommandMetadata = readAvailableCommandMetadata(
      envelope.event.payload,
    );
    if (availableCommandMetadata) {
      return {
        ...state,
        availableCommandMetadata,
      };
    }
  }

  const nextState = {
    ...state,
    items: appendOneTimelineItem(
      state.items,
      toTimelineItem(envelope.runId, envelope.event),
    ),
  };

  if (envelope.event.type === "error") {
    return finishRun(nextState);
  }

  if (envelope.event.type !== "lifecycle") {
    return nextState;
  }

  if (envelope.event.status === "promptSent") {
    const activated = activateRunStartQueuedPrompt({
      queue: nextState.queuedPrompts,
      items: nextState.items,
      runId: envelope.runId,
    });
    return {
      ...nextState,
      items: activated.items,
      queuedPrompts: activated.queue,
      isAwaitingPromptResponse: true,
    };
  }
  if (envelope.event.status === "promptCompleted") {
    return { ...nextState, isAwaitingPromptResponse: false };
  }
  if (envelope.event.status === "steerAccepted") {
    const [accepted] = nextState.pendingSteers;
    return {
      ...nextState,
      pendingSteers: accepted
        ? acceptPendingSteer(nextState.pendingSteers, accepted.id)
        : nextState.pendingSteers,
      isAwaitingPromptResponse: false,
    };
  }
  if (envelope.event.status === "steerRejected") {
    const [rejected] = nextState.pendingSteers;
    if (!rejected) {
      return { ...nextState, isAwaitingPromptResponse: false };
    }

    return {
      ...nextState,
      ...rejectPendingSteer({
        pendingSteers: nextState.pendingSteers,
        rejectedSteers: nextState.rejectedSteers,
        steerInputId: rejected.id,
        reason: envelope.event.message,
      }),
      isAwaitingPromptResponse: false,
    };
  }
  if (["completed", "cancelled"].includes(envelope.event.status)) {
    return finishRun(nextState);
  }

  return nextState;
}

export function addUserMessage(
  items: TimelineItem[],
  runId: string,
  text: string,
) {
  return appendOneTimelineItem(
    items,
    toTimelineItem(runId, { type: "userMessage", text }),
  );
}

export function hasUserMessage(
  items: TimelineItem[],
  runId: string,
  text: string,
) {
  return items.some(
    (item) =>
      item.runId === runId &&
      item.event.type === "userMessage" &&
      item.event.text === text,
  );
}

export function removeUserMessage(
  items: TimelineItem[],
  runId: string,
  text: string,
) {
  const index = items.findIndex(
    (item) =>
      item.runId === runId &&
      item.event.type === "userMessage" &&
      item.event.text === text,
  );
  if (index < 0) {
    return items;
  }
  return [...items.slice(0, index), ...items.slice(index + 1)];
}

export function createQueuedPrompt({
  id,
  text,
  source = "manual-queue",
  dispatchAfterRunStart = false,
}: {
  id: string;
  text: string;
  source?: QueuedPromptSource;
  dispatchAfterRunStart?: boolean;
}): QueuedPrompt | null {
  const normalizedText = text.trim();
  if (!normalizedText) {
    return null;
  }

  return {
    id,
    text: normalizedText,
    source,
    ...(dispatchAfterRunStart ? { dispatchAfterRunStart: true } : {}),
  };
}

export function createRunStartQueuedPrompt({
  id,
  text,
  source = "first-run",
}: {
  id: string;
  text: string;
  source?: QueuedPromptSource;
}) {
  return createQueuedPrompt({
    id,
    text,
    source,
    dispatchAfterRunStart: true,
  });
}

export function appendQueuedPrompt(
  queue: QueuedPrompt[],
  queuedPrompt: QueuedPrompt | null,
) {
  if (!queuedPrompt || queue.some((item) => item.id === queuedPrompt.id)) {
    return queue;
  }

  return [...queue, queuedPrompt];
}

export function shouldAutoDispatchQueuedPrompt(queue: QueuedPrompt[]) {
  return queue.length > 0 && !queue[0].dispatchAfterRunStart;
}

export function shouldAutoDispatchQueuedPromptWithSteers({
  queue,
  pendingSteers,
  suppressQueueAutosend = false,
}: {
  queue: QueuedPrompt[];
  pendingSteers: SteerInput[];
  suppressQueueAutosend?: boolean;
}) {
  return (
    shouldAutoDispatchQueuedPrompt(queue) &&
    pendingSteers.length === 0 &&
    !suppressQueueAutosend
  );
}

export function dequeueRunStartQueuedPrompt(queue: QueuedPrompt[]) {
  const [firstPrompt, ...remainingQueue] = queue;
  if (!firstPrompt?.dispatchAfterRunStart) {
    return { queue, queuedPrompt: null };
  }

  return { queue: remainingQueue, queuedPrompt: firstPrompt };
}

export function activateRunStartQueuedPrompt({
  queue,
  items,
  runId,
}: {
  queue: QueuedPrompt[];
  items: TimelineItem[];
  runId: string;
}) {
  const result = dequeueRunStartQueuedPrompt(queue);
  if (!result.queuedPrompt) {
    return { queue, items, queuedPrompt: null };
  }

  return {
    queue: result.queue,
    items: hasUserMessage(items, runId, result.queuedPrompt.text)
      ? items
      : addUserMessage(items, runId, result.queuedPrompt.text),
    queuedPrompt: result.queuedPrompt,
  };
}

export function moveQueuedPrompt(
  queue: QueuedPrompt[],
  fromIndex: number,
  toIndex: number,
) {
  if (
    fromIndex === toIndex ||
    fromIndex < 0 ||
    toIndex < 0 ||
    fromIndex >= queue.length ||
    toIndex >= queue.length
  ) {
    return queue;
  }

  const next = [...queue];
  const [movedPrompt] = next.splice(fromIndex, 1);
  next.splice(toIndex, 0, movedPrompt);
  return next;
}

export function updateQueuedPrompt(
  queue: QueuedPrompt[],
  promptId: string,
  text: string,
) {
  let updated = false;
  const nextQueue = queue.map((queuedPrompt) => {
    if (queuedPrompt.id !== promptId) {
      return queuedPrompt;
    }
    updated = true;
    return { ...queuedPrompt, text };
  });

  return { queue: nextQueue, updated };
}

export function removeQueuedPrompt(queue: QueuedPrompt[], promptId: string) {
  const index = queue.findIndex((queuedPrompt) => queuedPrompt.id === promptId);
  if (index < 0) {
    return { queue, queuedPrompt: null, index: -1 };
  }

  return {
    queue: [...queue.slice(0, index), ...queue.slice(index + 1)],
    queuedPrompt: queue[index],
    index,
  };
}

export function insertQueuedPrompt(
  queue: QueuedPrompt[],
  queuedPrompt: QueuedPrompt,
  index: number,
) {
  if (queue.some((item) => item.id === queuedPrompt.id)) {
    return queue;
  }

  const nextQueue = [...queue];
  nextQueue.splice(Math.max(0, Math.min(index, nextQueue.length)), 0, queuedPrompt);
  return nextQueue;
}

export function createSteerInput({
  id,
  targetRunId,
  text,
  source = "manual-input",
  createdAtSequence,
  originalQueueIndex,
}: {
  id: string;
  targetRunId: string;
  text: string;
  source?: SteerInputSource;
  createdAtSequence: number;
  originalQueueIndex?: number;
}): SteerInput | null {
  const normalizedText = text.trim();
  if (!targetRunId || !normalizedText) {
    return null;
  }

  return {
    id,
    targetRunId,
    text: normalizedText,
    source,
    status: "pending",
    createdAtSequence,
    ...(originalQueueIndex === undefined ? {} : { originalQueueIndex }),
  };
}

export function appendPendingSteer(
  pendingSteers: SteerInput[],
  steerInput: SteerInput | null,
) {
  if (!steerInput || pendingSteers.some((item) => item.id === steerInput.id)) {
    return pendingSteers;
  }
  return [...pendingSteers, steerInput];
}

export function acceptPendingSteer(pendingSteers: SteerInput[], steerInputId: string) {
  return pendingSteers.filter((item) => item.id !== steerInputId);
}

export function rejectPendingSteer({
  pendingSteers,
  rejectedSteers,
  steerInputId,
  reason,
}: {
  pendingSteers: SteerInput[];
  rejectedSteers: SteerInput[];
  steerInputId: string;
  reason: string;
}) {
  const target = pendingSteers.find((item) => item.id === steerInputId);
  if (!target) {
    return { pendingSteers, rejectedSteers };
  }

  return {
    pendingSteers: pendingSteers.filter((item) => item.id !== steerInputId),
    rejectedSteers: [
      ...rejectedSteers.filter((item) => item.id !== steerInputId),
      { ...target, status: "rejected" as const, errorMessage: reason },
    ],
  };
}

export function removeRejectedSteer(rejectedSteers: SteerInput[], steerInputId: string) {
  return rejectedSteers.filter((item) => item.id !== steerInputId);
}

export function moveRejectedSteerToQueue({
  queue,
  rejectedSteers,
  steerInputId,
}: {
  queue: QueuedPrompt[];
  rejectedSteers: SteerInput[];
  steerInputId: string;
}) {
  const target = rejectedSteers.find((item) => item.id === steerInputId);
  if (!target) {
    return { queue, rejectedSteers, queuedPrompt: null };
  }

  const queuedPrompt: QueuedPrompt = {
    id: `${target.id}:queued-fallback`,
    text: target.text,
    source: "steer-fallback",
  };

  return {
    queue: appendQueuedPrompt(queue, queuedPrompt),
    rejectedSteers: removeRejectedSteer(rejectedSteers, steerInputId),
    queuedPrompt,
  };
}

export function retryRejectedSteer({
  pendingSteers,
  rejectedSteers,
  steerInputId,
  nextId,
  createdAtSequence,
}: {
  pendingSteers: SteerInput[];
  rejectedSteers: SteerInput[];
  steerInputId: string;
  nextId: string;
  createdAtSequence: number;
}) {
  const target = rejectedSteers.find((item) => item.id === steerInputId);
  if (!target) {
    return { pendingSteers, rejectedSteers, steerInput: null };
  }

  const steerInput: SteerInput = {
    ...target,
    id: nextId,
    status: "pending",
    errorMessage: undefined,
    createdAtSequence,
  };

  return {
    pendingSteers: appendPendingSteer(pendingSteers, steerInput),
    rejectedSteers: removeRejectedSteer(rejectedSteers, steerInputId),
    steerInput,
  };
}

export function prepareQueuedPromptSteer({
  queue,
  queuedPromptId,
  targetRunId,
  steerInputId,
  createdAtSequence,
}: {
  queue: QueuedPrompt[];
  queuedPromptId: string;
  targetRunId: string;
  steerInputId: string;
  createdAtSequence: number;
}) {
  const result = removeQueuedPrompt(queue, queuedPromptId);
  if (!result.queuedPrompt) {
    return { queue, steerInput: null, removedPrompt: null, removedIndex: -1 };
  }

  return {
    queue: result.queue,
    steerInput: createSteerInput({
      id: steerInputId,
      targetRunId,
      text: result.queuedPrompt.text,
      source: "queued-prompt",
      createdAtSequence,
      originalQueueIndex: result.index,
    }),
    removedPrompt: result.queuedPrompt,
    removedIndex: result.index,
  };
}

export function buildSteerPrompt(originalPrompt: string, steerPrompt: string) {
  const normalizedOriginalPrompt = unwrapSteerPrompt(originalPrompt);

  return [
    "The previous prompt was interrupted because the user wants to steer the task.",
    "",
    "## Original prompt",
    normalizedOriginalPrompt,
    "",
    "## Steering instruction",
    steerPrompt.trim(),
    "",
    "Continue from the original prompt, but follow the steering instruction above.",
  ].join("\n");
}

function unwrapSteerPrompt(prompt: string) {
  const originalPromptMarker = "## Original prompt";
  const steeringInstructionMarker = "## Steering instruction";
  let current = prompt.trim();

  while (current.startsWith("The previous prompt was interrupted")) {
    const originalPromptStart = current.indexOf(originalPromptMarker);
    const steeringInstructionStart = current.indexOf(steeringInstructionMarker);
    if (originalPromptStart < 0 || steeringInstructionStart < 0) {
      break;
    }

    const next = current
      .slice(originalPromptStart + originalPromptMarker.length, steeringInstructionStart)
      .trim();
    if (!next || next === current) {
      break;
    }
    current = next;
  }

  return current;
}

export type QueuedSteerPlan = {
  steerPrompt: string;
  remainingQueue: QueuedPrompt[];
};

/**
 * queue에 담긴 특정 prompt를 steer 입력으로 쓰기 위한 계획을 만든다.
 * - 선택한 prompt만 queue에서 제거하고 나머지 순서는 유지한다.
 * - 선택한 prompt를 찾지 못하면 null을 반환한다(이미 전송/제거된 경우).
 */
export function prepareQueuedSteer(
  queue: QueuedPrompt[],
  promptId: string,
  originalPrompt: string,
): QueuedSteerPlan | null {
  const target = queue.find((queuedPrompt) => queuedPrompt.id === promptId);
  if (!target) {
    return null;
  }

  return {
    steerPrompt: buildSteerPrompt(originalPrompt, target.text),
    remainingQueue: queue.filter((queuedPrompt) => queuedPrompt.id !== promptId),
  };
}

function finishRun(state: RunEventState): RunEventState {
  return {
    ...state,
    isAwaitingPromptResponse: false,
    queuedPrompts: [],
    pendingSteers: [],
    rejectedSteers: [],
    isRunning: false,
    activeRunId: null,
    promptDispatchPhase: "idle",
    suppressQueueAutosend: false,
  };
}
