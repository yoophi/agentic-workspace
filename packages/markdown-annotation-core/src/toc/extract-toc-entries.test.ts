import { describe, expect, it } from "vitest";

import { parseMarkdownToBlocks } from "../parse/parse-markdown-to-blocks";
import { extractTocEntries } from "./extract-toc-entries";

const sampleMarkdown = [
  "# Title",
  "",
  "intro paragraph",
  "",
  "## Section A",
  "",
  "### Sub A1",
  "",
  "#### Deep heading",
  "",
  "##### Deeper heading",
  "",
  "###### Deepest heading",
  "",
  "## Section A",
  "",
  "closing paragraph",
].join("\n");

describe("extractTocEntries", () => {
  it("returns h1~h3 headings in document order with block fields preserved", () => {
    const blocks = parseMarkdownToBlocks(sampleMarkdown);
    const entries = extractTocEntries(blocks);

    const headingBlocks = blocks.filter(
      (block) => block.type === "heading" && (block.level ?? 0) <= 3,
    );

    expect(entries).toHaveLength(4);
    expect(entries.map((entry) => entry.text)).toEqual([
      "Title",
      "Section A",
      "Sub A1",
      "Section A",
    ]);
    expect(entries.map((entry) => entry.level)).toEqual([1, 2, 3, 2]);
    entries.forEach((entry, index) => {
      expect(entry.blockId).toBe(headingBlocks[index].id);
      expect(entry.startLine).toBe(headingBlocks[index].startLine);
    });
  });

  it("excludes h4~h6 headings", () => {
    const entries = extractTocEntries(parseMarkdownToBlocks(sampleMarkdown));

    expect(entries.some((entry) => entry.text.includes("Deep"))).toBe(false);
    expect(entries.every((entry) => entry.level >= 1 && entry.level <= 3)).toBe(true);
  });

  it("keeps duplicate heading texts as separate entries with distinct block ids", () => {
    const entries = extractTocEntries(parseMarkdownToBlocks(sampleMarkdown));
    const duplicates = entries.filter((entry) => entry.text === "Section A");

    expect(duplicates).toHaveLength(2);
    expect(duplicates[0].blockId).not.toBe(duplicates[1].blockId);
    expect(duplicates[0].startLine).toBeLessThan(duplicates[1].startLine);
  });

  it("strips inline formatting from entry text", () => {
    const blocks = parseMarkdownToBlocks("## **Bold** `code` [link](https://a.b)");
    const entries = extractTocEntries(blocks);

    expect(entries).toHaveLength(1);
    expect(entries[0].text).toBe("Bold code link");
  });

  it("preserves startLine for documents with frontmatter", () => {
    const markdown = ["---", "title: sample", "---", "", "# After frontmatter"].join("\n");
    const blocks = parseMarkdownToBlocks(markdown);
    const entries = extractTocEntries(blocks);
    const headingBlock = blocks.find((block) => block.type === "heading");

    expect(entries).toHaveLength(1);
    expect(headingBlock).toBeDefined();
    expect(entries[0].startLine).toBe(headingBlock?.startLine);
    expect(entries[0].startLine).toBeGreaterThan(3);
  });

  it("adds completed and open task counts to each h1 chapter entry", () => {
    const markdown = [
      "# First",
      "",
      "- [x] done",
      "- [ ] open",
      "## Nested section",
      "- [X] also done",
      "",
      "# Second",
      "",
      "- [ ] another open",
    ].join("\n");
    const entries = extractTocEntries(parseMarkdownToBlocks(markdown));

    expect(entries.find((entry) => entry.text === "First")?.taskSummary).toEqual({
      completed: 2,
      open: 1,
    });
    expect(entries.find((entry) => entry.text === "Nested section")?.taskSummary).toBeUndefined();
    expect(entries.find((entry) => entry.text === "Second")?.taskSummary).toEqual({
      completed: 0,
      open: 1,
    });
  });

  it("omits task summaries for taskless chapters and tasks before the first h1", () => {
    const markdown = [
      "- [x] preamble task",
      "",
      "## Section before h1",
      "- [ ] unscoped task",
      "",
      "# Taskless chapter",
    ].join("\n");
    const entries = extractTocEntries(parseMarkdownToBlocks(markdown));

    expect(entries.every((entry) => entry.taskSummary === undefined)).toBe(true);
  });

  it("returns an empty array for empty input", () => {
    expect(extractTocEntries([])).toEqual([]);
  });

  it("returns an empty array for documents without any heading", () => {
    const markdown = ["plain paragraph", "", "- list item", "", "> quote"].join("\n");

    expect(extractTocEntries(parseMarkdownToBlocks(markdown))).toEqual([]);
  });

  it("returns an empty array when only h4~h6 headings exist", () => {
    const markdown = ["#### h4 only", "", "##### h5 only", "", "###### h6 only"].join("\n");

    expect(extractTocEntries(parseMarkdownToBlocks(markdown))).toEqual([]);
  });

  it("does not mutate the input blocks", () => {
    const blocks = parseMarkdownToBlocks(sampleMarkdown);
    const snapshot = structuredClone(blocks);

    extractTocEntries(blocks);

    expect(blocks).toEqual(snapshot);
  });
});
