import {
  appendOneTimelineItem,
  toTimelineItem,
} from "@/entities/agent-run/model";
import type { RunEventEnvelope, TimelineItem } from "@/entities/agent-run/model";

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
  return [
    "The previous prompt was interrupted because the user wants to steer the task.",
    "",
    "## Original prompt",
    originalPrompt.trim(),
    "",
    "## Steering instruction",
    steerPrompt.trim(),
    "",
    "Continue from the original prompt, but follow the steering instruction above.",
  ].join("\n");
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
