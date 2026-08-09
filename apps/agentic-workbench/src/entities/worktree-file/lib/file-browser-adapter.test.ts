import { describe, expect, it } from "vitest";

import type { WorktreeFileEntry } from "../model/types";
import { buildWorktreeFileTreeRows, mergeWorktreeFileEntries } from "./file-browser-adapter";

function entry(relativePath: string, isDir = false, size = 10): WorktreeFileEntry {
  const segments = relativePath.split("/");
  return {
    name: segments[segments.length - 1] ?? relativePath,
    path: `/repo/${relativePath}`,
    relativePath,
    isDir,
    size,
    modifiedMs: 1,
  };
}

describe("worktree file browser adapter", () => {
  it("keeps first-wins lazy merge and legacy visible order", () => {
    const entries = mergeWorktreeFileEntries([
      [entry("README.md"), entry("src", true)],
      [entry("src/app.ts"), entry("src/deep", true)],
      [entry("src/app.ts", false, 20)],
    ]);
    expect(buildWorktreeFileTreeRows(entries, new Set(["src"])).map((row) => row.relativePath)).toEqual([
      "README.md", "src", "src/app.ts", "src/deep",
    ]);
    expect(entries.find((item) => item.relativePath === "src/app.ts")?.size).toBe(10);
  });

  it("preserves unicode paths and full-path identity", () => {
    const rows = buildWorktreeFileTreeRows(
      [entry("docs", true), entry("docs/한글 파일.md")],
      new Set(["docs"]),
    );
    expect(rows.map((row) => row.relativePath)).toEqual(["docs", "docs/한글 파일.md"]);
  });
});
