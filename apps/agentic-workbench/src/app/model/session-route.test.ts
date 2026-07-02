import { describe, expect, it } from "vitest";

import type { GitWorktree } from "@/entities/project/model/git-worktree";

import {
  buildProjectWorktreeRoute,
  createPlaceholderWorktree,
  readWorktreePath,
  resolveSessionWorktree,
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

describe("resolveSessionWorktree", () => {
  const loadedWorktree: GitWorktree = {
    path: "/repo/worktrees/feature-a",
    head: "abc123",
    branch: "feature-a",
    status: "clean",
    pruneReason: null,
    canDelete: true,
  };

  it("returns missing-path when the URL has no worktree path", () => {
    const resolution = resolveSessionWorktree({
      worktreePath: "",
      worktrees: undefined,
    });

    expect(resolution.kind).toBe("missing-path");
  });

  it("returns an unknown-status placeholder while the worktree list is loading", () => {
    const resolution = resolveSessionWorktree({
      worktreePath: "/repo/worktrees/feature-a",
      worktrees: undefined,
    });

    expect(resolution).toEqual({
      kind: "placeholder",
      worktree: createPlaceholderWorktree("/repo/worktrees/feature-a"),
    });
    if (resolution.kind === "placeholder") {
      expect(resolution.worktree.status).toBe("unknown");
      expect(resolution.worktree.canDelete).toBe(false);
    }
  });

  it("resolves to the matching worktree once the list arrives", () => {
    const resolution = resolveSessionWorktree({
      worktreePath: "/repo/worktrees/feature-a",
      worktrees: [loadedWorktree],
    });

    expect(resolution).toEqual({ kind: "resolved", worktree: loadedWorktree });
  });

  it("marks the path invalid when the loaded list has no matching worktree", () => {
    const resolution = resolveSessionWorktree({
      worktreePath: "/repo/worktrees/deleted",
      worktrees: [loadedWorktree],
    });

    expect(resolution.kind).toBe("invalid");
  });
});
