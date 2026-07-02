import {
  appendOneTimelineItem,
  toTimelineItem,
} from "@/entities/agent-run/model";
import type { RunEventEnvelope, TimelineItem } from "@/entities/agent-run/model";
import type {
  AgentCommandOverrides,
  AgentDescriptor,
  AgentProfile,
} from "@/entities/agent-run/model/types";
import { resolveAgentProfileLaunch } from "@/features/agent-command-override/model/command-overrides";

export type QueuedPrompt = {
  id: string;
  text: string;
};

export type UsageContext = {
  used: number;
  size: number;
};

export type RunEventState = {
  items: TimelineItem[];
  usageContext: UsageContext | null;
  isAwaitingPromptResponse: boolean;
  isRunning: boolean;
  activeRunId: string | null;
  queuedPrompts: QueuedPrompt[];
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
    return { ...nextState, isAwaitingPromptResponse: true };
  }
  if (envelope.event.status === "promptCompleted") {
    return { ...nextState, isAwaitingPromptResponse: false };
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
  const nextQueue = [...queue];
  nextQueue.splice(Math.max(0, Math.min(index, nextQueue.length)), 0, queuedPrompt);
  return nextQueue;
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
    isRunning: false,
    activeRunId: null,
  };
}
