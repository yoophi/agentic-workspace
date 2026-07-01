import { describe, expect, it } from "vitest";
import { buildBlockQuickPromptContext } from "@yoophi/markdown-annotation-core";

describe("markdown annotator block quick prompt integration", () => {
  it("uses MarkdownBlock rawContent for block prompt context", () => {
    const context = buildBlockQuickPromptContext({
      sourceLabel: "review.md",
      documentRevisionLabel: "review.md:80:6",
      block: {
        id: "table-1",
        type: "table",
        content: "| A | B |\n| - | - |",
        rawContent: "| A | B |\n| - | - |",
        startLine: 4,
        endLine: 5,
      },
    });

    expect(context.scope.label).toBe("Markdown 블럭");
    expect(context.scope.startLine).toBe(4);
    expect(context.content).toContain("| A | B |");
  });
});
