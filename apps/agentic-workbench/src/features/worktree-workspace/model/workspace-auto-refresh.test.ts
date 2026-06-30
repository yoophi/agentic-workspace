import { describe, expect, it } from "vitest";

import {
  AUTO_REFRESH_INTERVAL_MS,
  autoRefreshQueryOptions,
  findStaleFileSelection,
} from "@yoophi/workspace-auto-refresh";

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
});
