import { describe, expect, it } from "vitest";
import {
  buildBlockQuickPromptContext,
  buildDocumentQuickPromptContext,
  buildSelectionQuickPromptContext,
  createQuickPromptActionState,
  isQuickPromptSendable,
} from "./build-quick-prompt-context";

const baseOptions = {
  sourceLabel: "note.md",
  documentPath: "/tmp/note.md",
  documentRevisionLabel: "rev-1",
};

describe("quick prompt context builder", () => {
  it("builds selection context from single and multi-block anchors", () => {
    const context = buildSelectionQuickPromptContext({
      ...baseOptions,
      selectedText: "first\nsecond",
      anchors: [
        { blockId: "a", startLine: 2, endLine: 2, startOffset: 0, endOffset: 5, selectedText: "first" },
        { blockId: "b", startLine: 4, endLine: 4, startOffset: 1, endOffset: 7, selectedText: "second" },
      ],
    });

    expect(context.scope).toMatchObject({ kind: "selection", label: "선택 영역", startLine: 2, endLine: 4 });
    expect(context.content).toBe("first\nsecond");
    expect(context.scope.selectionOffsets).toHaveLength(2);
  });

  it("builds block context from raw Markdown content", () => {
    const context = buildBlockQuickPromptContext({
      ...baseOptions,
      block: {
        id: "code-1",
        type: "code",
        content: "const value = true;",
        rawContent: "```ts\nconst value = true;\n```",
        startLine: 8,
        endLine: 10,
      },
    });

    expect(context.scope).toMatchObject({ kind: "block", label: "Markdown 블럭", blockId: "code-1" });
    expect(context.content).toBe("```ts\nconst value = true;\n```");
  });

  it("builds document context and marks long content as reduced", () => {
    const context = buildDocumentQuickPromptContext({
      ...baseOptions,
      markdownText: "0123456789\nabcdef",
      maxContentLength: 10,
    });

    expect(context.scope).toMatchObject({ kind: "document", label: "전체 문서", startLine: 1, endLine: 2 });
    expect(context.lengthState).toBe("reduced");
    expect(context.includedLength).toBe(10);
    expect(context.originalLength).toBe(17);
    expect(context.reductionReason).toContain("truncated");
  });

  it("rejects empty context and exposes disabled action state", () => {
    expect(() =>
      buildDocumentQuickPromptContext({
        ...baseOptions,
        markdownText: "   ",
      }),
    ).toThrow("non-empty");

    expect(
      createQuickPromptActionState({
        scopeKind: "selection",
        enabled: false,
        disabledReason: "선택 영역이 없습니다.",
      }),
    ).toMatchObject({
      accessibleName: "선택 영역 quick prompt",
      disabledReason: "선택 영역이 없습니다.",
      enabled: false,
    });
  });

  it("requires prompt text, context, and target before sending", () => {
    const context = buildDocumentQuickPromptContext({
      ...baseOptions,
      markdownText: "# Title",
    });

    expect(isQuickPromptSendable({ promptText: "Review", context, targetAvailable: true })).toBe(true);
    expect(isQuickPromptSendable({ promptText: " ", context, targetAvailable: true })).toBe(false);
    expect(isQuickPromptSendable({ promptText: "Review", context, targetAvailable: false })).toBe(false);
  });
});
