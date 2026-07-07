import { describe, expect, it } from "vitest";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";

import { markdownViewerComponents } from "@/shared/ui/markdown-viewer-components";

describe("annotator Mermaid expanded view wiring", () => {
  it("uses the latest parsed Mermaid source with expanded-view components available", () => {
    const blocks = parseMarkdownToBlocks(`\`\`\`mermaid
flowchart TD
  A --> B
\`\`\``);

    expect(blocks[0]?.mermaid).toMatchObject({
      source: "flowchart TD\n  A --> B",
    });
    expect(markdownViewerComponents.MermaidExpandedDialog).toBeDefined();
  });
});
