import { describe, expect, it } from "vitest";
import {
  buildSelectionQuickPromptContext,
  createQuickPromptActionState,
  formatQuickPromptForAgent,
} from "@yoophi/markdown-annotation-core";

describe("markdown annotator selection quick prompt integration", () => {
  it("creates an agent prompt from selected text context", () => {
    const context = buildSelectionQuickPromptContext({
      sourceLabel: "review.md",
      documentPath: "/workspace/review.md",
      documentRevisionLabel: "review.md:42:3",
      selectedText: "selected sentence",
      anchors: [
        {
          blockId: "p1",
          startLine: 2,
          endLine: 2,
          startOffset: 0,
          endOffset: 17,
          selectedText: "selected sentence",
        },
      ],
    });

    expect(context.scope.label).toBe("선택 영역");
    expect(formatQuickPromptForAgent({ promptText: "Explain", context })).toContain("selected sentence");
  });

  it("exposes selection quick annotate as a selection toolbar action", () => {
    const action = createQuickPromptActionState({
      scopeKind: "selection",
      surface: "selection-toolbar",
      enabled: true,
    });

    expect(action).toMatchObject({
      accessibleName: "선택 영역 quick prompt",
      enabled: true,
      surface: "selection-toolbar",
    });
  });
});
