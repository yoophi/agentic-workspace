import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import {
  AUTO_REFRESH_INTERVAL_MS,
  autoRefreshQueryOptions,
  findStaleFileSelection,
} from "@yoophi/workspace-auto-refresh";

const WORKTREE_WORKSPACE_PANEL_SOURCE = readFileSync(
  new URL("../ui/worktree-workspace-panel.tsx", import.meta.url),
  "utf8",
);

describe("workbench workspace auto refresh integration", () => {
  it("uses the shared fallback refresh policy for worktree files", () => {
    expect(AUTO_REFRESH_INTERVAL_MS).toBe(30_000);
    expect(autoRefreshQueryOptions.refetchInterval).toBe(30_000);
  });

  it("detects stale selected file paths from refreshed file entries", () => {
    expect(
      findStaleFileSelection({
        selectedPath: "docs/removed.md",
        availablePaths: ["README.md", "src/main.tsx"],
        now: 1,
      }),
    ).toEqual({
      kind: "file",
      id: "docs/removed.md",
      reason: "deleted",
      detectedAt: 1,
    });
  });

  it("invalidates Speckit file queries through the worktree watcher path", () => {
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain("worktreeFileQueryKeys.speckit(worktree.path)");
    expect(WORKTREE_WORKSPACE_PANEL_SOURCE).toContain(
      'activeTab === "speckit" ? "active" : "none"',
    );
  });
});
