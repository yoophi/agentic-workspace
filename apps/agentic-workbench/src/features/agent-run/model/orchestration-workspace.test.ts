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

  // SC-004: the interval is "state change recorded -> visible in the task list". The
  // projection happens in the same synchronous step as the event, so the list cannot lag
  // behind a stored change by a polling interval.
  it("reflects a recorded state change in the projected list within the 1s budget", () => {
    const BUDGET_MS = 1000;
    const ready = hydrateOrchestrationWorkspace(
      createOrchestrationWorkspaceState(),
      session,
    );

    const changed = {
      ...session,
      revision: session.revision + 1,
      nodes: [
        ...session.nodes,
        {
          id: "child-late",
          kind: "child",
          parentNodeId: "main-agent-run",
          assignedTaskId: "task-late",
          currentRunId: "run-late",
          role: { name: "Reviewer" },
        },
      ],
      tasks: [{ id: "task-late", status: "running" }],
    } as unknown as OrchestrationSession;

    const recordedAt = performance.now();
    const next = hydrateOrchestrationWorkspace(ready, changed);
    const children = selectDirectChildren(next.session as OrchestrationSession);
    const elapsedMs = performance.now() - recordedAt;

    expect(next.session?.revision).toBe(session.revision + 1);
    expect(children.map((child) => child.id)).toContain("child-late");
    expect(elapsedMs).toBeLessThan(BUDGET_MS);
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

  // SC-016: at the representative load of 8 nodes with 4 active runs, the state work behind
  // a Composer keystroke and a Rail refresh must stay well inside the 200ms budget. The
  // budget covers state projection only; agent provider latency is out of scope per spec.
  it("projects the representative 8-node, 4-active-run load within the interaction budget", () => {
    const children = Array.from({ length: 7 }, (_, index) => ({
      ...(session.nodes[0] as OrchestrationSession["nodes"][number]),
      id: `child-${index}`,
      kind: "child" as const,
      parentNodeId: "main-agent-run",
      assignedTaskId: `task-${index}`,
      // Four runs are active; the remaining children are idle background nodes.
      currentRunId: index < 4 ? `run-${index}` : null,
      executionStatus: index < 4 ? ("active" as const) : ("idle" as const),
      role: { name: `Role ${index}` },
    }));
    const snapshot = {
      ...session,
      nodes: [...session.nodes, ...children],
      tasks: children.map((child, index) => ({
        id: child.assignedTaskId,
        status: index < 4 ? "running" : "completed",
        latestResultReportId: `report-${index}`,
      })),
      reports: children.map((_, index) => ({
        id: `report-${index}`,
        taskId: `task-${index}`,
        unresolved: [],
      })),
    } as unknown as OrchestrationSession;

    const BUDGET_MS = 200;
    // One pass per interaction; repeated so a single scheduling hiccup cannot pass the test.
    const INTERACTIONS = 50;
    const started = performance.now();
    for (let index = 0; index < INTERACTIONS; index += 1) {
      const state = hydrateOrchestrationWorkspace(
        createOrchestrationWorkspaceState(),
        snapshot,
      );
      selectMainNode(snapshot);
      selectDirectChildren(snapshot);
      selectChildResultSummaries(snapshot);
      expect(state.session?.nodes).toHaveLength(8);
    }
    const perInteractionMs = (performance.now() - started) / INTERACTIONS;

    expect(perInteractionMs).toBeLessThan(BUDGET_MS);
  });
});
