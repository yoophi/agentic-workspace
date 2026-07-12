import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { TocEntry } from "@yoophi/markdown-annotation-core/types";

import { MarkdownToc } from "./MarkdownToc";

const entries: TocEntry[] = [
  { blockId: "block-0", level: 1, text: "Title", startLine: 1 },
  { blockId: "block-2", level: 2, text: "Section A", startLine: 5 },
  { blockId: "block-4", level: 3, text: "Sub A1", startLine: 9 },
  { blockId: "block-6", level: 2, text: "Section A", startLine: 13 },
];

describe("MarkdownToc", () => {
  it("renders entries in order as buttons inside a nav", () => {
    const html = renderToStaticMarkup(<MarkdownToc entries={entries} />);

    expect(html).toContain("<nav");
    expect(html).toContain("<button");

    const positions = entries.map((entry) => html.indexOf(`data-toc-block-id="${entry.blockId}"`));
    expect(positions.every((position) => position >= 0)).toBe(true);
    expect([...positions].sort((a, b) => a - b)).toEqual(positions);
  });

  it("exposes data-toc-block-id per entry so duplicate texts stay distinguishable", () => {
    const html = renderToStaticMarkup(<MarkdownToc entries={entries} />);

    expect(html).toContain('data-toc-block-id="block-2"');
    expect(html).toContain('data-toc-block-id="block-6"');
    expect(html.match(/>Section A</g)?.length).toBe(2);
  });

  it("indents entries proportionally to heading level", () => {
    const html = renderToStaticMarkup(<MarkdownToc entries={entries} />);

    expect(buttonTagFor(html, "block-0")).toContain("padding-left:0.5rem");
    expect(buttonTagFor(html, "block-2")).toContain("padding-left:1.25rem");
    expect(buttonTagFor(html, "block-4")).toContain("padding-left:2rem");
  });

  it("renders nothing when entries are empty", () => {
    expect(renderToStaticMarkup(<MarkdownToc entries={[]} />)).toBe("");
  });

  it("keeps absolute level indentation when the document starts at h3", () => {
    const h3Only: TocEntry[] = [{ blockId: "block-9", level: 3, text: "Lonely", startLine: 1 }];
    const html = renderToStaticMarkup(<MarkdownToc entries={h3Only} />);

    expect(buttonTagFor(html, "block-9")).toContain("padding-left:2rem");
  });

  it("renders task progress for h1 chapters that contain tasks", () => {
    const taskEntries: TocEntry[] = [
      {
        blockId: "chapter-with-tasks",
        level: 1,
        text: "Tasks",
        startLine: 1,
        taskSummary: { completed: 3, open: 2 },
      },
      { blockId: "taskless-chapter", level: 1, text: "Notes", startLine: 10 },
    ];
    const html = renderToStaticMarkup(<MarkdownToc entries={taskEntries} />);

    expect(html).toContain('data-toc-task-summary="chapter-with-tasks"');
    expect(html).toContain('aria-label="3 completed tasks, 2 open tasks"');
    expect(html).not.toContain('data-toc-task-summary="taskless-chapter"');
  });
});

function buttonTagFor(html: string, blockId: string): string {
  const markerIndex = html.indexOf(`data-toc-block-id="${blockId}"`);
  expect(markerIndex).toBeGreaterThanOrEqual(0);
  const start = html.lastIndexOf("<button", markerIndex);
  const end = html.indexOf(">", markerIndex);
  return html.slice(start, end);
}
