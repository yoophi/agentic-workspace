import { describe, expect, it } from "vitest";

import {
  buildProjectWorktreeRoute,
  readWorktreePath,
} from "./session-route";

describe("session route helpers", () => {
  it("round-trips worktree paths with route-sensitive characters", () => {
    const worktreePath = "/tmp/작업 tree/a#b%c";
    const route = buildProjectWorktreeRoute("project-1", worktreePath);

    expect(route).toBe(
      "/projects/project-1/worktrees?worktreePath=%2Ftmp%2F%EC%9E%91%EC%97%85+tree%2Fa%23b%25c",
    );
    expect(readWorktreePath(new URLSearchParams(route.split("?")[1]))).toBe(
      worktreePath,
    );
  });
});
