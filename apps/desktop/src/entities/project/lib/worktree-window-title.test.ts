import { describe, expect, it } from "vitest";

import {
  formatWorktreeWindowTitle,
  worktreeNameFromPath,
} from "./worktree-window-title";

describe("worktree window title helpers", () => {
  it("formats project and worktree basename", () => {
    expect(
      formatWorktreeWindowTitle(
        "ACP Minimal App",
        "/Users/yoophi/project/worktrees/acp-minimal-app/feature-login",
      ),
    ).toBe("ACP Minimal App / feature-login");
  });

  it("handles trailing slashes and windows separators", () => {
    expect(worktreeNameFromPath("C:\\repo\\worktrees\\branch-a\\")).toBe("branch-a");
  });

  it("falls back for blank values", () => {
    expect(formatWorktreeWindowTitle("", "   ")).toBe("Project / worktree");
  });
});
