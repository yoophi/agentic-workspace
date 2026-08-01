import { describe, expect, it } from "vitest";

import { emptyFileBrowserState, reduceRootScanBatch } from "./use-file-browser";

describe("reduceRootScanBatch", () => {
  it("ignores stale scan ids and non-increasing sequences", () => {
    const state = { ...emptyFileBrowserState, scanId: "current", lastSequence: 2, scanning: true };
    const stale = { scanId: "old", sequence: 3, entries: [], warnings: [], visitedEntries: 3, matchedDocuments: 0, completed: false };
    expect(reduceRootScanBatch(state, stale)).toBe(state);
    expect(reduceRootScanBatch(state, { ...stale, scanId: "current", sequence: 2 })).toBe(state);
  });

  it("merges duplicate paths and partial warnings without duplication", () => {
    const state = { ...emptyFileBrowserState, scanId: "scan", scanning: true };
    const first = reduceRootScanBatch(state, {
      scanId: "scan", sequence: 0, entries: [{ path: "a/file.md", kind: "file" }],
      warnings: [{ relativePath: "locked", code: "permission_denied" }], visitedEntries: 10, matchedDocuments: 1, completed: false,
    });
    const second = reduceRootScanBatch(first, {
      scanId: "scan", sequence: 1, entries: [{ path: "a/file.md", kind: "file", size: 12 }],
      warnings: [{ relativePath: "locked", code: "permission_denied" }], visitedEntries: 20, matchedDocuments: 1, completed: true,
    });
    expect(second.entries).toEqual([{ path: "a/file.md", kind: "file", size: 12 }]);
    expect(second.warnings).toHaveLength(1);
    expect(second.scanning).toBe(false);
  });
});
