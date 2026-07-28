import type { ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";
import { MarkdownViewer } from "./MarkdownViewer";
import type { MarkdownViewerProps } from "./MarkdownViewer";
import type {
  MarkdownViewerComponents,
  ViewerButtonProps,
  ViewerTooltipProps,
} from "./types";

function Button({ children, ...props }: ViewerButtonProps) {
  return <button {...props}>{children}</button>;
}

function Tooltip({ children }: ViewerTooltipProps) {
  return children as ReactElement;
}

const components: MarkdownViewerComponents = { Button, Tooltip };

function renderMarkdown(markdown: string, extraProps: Partial<MarkdownViewerProps> = {}) {
  const blocks = parseMarkdownToBlocks(markdown);
  const html = renderToStaticMarkup(
    <MarkdownViewer blocks={blocks} components={components} {...extraProps} />,
  );
  return { blocks, html };
}

function countMatches(html: string, pattern: RegExp) {
  return html.match(pattern)?.length ?? 0;
}

describe("MarkdownViewer top-level list order (US1)", () => {
  it("CT-1: keeps document order for lists separated by a paragraph", () => {
    const { html } = renderMarkdown("- Apple\n\nMiddle paragraph here\n\n- Banana");
    const apple = html.indexOf("Apple");
    const middle = html.indexOf("Middle paragraph");
    const banana = html.indexOf("Banana");

    expect(apple).toBeLessThan(middle);
    expect(middle).toBeLessThan(banana);
    // Two separate unordered lists, not one merged list.
    expect(countMatches(html, /<ul>/g)).toBe(2);
  });

  it("CT-2: renders each list under its own heading", () => {
    const { html } = renderMarkdown(
      "## Setup\n\n- install\n- build\n\n## Usage\n\n- run\n- deploy",
    );

    expect(html.indexOf("install")).toBeLessThan(html.indexOf("Usage"));
    expect(html.indexOf("Usage")).toBeLessThan(html.indexOf("run"));
    expect(html.indexOf("build")).toBeLessThan(html.indexOf("Usage"));
  });

  it("CT-9: keeps lists inside their H1 chapters with task summaries in place", () => {
    const { html } = renderMarkdown("# Chapter One\n\n- [x] alpha\n\n# Chapter Two\n\n- [ ] bravo");

    expect(html.indexOf("Chapter One")).toBeLessThan(html.indexOf("alpha"));
    expect(html.indexOf("alpha")).toBeLessThan(html.indexOf("Chapter Two"));
    expect(html.indexOf("Chapter Two")).toBeLessThan(html.indexOf("bravo"));
    // A task summary is rendered for the chapters.
    expect(html).toContain("data-task-summary");
  });

  it("CT-7: renders list items without a missing-key warning", () => {
    const errors: string[] = [];
    const spy = vi
      .spyOn(console, "error")
      .mockImplementation((...args: unknown[]) => {
        errors.push(args.map((value) => String(value)).join(" "));
      });

    renderMarkdown("- a\n- b\n\ntext\n\n- c");
    spy.mockRestore();

    expect(errors.join("\n")).not.toMatch(/unique .?key.?/i);
  });

  it("CT-8: keeps an annotation on the correct item of a separated second list", () => {
    const markdown = "- Apple\n\nMiddle paragraph here\n\n- Banana";
    const blocks = parseMarkdownToBlocks(markdown);
    const banana = blocks.find((block) => block.content === "Banana");
    expect(banana).toBeDefined();

    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={blocks}
        components={components}
        annotatedBlockIds={new Set([banana?.id ?? ""])}
      />,
    );

    // The annotated flag sits on the Banana list item, not on Apple.
    expect(html).toContain(`data-annotated="true" data-block-id="${banana?.id}"`);
    expect(html).not.toContain(`data-annotated="true" data-block-id="${blocks[0]?.id}"`);
  });
});

describe("MarkdownViewer list type and nesting (US2)", () => {
  it("CT-3: splits an unordered run into ul and ol[start] on a type change", () => {
    const { html } = renderMarkdown("- alpha\n\n3. charlie\n4. delta");

    expect(html).toContain("<ul>");
    expect(html).toContain('<ol start="3">');
    expect(html.indexOf("alpha")).toBeLessThan(html.indexOf("charlie"));
  });

  it("CT-4: preserves deep nesting and the following top-level sibling order", () => {
    const { html } = renderMarkdown("- A\n  - A1\n    - A1a\n- B");

    expect(html.indexOf(">A<")).toBeLessThan(html.indexOf("A1"));
    expect(html.indexOf("A1a")).toBeLessThan(html.indexOf(">B<"));
    // outer list + two nested lists.
    expect(countMatches(html, /<ul>/g)).toBe(3);
  });

  it("CT-6: keeps two top-level items in one list when a nested item sits between them", () => {
    const { html } = renderMarkdown("- A\n  - A1\n- B");

    expect(html.indexOf(">A<")).toBeLessThan(html.indexOf("A1"));
    expect(html.indexOf("A1")).toBeLessThan(html.indexOf(">B<"));
    // One top-level <ul> (A, B) with a single nested <ul> (A1) — not two separate lists.
    expect(countMatches(html, /<ul>/g)).toBe(2);
  });
});

describe("MarkdownViewer list markers (no duplicate bullets/numbers)", () => {
  it("MK-1: increments ordered markers instead of repeating the start", () => {
    const { html } = renderMarkdown("1. one\n2. two\n3. three");

    expect(html).toContain(">1.<");
    expect(html).toContain(">2.<");
    expect(html).toContain(">3.<");
    expect(html.indexOf(">1.<")).toBeLessThan(html.indexOf(">2.<"));
    expect(html.indexOf(">2.<")).toBeLessThan(html.indexOf(">3.<"));
  });

  it("MK-2: numbers ordered items from a custom start", () => {
    const { html } = renderMarkdown("3. three\n4. four");

    expect(html).toContain(">3.<");
    expect(html).toContain(">4.<");
    expect(html).not.toContain(">1.<");
  });

  it("MK-3: renders a single custom marker per unordered item", () => {
    const { html } = renderMarkdown("- alpha\n- beta");

    // Exactly one custom '-' marker per item in the markup; native bullets are
    // turned off in styles.css (verified visually / via computed style in-app).
    expect(countMatches(html, />-<\/span>/g)).toBe(2);
  });
});

describe("MarkdownViewer list rendering performance (CT-5)", () => {
  it("renders 2,000 list items across two runs within the preview budget", () => {
    const firstRun = Array.from({ length: 1_000 }, (_, index) => `- item ${index + 1}`).join("\n");
    const secondRun = Array.from({ length: 1_000 }, (_, index) => `- item ${index + 1_001}`).join("\n");
    const markdown = `${firstRun}\n\n## Divider\n\n${secondRun}`;
    const blocks = parseMarkdownToBlocks(markdown);

    const startedAt = performance.now();
    const html = renderToStaticMarkup(<MarkdownViewer blocks={blocks} components={components} />);

    expect(html).toContain("item 2000");
    // Two separate runs split by the heading.
    expect(countMatches(html, /<ul>/g)).toBe(2);
    expect(performance.now() - startedAt).toBeLessThan(2_000);
  });
});
