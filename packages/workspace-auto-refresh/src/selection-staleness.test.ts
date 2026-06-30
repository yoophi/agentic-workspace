import { describe, expect, it } from "vitest";

import {
  createStaleSelection,
  findStaleCommitSelection,
  findStaleFileSelection,
  findStaleMarkdownDocument,
  isSelectionStillPresent,
} from "./selection-staleness";

describe("workspace selection staleness", () => {
  it("keeps file selection when the selected path still exists", () => {
    expect(
      findStaleFileSelection({
        selectedPath: "src/app.tsx",
        availablePaths: ["src/app.tsx", "src/main.tsx"],
      }),
    ).toBeNull();
  });

  it("marks missing file selections stale", () => {
    expect(
      findStaleFileSelection({
        selectedPath: "src/deleted.ts",
        availablePaths: ["src/app.tsx"],
        now: 100,
      }),
    ).toEqual({
      kind: "file",
      id: "src/deleted.ts",
      reason: "deleted",
      detectedAt: 100,
    });
  });

  it("marks missing commit selections stale", () => {
    expect(
      findStaleCommitSelection({
        selectedCommitHash: "abc123",
        availableCommitHashes: ["def456"],
        reason: "branch-changed",
        now: 200,
      }),
    ).toEqual({
      kind: "commit",
      id: "abc123",
      reason: "branch-changed",
      detectedAt: 200,
    });
  });

  it("keeps commit selection when the selected hash still exists", () => {
    expect(
      findStaleCommitSelection({
        selectedCommitHash: "abc123",
        availableCommitHashes: ["abc123", "def456"],
      }),
    ).toBeNull();
  });

  it("marks unreadable markdown document stale", () => {
    expect(
      findStaleMarkdownDocument({
        absolutePath: "/notes/today.md",
        readable: false,
        now: 300,
      }),
    ).toEqual({
      kind: "markdown-document",
      id: "/notes/today.md",
      reason: "unreadable",
      detectedAt: 300,
    });
  });

  it("keeps markdown document selection when the active file remains readable", () => {
    expect(
      findStaleMarkdownDocument({
        absolutePath: "/notes/today.md",
        readable: true,
      }),
    ).toBeNull();
  });

  it("checks selection presence with normalized ids", () => {
    expect(
      isSelectionStillPresent({
        selectedId: " src/app.tsx ",
        availableIds: ["src/app.tsx"],
      }),
    ).toBe(true);
  });

  it("creates explicit stale selection records", () => {
    expect(
      createStaleSelection({
        kind: "diff-file",
        id: "src/app.tsx",
        reason: "history-rewritten",
        now: 400,
      }),
    ).toEqual({
      kind: "diff-file",
      id: "src/app.tsx",
      reason: "history-rewritten",
      detectedAt: 400,
    });
  });
});
