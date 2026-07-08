import { describe, expect, it } from "vitest";

import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";
import {
  buildFileTreeRows,
  isParentDirectoryLoaded,
  mergeWorktreeFileEntries,
} from "@/features/worktree-workspace/model/file-tree";

function file(relativePath: string, size = 10): WorktreeFileEntry {
  const segments = relativePath.split("/");
  return {
    name: segments[segments.length - 1] ?? relativePath,
    path: `/repo/${relativePath}`,
    relativePath,
    isDir: false,
    size,
    modifiedMs: 1,
  };
}

function dir(relativePath: string): WorktreeFileEntry {
  const segments = relativePath.split("/");
  return {
    name: segments[segments.length - 1] ?? relativePath,
    path: `/repo/${relativePath}`,
    relativePath,
    isDir: true,
    size: 0,
    modifiedMs: 1,
  };
}

describe("worktree workspace file tree", () => {
  it("merges lazy-loaded entries by root-relative path", () => {
    const entries = mergeWorktreeFileEntries([
      [file("README.md"), dir("src")],
      [file("src/app.ts"), dir("src/deep")],
      [file("src/deep/inner.ts"), file("src/app.ts", 20)],
    ]);

    expect(entries.map((entry) => entry.relativePath)).toEqual([
      "README.md",
      "src",
      "src/app.ts",
      "src/deep",
      "src/deep/inner.ts",
    ]);
    expect(entries.find((entry) => entry.relativePath === "src/app.ts")?.size).toBe(10);
  });

  it("shows nested rows only when every ancestor is expanded", () => {
    const entries = [
      file("README.md"),
      dir("src"),
      file("src/app.ts"),
      dir("src/deep"),
      file("src/deep/inner.ts"),
    ];

    expect(buildFileTreeRows(entries, new Set()).map((row) => row.relativePath)).toEqual([
      "README.md",
      "src",
    ]);

    expect(buildFileTreeRows(entries, new Set(["src"])).map((row) => row.relativePath)).toEqual([
      "README.md",
      "src",
      "src/app.ts",
      "src/deep",
    ]);

    expect(
      buildFileTreeRows(entries, new Set(["src", "src/deep"])).map((row) => [
        row.relativePath,
        row.depth,
        row.isExpanded,
      ]),
    ).toEqual([
      ["README.md", 0, false],
      ["src", 0, true],
      ["src/app.ts", 1, false],
      ["src/deep", 1, true],
      ["src/deep/inner.ts", 2, false],
    ]);
  });

  it("keeps duplicate basenames distinct by full relative path", () => {
    const entries = mergeWorktreeFileEntries([
      [dir("src"), dir("docs")],
      [file("src/app.ts"), file("docs/app.ts")],
    ]);

    const rows = buildFileTreeRows(entries, new Set(["src", "docs"]));

    expect(rows.filter((row) => row.name === "app.ts").map((row) => row.relativePath)).toEqual([
      "docs/app.ts",
      "src/app.ts",
    ]);
  });

  it("preserves Korean and space-containing relative paths", () => {
    const entries = mergeWorktreeFileEntries([
      [dir("docs")],
      [file("docs/한글 파일.md")],
    ]);

    const rows = buildFileTreeRows(entries, new Set(["docs"]));

    expect(rows.map((row) => row.relativePath)).toEqual(["docs", "docs/한글 파일.md"]);
  });

  it("treats a selection as stale only after its parent directory is loaded", () => {
    expect(isParentDirectoryLoaded("README.md", [])).toBe(true);
    expect(isParentDirectoryLoaded("src/app.ts", [])).toBe(false);
    expect(isParentDirectoryLoaded("src/app.ts", ["src"])).toBe(true);
    expect(isParentDirectoryLoaded("src/deep/inner.ts", ["src"])).toBe(false);
    expect(isParentDirectoryLoaded("src/deep/inner.ts", ["src", "src/deep"])).toBe(true);
  });
});
