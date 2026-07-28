import { describe, expect, it } from "vitest";

import type { CoordinatorNotification, TaskCommand } from "./types";
import {
  initialTaskCommunicationState,
  latestTaskCommand,
  taskCommunicationReducer,
} from "./task-communication";

const command: TaskCommand = {
  id: "command-1",
  requestId: "request-1",
  payloadFingerprint: "payload",
  taskId: "task-1",
  nodeId: "child-1",
  runId: "run-1",
  attempt: 1,
  kind: "inputResponse",
  message: "continue",
  inputReportId: "report-input",
  delivery: "queue",
  source: "user",
  status: "pending",
  failure: null,
  createdAt: "2026-07-27T00:00:00Z",
  updatedAt: "2026-07-27T00:00:00Z",
};

const notification: CoordinatorNotification = {
  id: "notification-1",
  reportId: "report-1",
  taskId: "task-1",
  reportType: "result",
  generationId: "generation-1",
  mainRunId: "main-run",
  status: "pending",
  attemptCount: 0,
  failure: null,
  collectedAt: null,
  createdAt: "2026-07-27T00:00:00Z",
  updatedAt: "2026-07-27T00:00:00Z",
};

describe("taskCommunicationReducer", () => {
  it("hydrates durable command and notification state", () => {
    const state = taskCommunicationReducer(initialTaskCommunicationState, {
      type: "hydrate",
      commands: [command],
      notifications: [notification],
    });
    expect(latestTaskCommand(state, "task-1", "inputResponse")).toEqual(command);
    expect(state.notifications["notification-1"]).toEqual(notification);
  });

  it("keeps the newest exact-once delivery projection", () => {
    const accepted = {
      ...command,
      status: "accepted" as const,
      updatedAt: "2026-07-27T00:00:02Z",
    };
    const state = taskCommunicationReducer(
      taskCommunicationReducer(initialTaskCommunicationState, {
        type: "commandUpdated",
        command: accepted,
      }),
      { type: "commandUpdated", command },
    );
    expect(state.commands["command-1"]?.status).toBe("accepted");
  });
});
