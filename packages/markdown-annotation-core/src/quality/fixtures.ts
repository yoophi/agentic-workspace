import type { MarkdownRenderingFixture } from "./types";

const listCases = [
  "- one\ncontinuation", "- one\n\n  second paragraph", "- one\n  - child", "3. three\n4. four",
  "- [ ] open", "- [x] done", "- one\n\n  > quote", "- one\n\n      code",
  "- one\n  1. child", "* a\n* b", "- a\n\n- b", "- a\n  - b\n    - c",
  "1. one\n   continuation", "- a\n  \n  b", "- a\n  \`- code\`", "- a\n  \\- escaped",
  "- a\n  | A | B |\n  | - | - |\n  | 1 | 2 |", "- a\n  ~~strike~~", "- a\n  [link](./next.md)", "- a\n  www.example.com",
];

const annotationCases = Array.from({ length: 10 }, (_, index) => `- Item ${index + 1} with **annotated** text`);
const recoveryCases = [
  "[broken", "*unclosed", "- [", "    > not quote", "<!-- unclosed", "`unterminated",
  "- a\r\n\tcontinued", "<script>alert(1)</script>", "[x](javascript:alert(1))", "- a\n    - too deep",
];

export const markdownRenderingFixtures: MarkdownRenderingFixture[] = [
  ...listCases.map((markdown, index) => ({
    id: `list-${String(index + 1).padStart(2, "0")}`,
    category: "commonmark-list" as const,
    markdown,
    expectedBlockTypes: ["list-item" as const],
    expectedText: (() => {
      const match = markdown.match(/\b(one|a|three|open|done)\b/);
      return match ? [match[1]] : [];
    })(),
  })),
  ...annotationCases.map((markdown, index) => ({
    id: `annotation-${String(index + 1).padStart(2, "0")}`,
    category: "annotation" as const,
    markdown,
    expectedBlockTypes: ["list-item" as const],
    expectedText: ["Item"],
  })),
  ...recoveryCases.map((markdown, index) => ({
    id: `recovery-${String(index + 1).padStart(2, "0")}`,
    category: index === 7 || index === 8 ? "safety" as const : "recovery" as const,
    markdown,
    expectedBlockTypes: [],
    expectedText: [],
  })),
];
