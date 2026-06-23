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

function finishRun(state: RunEventState): RunEventState {
  return {
    ...state,
    isAwaitingPromptResponse: false,
    queuedPrompts: [],
    isRunning: false,
    activeRunId: null,
  };
}
