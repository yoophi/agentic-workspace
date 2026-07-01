import { describe, expect, it } from "vitest";
import { buildDocumentQuickPromptContext } from "@yoophi/markdown-annotation-core";

describe("markdown annotator document quick prompt integration", () => {
  it("creates document context and marks reduced long content", () => {
    const context = buildDocumentQuickPromptContext({
      sourceLabel: "long.md",
      documentRevisionLabel: "long.md:999:40",
      markdownText: "0123456789\nabcdef",
      maxContentLength: 6,
    });

    expect(context.scope.label).toBe("전체 문서");
    expect(context.scope.endLine).toBe(2);
    expect(context.lengthState).toBe("reduced");
    expect(context.content).toBe("012345");
  });
});
