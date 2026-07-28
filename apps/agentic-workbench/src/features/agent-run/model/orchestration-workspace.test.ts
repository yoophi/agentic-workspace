import { describe, expect, it } from "vitest";

import {
  applyOrchestrationEvent,
  createOrchestrationWorkspaceState,
  hydrateOrchestrationWorkspace,
  shouldBindMainRun,
  selectDirectChildren,
  selectMainNode,
  selectChildResultSummaries,
} from "./orchestration-workspace";
import type { OrchestrationSession } from "@/entities/agent-orchestration";

const session = {
  id: "workspace-1",
  revision: 3,
  mainNodeId: "main-agent-run",
  nodes: [{ id: "main-agent-run", kind: "main" }],
  tasks: [],
  reports: [],
  dispatches: [],
} as unknown as OrchestrationSession;

describe("orchestration workspace controller", () => {
  it("hydrates one workspace snapshot and detects revision gaps", () => {
    const ready = hydrateOrchestrationWorkspace(
      createOrchestrationWorkspaceState(),
      session,
    );
    expect(ready.phase).toBe("ready");
    expect(
      applyOrchestrationEvent(ready, {
        workspaceId: "workspace-1",
        revision: 5,
        reason: "workspaceChanged",
      }).needsRehydrate,
    ).toBe(true);
  });

  it("binds each Main run once", () => {
    expect(shouldBindMainRun(null, "run-1")).toBe(true);
    expect(shouldBindMainRun("run-1", "run-1")).toBe(false);
    expect(shouldBindMainRun("run-1", null)).toBe(false);
  });

  it("keeps task, node and run identities separate in a Main star", () => {
    const child = {
      ...(session.nodes[0] as OrchestrationSession["nodes"][number]),
      id: "child-1",
      kind: "child" as const,
      parentNodeId: "main-agent-run",
      currentRunId: "run-child-1",
      assignedTaskId: "task-child-1",
    };
    const snapshot = {
      ...session,
      nodes: [...session.nodes, child],
    };

    expect(selectMainNode(snapshot)?.id).toBe("main-agent-run");
    expect(selectDirectChildren(snapshot)).toHaveLength(1);
    expect(selectDirectChildren(snapshot)[0]).toMatchObject({
      id: "child-1",
      currentRunId: "run-child-1",
      assignedTaskId: "task-child-1",
    });
  });

  it("projects the maximum eight-node workspace without losing result provenance", () => {
    const children = Array.from({ length: 7 }, (_, index) => ({
      ...(session.nodes[0] as OrchestrationSession["nodes"][number]),
      id: `child-${index}`,
      kind: "child" as const,
      parentNodeId: "main-agent-run",
      assignedTaskId: `task-${index}`,
      currentRunId: `run-${index}`,
      role: { name: `Role ${index}` },
    }));
    const snapshot = {
      ...session,
      nodes: [...session.nodes, ...children],
      tasks: children.map((child, index) => ({
        id: child.assignedTaskId,
        latestResultReportId: `report-${index}`,
      })),
      reports: children.map((_, index) => ({
        id: `report-${index}`,
        taskId: `task-${index}`,
        unresolved: index === 0 ? ["conflict-a"] : [],
      })),
    } as unknown as OrchestrationSession;

    const summaries = selectChildResultSummaries(snapshot);
    expect(summaries).toHaveLength(7);
    expect(summaries[0]).toMatchObject({
      nodeId: "child-0",
      result: { id: "report-0" },
      unresolved: ["conflict-a"],
    });
  });
});
