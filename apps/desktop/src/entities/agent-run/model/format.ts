import type { EventGroup, RunEvent, TimelineItem } from "./types";

export function toTimelineItem(runId: string, event: RunEvent): TimelineItem {
  const normalizedEvent = normalizeRunEvent(event);
  const createdAt = Date.now();
  const base = {
    id:
      normalizedEvent.type === "tool" && normalizedEvent.toolCallId
        ? `${runId}-tool-${normalizedEvent.toolCallId}`
        : `${runId}-${createdAt}-${Math.random().toString(16).slice(2)}`,
    runId,
    createdAt,
    event: normalizedEvent,
  };
  return buildItem(base, normalizedEvent);
}

function normalizeRunEvent(event: RunEvent): RunEvent {
  if (event.type !== "tool") {
    return event;
  }

  const rawEvent = event as RunEvent & { tool_call_id?: string };
  const toolCallId = event.toolCallId ?? rawEvent.tool_call_id;
  if (!toolCallId) {
    return event;
  }

  return {
    ...event,
    toolCallId,
  };
}

function buildItem(
  base: Pick<TimelineItem, "id" | "runId" | "createdAt" | "event">,
  event: RunEvent,
): TimelineItem {
  switch (event.type) {
    case "userMessage":
      return {
        ...base,
        group: "user/message",
        title: "user/message",
        body: event.text,
      };
    case "agentMessage":
      return {
        ...base,
        group: "assistant/message",
        title: "assistant/message",
        body: event.text,
      };
    case "thought":
      return { ...base, group: "thought", title: "thought", body: event.text };
    case "plan":
      return {
        ...base,
        group: "lifecycle",
        title: "plan",
        body: event.entries.map((entry) => `${entry.status}: ${entry.content}`).join("\n"),
      };
    case "tool":
      return {
        ...base,
        group: "tool_call/tool_result",
        title: event.title || event.toolCallId || "tool",
        body: [event.title, ...event.locations.map((path) => `path: ${path}`)]
          .filter(Boolean)
          .join("\n"),
        tone: event.status === "failed" ? "danger" : event.status === "completed" ? "success" : "info",
      };
    case "usage":
      return {
        ...base,
        group: "usage",
        title: "usage",
        body: `context ${event.used}/${event.size}`,
      };
    case "permission":
      return {
        ...base,
        group: "permission",
        title: "permission",
        body: [
          event.title,
          event.input ? JSON.stringify(event.input, null, 2) : "",
          event.requiresResponse ? "waiting for approval" : "",
          `selected: ${event.selected ?? "none"}`,
        ]
          .filter(Boolean)
          .join("\n"),
        tone: "warning",
      };
    case "fileSystem":
      return {
        ...base,
        group: "tool_call/tool_result",
        title: `fs.${event.operation}`,
        body: event.path,
      };
    case "terminal":
      return {
        ...base,
        group: "terminal",
        title: `terminal ${event.operation}`,
        body: [event.terminalId, event.message].filter(Boolean).join("\n"),
      };
    case "diagnostic":
      return { ...base, group: "lifecycle", title: "diagnostic", body: event.message };
    case "lifecycle":
      return {
        ...base,
        group: "lifecycle",
        title: "Agent run",
        body: formatLifecycleLine(event),
        tone:
          event.status === "completed" || event.status === "promptCompleted"
            ? "success"
            : event.status === "cancelled"
              ? "warning"
              : "info",
      };
    case "raw":
      return {
        ...base,
        group: "raw",
        title: event.method,
        body: JSON.stringify(event.payload, null, 2),
      };
    case "error":
      return { ...base, group: "error", title: "error", body: event.message, tone: "danger" };
  }
}

export const eventGroups: Array<{ id: EventGroup | "all"; label: string }> = [
  { id: "all", label: "All" },
  { id: "user/message", label: "User" },
  { id: "assistant/message", label: "Message" },
  { id: "thought", label: "Thought" },
  { id: "tool_call/tool_result", label: "Tool" },
  { id: "permission", label: "Permission" },
  { id: "terminal", label: "Terminal" },
  { id: "lifecycle", label: "Lifecycle" },
  { id: "error", label: "Error" },
  { id: "raw", label: "Raw" },
];

export function appendTimelineItem(items: TimelineItem[], item: TimelineItem[]) {
  return item.reduce(appendOneTimelineItem, items);
}

export function appendOneTimelineItem(items: TimelineItem[], item: TimelineItem) {
  const previous = items[items.length - 1];
  const canMerge =
    (previous?.event.type === "agentMessage" && item.event.type === "agentMessage") ||
    (previous?.event.type === "thought" && item.event.type === "thought");

  if (item.event.type === "tool") {
    const matchingIndex = findMatchingToolIndex(items, item.event);
    if (matchingIndex >= 0) {
      const current = items[matchingIndex];
      const currentEvent = current.event.type === "tool" ? current.event : undefined;
      const nextEvent: Extract<RunEvent, { type: "tool" }> = {
        ...item.event,
        toolCallId: item.event.toolCallId || currentEvent?.toolCallId,
        title: shouldKeepCurrentToolTitle(item.event, currentEvent)
          ? currentEvent?.title ?? item.event.title
          : item.event.title || currentEvent?.title || item.event.title,
        locations:
          item.event.locations.length > 0
            ? item.event.locations
            : currentEvent
              ? currentEvent.locations
              : item.event.locations,
      };
      const nextItem = buildItem(
        {
          id: current.id,
          runId: current.runId,
          createdAt: current.createdAt,
          event: nextEvent,
        },
        nextEvent,
      );
      return [
        ...items.slice(0, matchingIndex),
        nextItem,
        ...items.slice(matchingIndex + 1),
      ];
    }
  }

  if (item.event.type === "lifecycle") {
    const matchingIndex = findMatchingLifecycleIndex(items, item.runId);
    if (matchingIndex >= 0) {
      const current = items[matchingIndex];
      const nextItem = buildItem(
        {
          id: current.id,
          runId: current.runId,
          createdAt: current.createdAt,
          event: item.event,
        },
        item.event,
      );
      return [
        ...items.slice(0, matchingIndex),
        {
          ...nextItem,
          body: appendLine(current.body, formatLifecycleLine(item.event)),
        },
        ...items.slice(matchingIndex + 1),
      ];
    }
  }

  if (!previous || !canMerge) {
    return [...items, item];
  }

  const previousText = (previous.event as { text: string }).text;
  const incomingText = (item.event as { text: string }).text;
  return [
    ...items.slice(0, -1),
    {
      ...previous,
      body: `${previous.body}${item.body}`,
      event: { ...previous.event, text: `${previousText}${incomingText}` } as typeof previous.event,
    },
  ];
}

function findMatchingToolIndex(items: TimelineItem[], event: Extract<RunEvent, { type: "tool" }>) {
  if (!event.toolCallId) {
    return -1;
  }

  for (let index = items.length - 1; index >= 0; index -= 1) {
    const candidate = items[index];
    if (candidate.event.type !== "tool") {
      continue;
    }
    if (candidate.event.toolCallId === event.toolCallId) {
      return index;
    }
  }

  return -1;
}

function shouldKeepCurrentToolTitle(
  incoming: Extract<RunEvent, { type: "tool" }>,
  current?: Extract<RunEvent, { type: "tool" }>,
) {
  if (!current?.title) {
    return false;
  }
  if (!incoming.title) {
    return true;
  }
  return incoming.title.startsWith("id=") && !current.title.startsWith("id=");
}

function findMatchingLifecycleIndex(items: TimelineItem[], runId: string) {
  for (let index = items.length - 1; index >= 0; index -= 1) {
    const candidate = items[index];
    if (candidate.runId === runId && candidate.event.type === "lifecycle") {
      return index;
    }
  }
  return -1;
}

function formatLifecycleLine(event: Extract<RunEvent, { type: "lifecycle" }>) {
  return `${event.status}: ${event.message}`;
}

function appendLine(current: string, next: string) {
  if (!current) {
    return next;
  }
  if (current.split("\n").includes(next)) {
    return current;
  }
  return `${current}\n${next}`;
}
