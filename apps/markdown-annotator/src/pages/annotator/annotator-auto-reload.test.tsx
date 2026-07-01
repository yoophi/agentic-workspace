import { describe, expect, it } from "vitest";

import {
  AUTO_REFRESH_INTERVAL_MS,
  findStaleMarkdownDocument,
} from "@yoophi/workspace-auto-refresh";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";

describe("markdown annotator auto reload integration", () => {
  it("uses the shared fallback refresh policy for active markdown documents", () => {
    expect(AUTO_REFRESH_INTERVAL_MS).toBe(30_000);
  });

  it("marks unreadable active markdown documents as stale", () => {
    expect(
      findStaleMarkdownDocument({
        absolutePath: "/notes/missing.md",
        readable: false,
        now: 2,
      }),
    ).toEqual({
      kind: "markdown-document",
      id: "/notes/missing.md",
      reason: "unreadable",
      detectedAt: 2,
    });
  });

  it("parses updated Mermaid source after markdown text changes", () => {
    const initialBlocks = parseMarkdownToBlocks(`\`\`\`mermaid
flowchart TD
  A --> B
\`\`\``);
    const reloadedBlocks = parseMarkdownToBlocks(`\`\`\`mermaid
sequenceDiagram
  participant A
  participant B
  A->>B: Reloaded
\`\`\``);

    expect(initialBlocks[0]?.mermaid).toMatchObject({
      declaration: "mermaid",
      source: "flowchart TD\n  A --> B",
    });
    expect(reloadedBlocks[0]?.mermaid).toMatchObject({
      declaration: "mermaid",
      source: "sequenceDiagram\n  participant A\n  participant B\n  A->>B: Reloaded",
    });
  });
});
