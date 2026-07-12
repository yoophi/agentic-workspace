import { describe, expect, it } from "vitest";

import { parseMarkdownToBlocks } from "./parse-markdown-to-blocks";

describe("parseMarkdownToBlocks", () => {
  it("skips frontmatter and preserves source line numbers", () => {
    const blocks = parseMarkdownToBlocks(`---
title: Example
---
# Heading

Paragraph line one
Paragraph line two`);

    expect(blocks).toMatchObject([
      {
        id: "block-0",
        order: 0,
        type: "heading",
        level: 1,
        content: "Heading",
        rawContent: "# Heading",
        startLine: 4,
        endLine: 4,
      },
      {
        id: "block-1",
        order: 1,
        type: "paragraph",
        content: "Paragraph line one\nParagraph line two",
        rawContent: "Paragraph line one\nParagraph line two",
        startLine: 6,
        endLine: 7,
      },
    ]);
  });

  it("parses fenced code blocks with raw markdown and language", () => {
    const blocks = parseMarkdownToBlocks(`Before

\`\`\`ts
const value = 1;
\`\`\`

After`);

    expect(blocks).toMatchObject([
      {
        type: "paragraph",
        content: "Before",
        startLine: 1,
        endLine: 1,
      },
      {
        type: "code",
        language: "ts",
        content: "const value = 1;",
        rawContent: "```ts\nconst value = 1;\n```",
        startLine: 3,
        endLine: 5,
      },
      {
        type: "paragraph",
        content: "After",
        startLine: 7,
        endLine: 7,
      },
    ]);
  });

  it("attaches mermaid metadata for mermaid language markers", () => {
    const blocks = parseMarkdownToBlocks(`\`\`\`mermaid
flowchart TD
  A --> B
\`\`\``);

    expect(blocks[0]).toMatchObject({
      type: "code",
      language: "mermaid",
      content: "flowchart TD\n  A --> B",
      mermaid: {
        detected: true,
        reason: "language-marker",
        declaration: "mermaid",
        source: "flowchart TD\n  A --> B",
      },
    });
  });

  it("attaches mermaid metadata for priority start tokens without language markers", () => {
    const blocks = parseMarkdownToBlocks(`\`\`\`
requirementDiagram
  requirement test_req {
    id: 1
  }
\`\`\``);

    expect(blocks[0]).toMatchObject({
      type: "code",
      language: undefined,
      mermaid: {
        detected: true,
        reason: "leading-declaration",
        declaration: "requirementDiagram",
      },
    });
  });

  it("preserves ordinary code blocks without mermaid metadata", () => {
    const blocks = parseMarkdownToBlocks(`\`\`\`ts
const graph = new Map();
\`\`\``);

    expect(blocks[0]).toMatchObject({
      type: "code",
      language: "ts",
      content: "const graph = new Map();",
    });
    expect(blocks[0]?.mermaid).toBeUndefined();
  });

  it("parses tables, nested checklist items, and blockquotes", () => {
    const blocks = parseMarkdownToBlocks(`| Name | Value |
| --- | --- |
| A | 1 |

- [x] Done
  - [ ] Child

> quoted
> text`);

    expect(blocks).toMatchObject([
      {
        type: "table",
        content: "| Name | Value |\n| --- | --- |\n| A | 1 |",
        startLine: 1,
        endLine: 3,
      },
      {
        type: "list-item",
        content: "Done",
        checked: true,
        level: 0,
        ordered: false,
        startLine: 5,
      },
      {
        type: "list-item",
        content: "Child",
        checked: false,
        level: 1,
        ordered: false,
        startLine: 6,
      },
      {
        type: "blockquote",
        content: "quoted\ntext",
        rawContent: "> quoted\n> text",
        startLine: 8,
        endLine: 9,
      },
    ]);
  });

  it("parses a 2,000-block document within the Preview performance budget", () => {
    const markdown = Array.from({ length: 1_000 }, (_, index) =>
      `## Section ${index + 1}\n\nParagraph ${index + 1}`,
    ).join("\n\n");
    const startedAt = performance.now();
    const blocks = parseMarkdownToBlocks(markdown);

    expect(blocks).toHaveLength(2_000);
    expect(performance.now() - startedAt).toBeLessThan(2_000);
  });
});
