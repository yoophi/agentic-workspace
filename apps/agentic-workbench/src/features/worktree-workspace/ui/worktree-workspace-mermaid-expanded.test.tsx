import { describe, expect, it } from "vitest";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";

import { markdownViewerComponents } from "./markdown-viewer-components";

describe("worktree workspace Mermaid expanded view wiring", () => {
  it("uses newly selected markdown source for future expanded Mermaid views", () => {
    const firstFileBlocks = parseMarkdownToBlocks(`\`\`\`mermaid
flowchart TD
  A --> B
\`\`\``);
    const nextFileBlocks = parseMarkdownToBlocks(`\`\`\`mermaid
sequenceDiagram
  A->>B: Latest
\`\`\``);

    expect(firstFileBlocks[0]?.mermaid?.source).toBe("flowchart TD\n  A --> B");
    expect(nextFileBlocks[0]?.mermaid?.source).toBe("sequenceDiagram\n  A->>B: Latest");
    expect(markdownViewerComponents.MermaidExpandedDialog).toBeDefined();
  });
});
