import type { ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { MarkdownBlock } from "@yoophi/markdown-annotation-core/types";

import { MarkdownViewer } from "./MarkdownViewer";
import type { MarkdownViewerComponents, ViewerButtonProps, ViewerTooltipProps } from "./types";

function Button({ children, ...props }: ViewerButtonProps) {
  return <button {...props}>{children}</button>;
}

function Tooltip({ children }: ViewerTooltipProps) {
  return children as ReactElement;
}

const components: MarkdownViewerComponents = {
  Button,
  Tooltip,
};

function codeBlock(overrides: Partial<MarkdownBlock>): MarkdownBlock {
  return {
    id: "block-1",
    type: "code",
    content: "const graph = true;",
    rawContent: "```ts\nconst graph = true;\n```",
    order: 0,
    startLine: 1,
    endLine: 3,
    language: "ts",
    ...overrides,
  };
}

describe("MarkdownViewer", () => {
  it("preserves ordinary code block rendering", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer blocks={[codeBlock({})]} components={components} />,
    );

    expect(html).toContain("<pre>");
    expect(html).toContain("const graph = true;");
    expect(html).not.toContain("data-mermaid-status");
  });

  it("renders mermaid blocks inside the existing block shell", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={[
          codeBlock({
            content: "flowchart TD\n  A --> B",
            rawContent: "```\nflowchart TD\n  A --> B\n```",
            language: undefined,
            mermaid: {
              detected: true,
              reason: "leading-declaration",
              declaration: "flowchart",
              source: "flowchart TD\n  A --> B",
            },
          }),
        ]}
        components={components}
      />,
    );

    expect(html).toContain('data-block-id="block-1"');
    expect(html).toContain('data-start-line="1"');
    expect(html).toContain('data-end-line="3"');
    expect(html).toContain('data-mermaid-status="loading"');
    expect(html).toContain('aria-label="Delete block"');
    expect(html).toContain('aria-label="Comment on block"');
  });

  it("keeps mermaid fallback source isolated to the affected block", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={[
          codeBlock({
            content: "",
            rawContent: "```mermaid\n```",
            language: "mermaid",
            mermaid: {
              detected: true,
              reason: "language-marker",
              declaration: "mermaid",
              source: "",
            },
          }),
          {
            id: "block-2",
            type: "paragraph",
            content: "Following text remains readable.",
            rawContent: "Following text remains readable.",
            order: 1,
            startLine: 4,
            endLine: 4,
          },
        ]}
        components={components}
      />,
    );

    expect(html).toContain("Mermaid diagram source is empty.");
    expect(html).toContain("Following text remains readable.");
  });
});
