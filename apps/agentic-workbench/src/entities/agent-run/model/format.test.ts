import { describe, expect, it } from "vitest";

import {
  appendOneTimelineItem,
  isSessionInfoUpdateEvent,
  readAgentThreadStatus,
  toTimelineItem,
} from "@/entities/agent-run/model/format";

describe("run event formatting", () => {
  it("detects session_info_update raw events in direct, update, params, and wrapped payload shapes", () => {
    const update = {
      sessionUpdate: "session_info_update",
      _meta: { codex: { threadStatus: { type: "idle" } } },
    };

    expect(isSessionInfoUpdateEvent({ type: "raw", method: "session/update", payload: update })).toBe(true);
    expect(isSessionInfoUpdateEvent({ type: "raw", method: "session/update", payload: { update } })).toBe(true);
    expect(isSessionInfoUpdateEvent({ type: "raw", method: "session/update", payload: { params: { update } } })).toBe(true);
    expect(
      isSessionInfoUpdateEvent({
        type: "raw",
        method: "session/update",
        payload: { message: { params: { update } } },
      }),
    ).toBe(true);
  });

  it("does not treat unrelated raw events as session_info_update", () => {
    expect(
      isSessionInfoUpdateEvent({
        type: "raw",
        method: "session/update",
        payload: { sessionUpdate: "available_commands_update" },
      }),
    ).toBe(false);
    expect(
      isSessionInfoUpdateEvent({
        type: "raw",
        method: "other/update",
        payload: { sessionUpdate: "session_info_update" },
      }),
    ).toBe(false);
  });

  it("reads active, idle, unknown, and missing agent thread status from session info updates", () => {
    const baseUpdate = {
      sessionUpdate: "session_info_update",
      _meta: { codex: { threadStatus: { type: "active", activeFlags: ["busy", 1] } } },
    };

    expect(
      readAgentThreadStatus({
        type: "raw",
        method: "session/update",
        payload: baseUpdate,
      }),
    ).toEqual({ type: "active", activeFlags: ["busy"] });
    expect(
      readAgentThreadStatus({
        type: "raw",
        method: "session/update",
        payload: {
          ...baseUpdate,
          _meta: { codex: { threadStatus: { type: "idle" } } },
        },
      }),
    ).toEqual({ type: "idle" });
    expect(
      readAgentThreadStatus({
        type: "raw",
        method: "session/update",
        payload: {
          ...baseUpdate,
          _meta: { codex: { threadStatus: { type: "blocked" } } },
        },
      }),
    ).toEqual({ type: "unknown" });
    expect(
      readAgentThreadStatus({
        type: "raw",
        method: "session/update",
        payload: { sessionUpdate: "session_info_update", title: "test" },
      }),
    ).toBeNull();
  });

  it("detects typed sessionInfo events and reads their thread status", () => {
    expect(
      isSessionInfoUpdateEvent({
        type: "sessionInfo",
        threadStatus: { type: "active" },
      }),
    ).toBe(true);
    expect(
      readAgentThreadStatus({
        type: "sessionInfo",
        threadStatus: { type: "idle" },
      }),
    ).toEqual({ type: "idle" });
  });

  it("shows Ralph loop iteration number and status", () => {
    const item = toTimelineItem("run-1", {
      type: "ralphLoop",
      iteration: 2,
      maxIterations: 5,
      status: "completed",
    });

    expect(item.group).toBe("lifecycle");
    expect(item.title).toBe("Ralph loop 2/5");
    expect(item.body).toBe("iteration 2/5: completed");
    expect(item.tone).toBe("success");
  });

  it("preserves tool file changes when a completion update has no file changes", () => {
    const started = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "in_progress",
      title: "Edit file",
      locations: ["src/app.ts"],
      fileChanges: [
        {
          path: "src/app.ts",
          oldPath: null,
          kind: "modified",
          status: "inProgress",
          diff: "@@ -1 +1 @@\n-old\n+new",
          content: null,
          binary: false,
          truncated: false,
          message: null,
        },
      ],
    });
    const completed = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "completed",
      title: "",
      locations: [],
    });

    const [merged] = appendOneTimelineItem([started], completed);

    expect(merged.event.type).toBe("tool");
    if (merged.event.type !== "tool") {
      return;
    }
    expect(merged.event.fileChanges).toHaveLength(1);
    expect(merged.event.fileChanges?.[0]).toMatchObject({
      path: "src/app.ts",
      status: "completed",
    });
  });

  it("replaces matching tool file changes during merge", () => {
    const initial = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "in_progress",
      title: "Write file",
      locations: ["src/app.ts"],
      fileChanges: [
        {
          path: "src/app.ts",
          oldPath: null,
          kind: "modified",
          status: "inProgress",
          diff: "@@ -1 +1 @@\n-old\n+new",
          content: null,
          binary: false,
          truncated: false,
          message: null,
        },
      ],
    });
    const update = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "completed",
      title: "Write file",
      locations: ["src/app.ts"],
      fileChanges: [
        {
          path: "src/app.ts",
          oldPath: null,
          kind: "modified",
          status: "completed",
          diff: "@@ -1 +1,2 @@\n old\n+new",
          content: null,
          binary: false,
          truncated: false,
          message: null,
        },
        {
          path: "src/other.ts",
          oldPath: null,
          kind: "added",
          status: "completed",
          diff: null,
          content: "export {};\n",
          binary: false,
          truncated: false,
          message: null,
        },
      ],
    });

    const [merged] = appendOneTimelineItem([initial], update);

    expect(merged.event.type).toBe("tool");
    if (merged.event.type !== "tool") {
      return;
    }
    expect(merged.event.fileChanges).toHaveLength(2);
    expect(merged.event.fileChanges?.[0].diff).toContain("+new");
    expect(merged.event.fileChanges?.[1]).toMatchObject({ path: "src/other.ts", kind: "added" });
  });

  it("does not overwrite unavailable file change status on later tool completion", () => {
    const initial = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "in_progress",
      title: "Write binary",
      locations: ["asset.bin"],
      fileChanges: [
        {
          path: "asset.bin",
          oldPath: null,
          kind: "modified",
          status: "unavailable",
          diff: null,
          content: null,
          binary: true,
          truncated: false,
          message: "Binary content cannot be displayed.",
        },
      ],
    });
    const completed = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "completed",
      title: "",
      locations: [],
    });

    const [merged] = appendOneTimelineItem([initial], completed);

    expect(merged.event.type).toBe("tool");
    if (merged.event.type !== "tool") {
      return;
    }
    expect(merged.event.fileChanges?.[0].status).toBe("unavailable");
  });

  it("deduplicates tool locations while merging updates", () => {
    const initial = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "in_progress",
      title: "Edit file",
      locations: ["src/app.ts", "src/app.ts"],
    });
    const update = toTimelineItem("run-1", {
      type: "tool",
      toolCallId: "tool-1",
      status: "completed",
      title: "Edit file",
      locations: ["src/app.ts", "src/app.ts"],
    });

    const [merged] = appendOneTimelineItem([initial], update);

    expect(merged.event.type).toBe("tool");
    if (merged.event.type !== "tool") {
      return;
    }
    expect(merged.event.locations).toEqual(["src/app.ts"]);
  });
});
