import { describe, expect, it } from "vitest";

import type { WorktreeChange } from "@/entities/agent-run/model/types";
import {
  changeBadgeVariant,
  changeTypeLabel,
  languageFromPath,
  previewForChange,
} from "./worktree-change-view";

function change(overrides: Partial<WorktreeChange> = {}): WorktreeChange {
  return {
    path: "src/lib.ts",
    oldPath: null,
    changeType: "modified",
    binary: false,
    diff: null,
    content: null,
    truncated: false,
    ...overrides,
  };
}

describe("worktree change view", () => {
  it("maps change types to Korean labels and badge variants", () => {
    expect(changeTypeLabel("added")).toBe("생성");
    expect(changeTypeLabel("deleted")).toBe("삭제");
    expect(changeBadgeVariant("deleted")).toBe("destructive");
    expect(changeBadgeVariant("modified")).toBe("secondary");
  });

  it("guesses language from extension and falls back to text", () => {
    expect(languageFromPath("src/lib.ts")).toBe("typescript");
    expect(languageFromPath("Cargo.toml")).toBe("toml");
    expect(languageFromPath("README")).toBe("text");
    expect(languageFromPath("data.unknownext")).toBe("text");
  });

  it("prefers diff, then content, then empty for preview", () => {
    expect(previewForChange(change({ diff: "@@ -1 +1 @@" }))).toEqual({
      kind: "diff",
      text: "@@ -1 +1 @@",
      language: "diff",
    });

    expect(
      previewForChange(
        change({ changeType: "untracked", content: "new file", path: "a.rs" }),
      ),
    ).toEqual({ kind: "content", text: "new file", language: "rust" });

    expect(previewForChange(change())).toEqual({ kind: "empty" });
  });

  it("reports binary changes as not previewable", () => {
    expect(previewForChange(change({ binary: true }))).toEqual({ kind: "binary" });
  });
});
