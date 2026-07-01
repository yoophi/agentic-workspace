import { describe, expect, it } from "vitest";

import {
  AUTO_REFRESH_INTERVAL_MS,
  findStaleMarkdownDocument,
} from "@yoophi/workspace-auto-refresh";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";
import { shouldSwapDocument } from "./model/document-reload";

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

  it("swaps a reloaded document when content changes back to an earlier value", () => {
    const current = {
      fileName: "notes.md",
      absolutePath: "/notes/notes.md",
      markdownText: "B",
    };
    const reloaded = {
      fileName: "notes.md",
      absolutePath: "/notes/notes.md",
      markdownText: "A",
    };

    expect(shouldSwapDocument(current, reloaded)).toBe(true);
  });

  it("keeps document state stable when polling reloads identical content", () => {
    const current = {
      fileName: "notes.md",
      absolutePath: "/notes/notes.md",
      markdownText: "A",
    };

    expect(shouldSwapDocument(current, { ...current })).toBe(false);
  });
});
