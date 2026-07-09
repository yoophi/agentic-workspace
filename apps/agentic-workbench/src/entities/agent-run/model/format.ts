import type {
  AgentThreadStatus,
  EventGroup,
  RunEvent,
  SessionInfoUpdateMetadata,
  TimelineItem,
  ToolFileChange,
} from "./types";

export type TimelineRunEvent = Exclude<RunEvent, { type: "usage" }>;

export function isSessionInfoUpdateEvent(event: TimelineRunEvent) {
  return event.type === "sessionInfo" || (event.type === "raw" && readSessionInfoUpdateMetadata(event) !== null);
}

export function readAgentThreadStatus(event: TimelineRunEvent): AgentThreadStatus | null {
  if (event.type === "sessionInfo") {
    return normalizeAgentThreadStatus(event.threadStatus);
  }
  return readSessionInfoUpdateMetadata(event)?.threadStatus ?? null;
}

export function readSessionInfoUpdateMetadata(
  event: TimelineRunEvent,
): SessionInfoUpdateMetadata | null {
  if (event.type !== "raw" || event.method !== "session/update") {
    return null;
  }

  const update = findSessionUpdateObject(event.payload);
  if (!update || readString(update.sessionUpdate) !== "session_info_update") {
    return null;
  }

  return {
    sessionUpdate: "session_info_update",
    threadStatus: readThreadStatus(update),
    title: readString(update.title),
    updatedAt: readString(update.updatedAt),
  };
}

export function toTimelineItem(runId: string, event: TimelineRunEvent): TimelineItem {
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

function normalizeRunEvent(event: TimelineRunEvent): TimelineRunEvent {
  if (event.type !== "tool") {
    return event;
  }

  const rawEvent = event as TimelineRunEvent & { tool_call_id?: string };
  const toolCallId = event.toolCallId ?? rawEvent.tool_call_id;
  if (!toolCallId) {
    return event;
  }

  return {
    ...event,
    toolCallId,
  };
}

function findSessionUpdateObject(payload: unknown): Record<string, unknown> | null {
  const root = asRecord(payload);
  if (!root) {
    return null;
  }

  const candidates = [
    root,
    asRecord(root.update),
    asRecord(asRecord(root.params)?.update),
    asRecord(asRecord(asRecord(root.message)?.params)?.update),
  ];

  return (
    candidates.find(
      (candidate): candidate is Record<string, unknown> =>
        Boolean(candidate && readString(candidate.sessionUpdate) === "session_info_update"),
    ) ?? null
  );
}

function readThreadStatus(update: Record<string, unknown>): AgentThreadStatus | null {
  const meta = asRecord(update._meta);
  const codex = asRecord(meta?.codex);
  const threadStatus = asRecord(codex?.threadStatus);
  if (!threadStatus) {
    return null;
  }

  const type = readString(threadStatus.type);
  const activeFlags = Array.isArray(threadStatus.activeFlags)
    ? threadStatus.activeFlags.filter((value): value is string => typeof value === "string")
    : undefined;
  if (type === "active" || type === "idle") {
    return {
      type,
      ...(activeFlags ? { activeFlags } : {}),
    };
  }
  return { type: "unknown" };
}

function normalizeAgentThreadStatus(status: AgentThreadStatus | null | undefined): AgentThreadStatus | null {
  if (!status) {
    return null;
  }
  if (status.type === "active" || status.type === "idle") {
    return status;
  }
  return { type: "unknown" };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" ? (value as Record<string, unknown>) : null;
}

function readString(value: unknown) {
  return typeof value === "string" ? value : null;
}

function buildItem(
  base: Pick<TimelineItem, "id" | "runId" | "createdAt" | "event">,
  event: TimelineRunEvent,
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
        body: [
          event.title,
          ...event.locations.map((path) => `path: ${path}`),
          ...(event.fileChanges ?? []).map(
            (change) => `${change.kind}: ${change.path}${change.truncated ? " (truncated)" : ""}`,
          ),
        ]
          .filter(Boolean)
          .join("\n"),
        tone: event.status === "failed" ? "danger" : event.status === "completed" ? "success" : "info",
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
    case "ralphLoop":
      return {
        ...base,
        group: "lifecycle",
        title: `Ralph loop ${event.iteration}/${event.maxIterations}`,
        body: formatRalphLoopLine(event),
        tone:
          event.status === "failed"
            ? "danger"
            : event.status === "completed" || event.status === "stopped"
              ? "success"
              : "info",
      };
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
    case "sessionInfo":
      return {
        ...base,
        group: "lifecycle",
        title: "session info",
        body: "",
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
      const nextEvent: Extract<TimelineRunEvent, { type: "tool" }> = {
        ...item.event,
        toolCallId: item.event.toolCallId || currentEvent?.toolCallId,
        title: shouldKeepCurrentToolTitle(item.event, currentEvent)
          ? currentEvent?.title ?? item.event.title
          : item.event.title || currentEvent?.title || item.event.title,
        locations:
          item.event.locations.length > 0
            ? uniqueStrings(item.event.locations)
            : currentEvent
              ? uniqueStrings(currentEvent.locations)
              : uniqueStrings(item.event.locations),
        fileChanges: mergeToolFileChanges(currentEvent, item.event),
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

function uniqueStrings(values: string[]) {
  return values.filter((value, index, list) => list.indexOf(value) === index);
}

function mergeToolFileChanges(
  current: Extract<TimelineRunEvent, { type: "tool" }> | undefined,
  incoming: Extract<TimelineRunEvent, { type: "tool" }>,
) {
  const incomingChanges = incoming.fileChanges ?? [];
  const currentChanges = current?.fileChanges ?? [];
  const merged = currentChanges.map((change) =>
    applyToolStatusToFileChange(change, incoming.status),
  );

  for (const incomingChange of incomingChanges) {
    const index = merged.findIndex(
      (change) => change.path === incomingChange.path && change.kind === incomingChange.kind,
    );
    if (index >= 0) {
      merged[index] = incomingChange;
    } else {
      merged.push(incomingChange);
    }
  }

  return merged.length > 0 ? merged : undefined;
}

function applyToolStatusToFileChange(change: ToolFileChange, toolStatus: string): ToolFileChange {
  if (change.status === "failed" || change.status === "unavailable") {
    return change;
  }

  const status = normalizeToolFileChangeStatus(toolStatus);
  return status ? { ...change, status } : change;
}

function normalizeToolFileChangeStatus(status: string): ToolFileChange["status"] | null {
  if (status === "completed") {
    return "completed";
  }
  if (status === "failed") {
    return "failed";
  }
  if (status === "pending" || status === "running" || status === "in_progress" || status === "inProgress") {
    return "inProgress";
  }
  return null;
}

function findMatchingToolIndex(items: TimelineItem[], event: Extract<TimelineRunEvent, { type: "tool" }>) {
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
  incoming: Extract<TimelineRunEvent, { type: "tool" }>,
  current?: Extract<TimelineRunEvent, { type: "tool" }>,
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

function formatRalphLoopLine(event: Extract<RunEvent, { type: "ralphLoop" }>) {
  const labels = {
    started: "started",
    completed: "completed",
    failed: "failed",
    stopped: "stopped at max iterations",
  } satisfies Record<typeof event.status, string>;

  return `iteration ${event.iteration}/${event.maxIterations}: ${labels[event.status]}`;
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
