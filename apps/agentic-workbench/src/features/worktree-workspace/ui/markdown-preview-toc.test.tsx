import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { TocEntry } from "@yoophi/markdown-annotation-core/types";

import { MarkdownPreviewToc } from "./markdown-preview-toc";

const entries: TocEntry[] = [
  { blockId: "block-0", level: 1, text: "Plan", startLine: 1 },
  { blockId: "block-2", level: 2, text: "Goal", startLine: 5 },
  { blockId: "block-4", level: 3, text: "Detail", startLine: 9 },
  { blockId: "block-6", level: 2, text: "Goal", startLine: 13 },
];

describe("MarkdownPreviewToc", () => {
  it("renders toc entries in order when opened", () => {
    const html = renderToStaticMarkup(<MarkdownPreviewToc defaultOpen entries={entries} />);

    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("<nav");

    const positions = entries.map((entry) => html.indexOf(`data-toc-block-id="${entry.blockId}"`));
    expect(positions.every((position) => position >= 0)).toBe(true);
    expect([...positions].sort((a, b) => a - b)).toEqual(positions);
  });

  it("renders only the collapsed toggle row by default", () => {
    const html = renderToStaticMarkup(<MarkdownPreviewToc entries={entries} />);

    expect(html).toContain('aria-expanded="false"');
    expect(html).not.toContain("<nav");
    expect(html).not.toContain("data-toc-block-id");
  });

  it("renders nothing at all when entries are empty", () => {
    expect(renderToStaticMarkup(<MarkdownPreviewToc entries={[]} />)).toBe("");
    expect(renderToStaticMarkup(<MarkdownPreviewToc defaultOpen entries={[]} />)).toBe("");
  });

  it("constrains the opened list with its own scroll area", () => {
    const html = renderToStaticMarkup(<MarkdownPreviewToc defaultOpen entries={entries} />);
    const listContainer = html.slice(html.indexOf("</button>"), html.indexOf("<nav"));

    expect(listContainer).toContain("max-h-");
    expect(listContainer).toContain("overflow-y-auto");
  });
});
