import { describe, expect, it } from "vitest";
import { buildDocumentQuickPromptContext, buildSelectionQuickPromptContext } from "./build-quick-prompt-context";
import { formatQuickPromptForAgent } from "./format-quick-prompt-for-agent";

const baseOptions = {
  sourceLabel: "note.md",
  documentPath: "/tmp/note.md",
  documentRevisionLabel: "rev-1",
};

describe("formatQuickPromptForAgent", () => {
  it("formats selected context with prompt and source metadata", () => {
    const context = buildSelectionQuickPromptContext({
      ...baseOptions,
      selectedText: "Selected markdown",
      anchors: [{ blockId: "p1", startLine: 3, endLine: 3, selectedText: "Selected markdown" }],
    });

    const prompt = formatQuickPromptForAgent({ promptText: "Explain this", context });

    expect(prompt).toContain("# Markdown Quick Prompt");
    expect(prompt).toContain("## 요청\nExplain this");
    expect(prompt).toContain("- 범위 유형: 선택 영역");
    expect(prompt).toContain("- 행: 3");
    expect(prompt).toContain("```markdown\nSelected markdown\n```");
  });

  it("formats document-wide reduced context notice", () => {
    const context = buildDocumentQuickPromptContext({
      ...baseOptions,
      markdownText: "0123456789\nabcdef",
      maxContentLength: 8,
    });

    const prompt = formatQuickPromptForAgent({ promptText: "Review all", context });

    expect(prompt).toContain("- 범위: 전체 문서 (2 lines)");
    expect(prompt).toContain("- 첨부 상태: reduced (8/17 chars)");
    expect(prompt).toContain("- 축약 이유:");
  });

  it("rejects empty prompt text", () => {
    const context = buildDocumentQuickPromptContext({
      ...baseOptions,
      markdownText: "# Title",
    });

    expect(() => formatQuickPromptForAgent({ promptText: " ", context })).toThrow("required");
  });
});
