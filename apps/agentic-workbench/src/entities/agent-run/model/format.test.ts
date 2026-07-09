import { describe, expect, it } from "vitest";

import {
  appendOneTimelineItem,
  appendSessionLifecycleStatusMessage,
  createSessionIdleLifecycleStatusMessage,
  createSessionStartLifecycleStatusMessage,
  formatAvailableCommandsSummary,
  formatSessionFreshnessLabel,
  isSessionInfoUpdateEvent,
  normalizeSessionUpdatedAt,
  readAgentThreadStatus,
  readSessionInfoUpdateMetadata,
  toTimelineItem,
} from "@/entities/agent-run/model/format";
import type { TimelineItem } from "@/entities/agent-run/model/types";

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

  it("reads title and updatedAt metadata from typed and raw session info updates", () => {
    expect(
      readSessionInfoUpdateMetadata({
        type: "sessionInfo",
        title: "  Fix session metadata  ",
        updatedAt: "2026-07-09T03:20:00.000Z",
        threadStatus: { type: "active", activeFlags: ["thinking"] },
      }),
    ).toEqual({
      sessionUpdate: "session_info_update",
      threadStatus: { type: "active", activeFlags: ["thinking"] },
      title: "  Fix session metadata  ",
      updatedAt: "2026-07-09T03:20:00.000Z",
    });

    expect(
      readSessionInfoUpdateMetadata({
        type: "raw",
        method: "session/update",
        payload: {
          params: {
            update: {
              sessionUpdate: "session_info_update",
              title: "Fallback parsed title",
              updatedAt: "2026-07-09T03:30:00.000Z",
              _meta: { codex: { threadStatus: { type: "idle" } } },
            },
          },
        },
      }),
    ).toEqual({
      sessionUpdate: "session_info_update",
      threadStatus: { type: "idle" },
      title: "Fallback parsed title",
      updatedAt: "2026-07-09T03:30:00.000Z",
    });
  });

  it("normalizes and formats session freshness labels from valid updatedAt values", () => {
    expect(normalizeSessionUpdatedAt("2026-07-09T03:20:00.000Z")).toBe(
      "2026-07-09T03:20:00.000Z",
    );
    expect(formatSessionFreshnessLabel("2026-07-09T03:20:00.000Z")).toBe(
      "Updated 2026-07-09 03:20 UTC",
    );
  });

  it("omits session freshness labels for missing or malformed updatedAt values", () => {
    expect(normalizeSessionUpdatedAt(null)).toBeNull();
    expect(normalizeSessionUpdatedAt("not-a-date")).toBeNull();
    expect(formatSessionFreshnessLabel(undefined)).toBeNull();
    expect(formatSessionFreshnessLabel("not-a-date")).toBeNull();
  });

  it("formats available command summaries for empty, singular, and plural counts", () => {
    expect(formatAvailableCommandsSummary(null)).toBe("No commands available");
    expect(
      formatAvailableCommandsSummary({
        sessionUpdate: "available_commands_update",
        updatedAt: 1,
        commands: [],
      }),
    ).toBe("No commands available");
    expect(
      formatAvailableCommandsSummary({
        sessionUpdate: "available_commands_update",
        updatedAt: 1,
        commands: [
          {
            id: "cmd-1",
            name: "status",
            description: null,
            inputHint: null,
            source: "appCommand",
          },
        ],
      }),
    ).toBe("1 command available");
    expect(
      formatAvailableCommandsSummary({
        sessionUpdate: "available_commands_update",
        updatedAt: 1,
        commands: [
          {
            id: "cmd-1",
            name: "status",
            description: null,
            inputHint: null,
            source: "appCommand",
          },
          {
            id: "cmd-2",
            name: "$skill",
            description: null,
            inputHint: null,
            source: "extension",
          },
        ],
      }),
    ).toBe("2 commands available");
  });

  it("creates concise session lifecycle status messages with stable dedupe keys", () => {
    expect(createSessionStartLifecycleStatusMessage("run-1")).toEqual({
      status: "sessionCreated",
      label: "Session started",
      description: "Agent session started.",
      tone: "info",
      dedupeKey: "run-1:sessionCreated",
    });
    expect(
      createSessionIdleLifecycleStatusMessage({
        runId: "run-1",
        previousStatus: { type: "active" },
        nextStatus: { type: "idle" },
      }),
    ).toEqual({
      status: "sessionIdle",
      label: "Agent idle",
      description: "Ready for the next prompt.",
      tone: "info",
      dedupeKey: "run-1:sessionIdle",
    });
  });

  it("does not create idle lifecycle messages for repeated or malformed statuses", () => {
    expect(
      createSessionIdleLifecycleStatusMessage({
        runId: "run-1",
        previousStatus: { type: "idle" },
        nextStatus: { type: "idle" },
      }),
    ).toBeNull();
    expect(
      createSessionIdleLifecycleStatusMessage({
        runId: "run-1",
        previousStatus: { type: "active" },
        nextStatus: { type: "unknown" },
      }),
    ).toBeNull();
    expect(
      createSessionIdleLifecycleStatusMessage({
        runId: "run-1",
        previousStatus: { type: "unknown" },
        nextStatus: null,
      }),
    ).toBeNull();
  });

  it("appends session lifecycle status messages as deduped lifecycle lines", () => {
    const started = createSessionStartLifecycleStatusMessage("run-1");
    const idle = createSessionIdleLifecycleStatusMessage({
      runId: "run-1",
      previousStatus: { type: "active" },
      nextStatus: { type: "idle" },
    });

    const items = [
      started,
      started,
      idle,
      idle,
    ].reduce(
      (currentItems, message) =>
        appendSessionLifecycleStatusMessage(currentItems, "run-1", message),
      [] as TimelineItem[],
    );

    expect(items).toHaveLength(1);
    expect(items[0]).toMatchObject({
      group: "lifecycle",
      title: "Agent run",
      tone: "info",
    });
    expect(items[0].body).toBe(
      "sessionCreated: Agent session started.\nsessionIdle: Ready for the next prompt.",
    );
  });

  it("keeps non-command raw events as timeline content", () => {
    const item = toTimelineItem("run-1", {
      type: "raw",
      method: "session/update",
      payload: { sessionUpdate: "other_update" },
    });

    expect(item.group).toBe("raw");
    expect(item.title).toBe("session/update");
    expect(item.body).toContain("other_update");
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
