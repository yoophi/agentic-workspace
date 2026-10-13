import type { RunEvent } from "@yoophi/workbench-client";
import type { ChatSession, SessionStatus } from "./types";

export function newSession(
  id: string,
  agentId: string,
  label: string,
): ChatSession {
  return {
    id,
    agentId,
    label,
    status: "creating",
    messages: [],
    events: [],
    permissions: [],
    lastSequence: 0,
    historyIncomplete: false,
  };
}

/** Pure reducer; live events and replay snapshots cross the same sequence check. */
export function applyRunEvent(
  session: ChatSession,
  sequence: number,
  event: RunEvent,
): ChatSession {
  if (sequence <= session.lastSequence || session.status === "lost")
    return session;
  const next = {
    ...session,
    lastSequence: sequence,
    historyIncomplete:
      session.historyIncomplete || sequence > session.lastSequence + 1,
    events: [...session.events, { sequence, event }],
  };
  if (event.type === "agentMessage" || event.type === "thought") {
    const role = event.type === "thought" ? "thought" : "assistant";
    const previous = next.messages[next.messages.length - 1];
    const previousEvent = session.events[session.events.length - 1]?.event;
    next.messages =
      sequence === session.lastSequence + 1 &&
      previous?.role === role &&
      previousEvent?.type === event.type
        ? [
            ...next.messages.slice(0, -1),
            { ...previous, text: previous.text + event.text },
          ]
        : [
            ...next.messages,
            { id: `${session.id}:${sequence}`, role, text: event.text },
          ];
  } else if (event.type === "permission" && event.permissionId) {
    const previousPermission = next.permissions.find(
      (permission) => permission.id === event.permissionId,
    );
    next.permissions = next.permissions.filter(
      (permission) => permission.id !== event.permissionId,
    );
    if (event.requiresResponse && !event.selected) {
      next.permissions = [
        ...next.permissions,
        {
          id: event.permissionId,
          title: event.title,
          input: event.input,
          options: event.options,
          responding: previousPermission?.responding ?? false,
          outcome: previousPermission?.outcome,
        },
      ];
      if (next.status === "ready") next.status = "busy";
    }
  } else if (event.type === "lifecycle") {
    let status: SessionStatus = next.status;
    switch (event.status) {
      case "sessionCreated":
        if (
          status === "creating" ||
          (status === "uncertain" &&
            session.messages.length === 0 &&
            (session.creationOutcome?.kind === "unknown" ||
              (session.creationOutcome?.kind === "fault" &&
                session.creationOutcome.fault.outcome !== "notApplied")))
        ) {
          status = "ready";
          next.error = undefined;
        }
        break;
      case "promptSent":
        if (status === "ready" || status === "busy") status = "busy";
        break;
      case "promptCompleted":
        if (status === "busy") status = "ready";
        next.permissions = [];
        break;
      case "cancelled":
        status = "closed";
        next.permissions = [];
        break;
      case "completed":
        if (status !== "failed") status = "closed";
        next.permissions = [];
        break;
    }
    next.status = status;
  } else if (event.type === "error") {
    next.status = "failed";
    next.error = event.message;
  }
  return next;
}

export function readRunEvent(body: unknown): RunEvent {
  if (
    typeof body !== "object" ||
    body === null ||
    !("type" in body) ||
    typeof body.type !== "string"
  ) {
    throw new Error("Invalid run event received");
  }
  // The server serializes typed RunEvent values; generated contract keeps this Interface shared.
  return body as RunEvent;
}
