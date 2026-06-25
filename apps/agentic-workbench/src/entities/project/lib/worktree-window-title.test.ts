import { describe, expect, it } from "vitest";

import {
  formatWorktreeWindowTitle,
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
});
