import { describe, expect, it } from "vitest";

import {
  AGENT_WINDOW_TITLE_MAX_LENGTH,
  formatWorktreeWindowTitle,
  normalizeAgentWindowTitle,
  resolveWorktreeWindowTitle,
  worktreeNameFromPath,
} from "./worktree-window-title";

describe("worktree window title helpers", () => {
  it("formats project and worktree basename", () => {
    expect(
      formatWorktreeWindowTitle(
        "Agentic Workbench",
        "/Users/yoophi/project/worktrees/agentic-workbench/feature-login",
      ),
    ).toBe("Agentic Workbench / feature-login");
  });

  it("handles trailing slashes and windows separators", () => {
    expect(worktreeNameFromPath("C:\\repo\\worktrees\\branch-a\\")).toBe("branch-a");
  });

  it("falls back for blank values", () => {
    expect(formatWorktreeWindowTitle("", "   ")).toBe("Project / worktree");
  });

  it("normalizes agent-provided window title overrides", () => {
    expect(normalizeAgentWindowTitle("  Fix login retry state  ")).toBe(
      "Fix login retry state",
    );
  });

  it("rejects invalid agent-provided window title overrides", () => {
    expect(normalizeAgentWindowTitle("   ")).toBeNull();
    expect(normalizeAgentWindowTitle("bad\u0007title")).toBeNull();
    expect(normalizeAgentWindowTitle("a".repeat(AGENT_WINDOW_TITLE_MAX_LENGTH + 1))).toBeNull();
  });

  it("uses valid agent override over the default title", () => {
    expect(resolveWorktreeWindowTitle("Project / worktree", "Fix state")).toBe("Fix state");
    expect(resolveWorktreeWindowTitle("Project / worktree", null)).toBe("Project / worktree");
  });
});
