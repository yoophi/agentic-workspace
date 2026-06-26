import { describe, expect, it } from "vitest";

import { parseMarkdownToBlocks } from "./parseMarkdownToBlocks";

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
});
