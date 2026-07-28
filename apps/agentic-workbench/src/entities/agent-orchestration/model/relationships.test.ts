import { describe, expect, it } from "vitest";

import {
  assertValidOrchestrationRelationships,
  createChildNode,
  createMainNode,
} from "./relationships";

describe("orchestration relationships", () => {
  it("creates one immutable Main and direct children", () => {
    const main = createMainNode("2026-07-27T00:00:00Z");
    const child = createChildNode({
      id: "child-1",
      role: {
        id: "reviewer",
        name: "Reviewer",
        responsibility: "검토",
        expectedOutput: "위험 보고",
      },
      createdAt: "2026-07-27T00:00:00Z",
      createdBy: "coordinator",
    });

    expect(main.id).toBe("main-agent-run");
    expect(main.parentNodeId).toBeNull();
    expect(child.parentNodeId).toBe(main.id);
    expect(() =>
      assertValidOrchestrationRelationships([main, child], main.id),
    ).not.toThrow();
  });

  it("rejects a grandchild relationship", () => {
    const main = createMainNode("2026-07-27T00:00:00Z");
    const child = createChildNode({
      id: "child-1",
      role: {
        id: "reviewer",
        name: "Reviewer",
        responsibility: "검토",
        expectedOutput: "위험 보고",
      },
      createdAt: "2026-07-27T00:00:00Z",
      createdBy: "coordinator",
    });
    const grandchild = {
      ...child,
      id: "grandchild",
      parentNodeId: child.id,
    };

    expect(() =>
      assertValidOrchestrationRelationships([main, child, grandchild], main.id),
    ).toThrow(/direct child/i);
  });
});
