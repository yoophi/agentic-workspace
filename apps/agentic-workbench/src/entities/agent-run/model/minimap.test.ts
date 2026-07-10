import { describe, expect, it } from "vitest";

import type { TimelineItem } from "./types";
import {
  MINIMAP_SUMMARY_MAX_LENGTH,
  projectTimelineToMinimapEntries,
} from "./minimap";

function timelineItem(
  id: string,
  group: TimelineItem["group"],
  body: string,
  runId = "run-1",
): TimelineItem {
  const event =
    group === "user/message"
      ? ({ type: "userMessage", text: body } as const)
      : group === "assistant/message"
        ? ({ type: "agentMessage", text: body } as const)
        : ({ type: "lifecycle", status: "started", message: body } as const);
  return { id, runId, group, title: group, body, createdAt: 1, event };
}

describe("projectTimelineToMinimapEntries", () => {
  it("keeps only user and assistant messages in source order", () => {
    const entries = projectTimelineToMinimapEntries([
      timelineItem("u1", "user/message", "first"),
      timelineItem("tool", "tool_call/tool_result", "ignored"),
      timelineItem("a1", "assistant/message", "second"),
      timelineItem("life", "lifecycle", "ignored"),
    ]);

    expect(entries.map(({ id, role, sourceOrder }) => ({ id, role, sourceOrder }))).toEqual([
      { id: "u1", role: "user", sourceOrder: 0 },
      { id: "a1", role: "assistant", sourceOrder: 2 },
    ]);
  });

  it("normalizes whitespace and bounds long summaries", () => {
    const [entry] = projectTimelineToMinimapEntries([
      timelineItem("u1", "user/message", `  multi\n\tspace ${"content ".repeat(30)}  `),
    ]);

    expect(entry.summary).not.toMatch(/\s{2,}/);
    expect(entry.summary.length).toBeLessThanOrEqual(MINIMAP_SUMMARY_MAX_LENGTH);
    expect(entry.summary.endsWith("...")).toBe(true);
  });

  it("preserves run ownership and clamps content weight", () => {
    const entries = projectTimelineToMinimapEntries([
      timelineItem("empty", "user/message", "", "owned-run"),
      timelineItem("long", "assistant/message", "x".repeat(10_000), "owned-run"),
    ]);

    expect(entries[0]).toMatchObject({ runId: "owned-run", contentWeight: 1 });
    expect(entries[1].contentWeight).toBe(12);
  });

  it("projects 500 mixed entries under the interaction budget without rich rendering", () => {
    const items = Array.from({ length: 750 }, (_, index) =>
      timelineItem(
        `item-${index}`,
        index % 3 === 0
          ? "tool_call/tool_result"
          : index % 2 === 0
            ? "user/message"
            : "assistant/message",
        index % 5 === 0 ? "```mermaid\ngraph TD; A-->B\n```" : `message ${index}`,
      ),
    );
    const startedAt = performance.now();
    const entries = projectTimelineToMinimapEntries(items);
    const elapsed = performance.now() - startedAt;

    expect(entries).toHaveLength(500);
    expect(entries.some((entry) => entry.id === "item-0")).toBe(false);
    expect(entries.every((entry) => entry.summary.length <= MINIMAP_SUMMARY_MAX_LENGTH)).toBe(
      true,
    );
    expect(elapsed).toBeLessThan(100);
  });
});
