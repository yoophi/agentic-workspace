import { describe, expect, it } from "vitest";

import {
  createPermissionDisplayModel,
  formatPermissionInput,
  summarizeOptionLabel,
} from "./permission-display";

describe("permission display", () => {
  it("formats JSON-like input and preserves full detail text", () => {
    const model = createPermissionDisplayModel({
      type: "permission",
      permissionId: "perm-1",
      title: "  ",
      input: {
        command: "gh issue create --body long markdown",
        cwd: "/Users/yoophi/project/agentic-workspace",
        payload: { nested: true },
      },
      options: [],
      requiresResponse: true,
    });

    expect(model.title).toBe("Tool request");
    expect(model.detail?.text).toContain('"command": "gh issue create --body long markdown"');
    expect(model.detail?.text).toContain('"cwd": "/Users/yoophi/project/agentic-workspace"');
    expect(model.detail?.isMultiline).toBe(true);
  });

  it("marks long single-line detail without losing the original text", () => {
    const longCommand = `gh issue create --body ${"x".repeat(1200)}`;
    const model = createPermissionDisplayModel({
      type: "permission",
      permissionId: "perm-1",
      title: "Run command",
      input: longCommand,
      options: [],
      requiresResponse: true,
    });

    expect(model.detail?.text).toBe(longCommand);
    expect(model.detail?.isLong).toBe(true);
    expect(model.detail?.isMultiline).toBe(false);
  });

  it("handles unserializable input without throwing", () => {
    const circular: Record<string, unknown> = {};
    circular.self = circular;

    expect(formatPermissionInput(circular)).toContain("Unable to serialize permission input");
  });

  it("summarizes long or command-like approval labels", () => {
    expect(
      summarizeOptionLabel(
        "gh issue create --body " + "markdown ".repeat(20),
        "allow_once",
      ),
    ).toBe("Allow");
    expect(summarizeOptionLabel("No", "reject_once", true)).toBe("No");
    expect(summarizeOptionLabel("Cancel this request", "reject_once", true)).toBe("Cancel this request");
  });

  it("creates option summaries with reject detection and fallbacks", () => {
    const model = createPermissionDisplayModel({
      type: "permission",
      permissionId: "perm-1",
      title: "Tool request",
      input: undefined,
      requiresResponse: true,
      options: [
        {
          optionId: "allow-long",
          kind: "allow_once",
          name: "gh issue create --body " + "long ".repeat(30),
        },
        { optionId: "reject", kind: "reject_once", name: "" },
      ],
    });

    expect(model.options[0]).toMatchObject({
      optionId: "allow-long",
      fullLabel: expect.stringContaining("gh issue create"),
      buttonLabel: "Allow",
      isDestructiveOrReject: false,
    });
    expect(model.options[1]).toMatchObject({
      optionId: "reject",
      fullLabel: "reject_once",
      buttonLabel: "Reject",
      isDestructiveOrReject: true,
    });
  });
});
