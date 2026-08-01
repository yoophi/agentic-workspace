import { describe, expect, it } from "vitest";

import { createFileBrowserRows, FileBrowserPathError } from "./create-file-browser-rows";
import { compressedMarkdownFixture, unicodeFixture } from "./fixtures";

describe("createFileBrowserRows", () => {
  it("synthesizes ancestors and compresses directory-only chains", () => {
    const rows = createFileBrowserRows(compressedMarkdownFixture, {
      expandedPaths: new Set(["a", "b/b1", "d"]),
    });

    expect(rows.filter((row) => row.kind === "directory").map((row) => row.label)).toEqual([
      "a",
      "b/b1",
      "d",
    ]);
    expect(rows.find((row) => row.label === "b/b1")).toMatchObject({
      chainPaths: ["b", "b/b1"],
      path: "b/b1",
    });
  });

  it("deduplicates progressive batches and merges explicit directory state", () => {
    const rows = createFileBrowserRows(
      [
        { kind: "directory", path: "docs", childState: "loading" },
        { kind: "file", path: "docs/readme.md" },
        { kind: "file", path: "docs/readme.md" },
        { kind: "directory", path: "docs", childState: "loaded" },
      ],
      { expandedPaths: new Set(["docs"]) },
    );

    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({ childState: "loaded", path: "docs" });
  });

  it("uses directory-first case-insensitive natural sorting", () => {
    const rows = createFileBrowserRows(unicodeFixture, {
      expandedPaths: new Set(["문서"]),
    });

    expect(rows.map((row) => row.label)).toEqual([
      "문서",
      "키오스크2.md",
      "키오스크10.md",
      "README.md",
    ]);
  });

  it("searches relative paths and keeps matching ancestors", () => {
    const rows = createFileBrowserRows(compressedMarkdownFixture, {
      expandedPaths: new Set(),
      searchQuery: "b/b1",
    });

    expect(rows.map((row) => row.path)).toEqual(["b/b1", "b/b1/file2.md"]);
    expect(rows[1]?.matchRanges.length).toBeGreaterThan(0);
  });

  it.each(["/tmp/a.md", "../a.md", "a/../../b.md", "a\\b.md", "a\0b.md"])(
    "rejects unsafe or non-portable path %s",
    (path) => {
      expect(() =>
        createFileBrowserRows([{ kind: "file", path }], { expandedPaths: new Set() }),
      ).toThrow(FileBrowserPathError);
    },
  );

  it("builds and searches 10,000 entries within the interaction budget", () => {
    const entries = Array.from({ length: 10_000 }, (_, index) => ({ kind: "file" as const, path: `folder-${index % 100}/document-${index}.md` }));
    const started = performance.now();
    const rows = createFileBrowserRows(entries, { expandedPaths: new Set(), searchQuery: "document-9999" });
    expect(rows.some((row) => row.path.endsWith("document-9999.md"))).toBe(true);
    expect(performance.now() - started).toBeLessThan(100);
  });
});
