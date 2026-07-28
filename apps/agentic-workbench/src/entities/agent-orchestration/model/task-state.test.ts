import { describe, expect, it } from "vitest";

import {
  canTransitionTask,
  detachNode,
  promoteNode,
} from "./task-state";
import type { AgentNode } from "./types";

const childNode: AgentNode = {
  id: "child-1",
  kind: "child",
  parentNodeId: "main-agent-run",
  role: {
    id: "researcher",
    name: "Researcher",
    responsibility: "조사",
    expectedOutput: "근거가 있는 결과",
  },
  currentRunId: "run-1",
  assignedTaskId: "task-1",
  executionStatus: "active",
  presentationStatus: "background",
  promotionPolicy: "onAttention",
  runtimeProfile: null,
  lastActivityAt: null,
  createdBy: "coordinator",
  createdAt: "2026-07-27T00:00:00Z",
};

describe("orchestration task and presentation state", () => {
  it("keeps terminal task states terminal", () => {
    expect(canTransitionTask("running", "completed")).toBe(true);
    expect(canTransitionTask("completed", "running")).toBe(false);
    expect(canTransitionTask("cancelled", "ready")).toBe(false);
  });

  it("promotes and detaches without changing the run binding", () => {
    const panel = promoteNode(childNode);
    const detached = detachNode(panel);

    expect(panel.presentationStatus).toBe("panel");
    expect(detached.presentationStatus).toBe("detached");
    expect(detached.currentRunId).toBe("run-1");
    expect(detached.executionStatus).toBe("active");
  });
});
