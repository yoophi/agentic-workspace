import type { ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { MarkdownBlock } from "@yoophi/markdown-annotation-core/types";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";

import { MarkdownViewer } from "./MarkdownViewer";
import type {
  MarkdownViewerComponents,
  MermaidExpandedDialogContentProps,
  MermaidExpandedDialogRootProps,
  MermaidExpandedDialogTriggerProps,
  ViewerButtonProps,
  ViewerTooltipProps,
} from "./types";

function Button({ children, ...props }: ViewerButtonProps) {
  return <button {...props}>{children}</button>;
}

function Tooltip({ children }: ViewerTooltipProps) {
  return children as ReactElement;
}

function DialogRoot({ children, open }: MermaidExpandedDialogRootProps) {
  return <div data-dialog-open={open ? "true" : "false"}>{children}</div>;
}

function DialogTrigger({ children }: MermaidExpandedDialogTriggerProps) {
  return children;
}

function DialogContent({ children, description, title }: MermaidExpandedDialogContentProps) {
  return (
    <section data-mermaid-expanded-content>
      <h2>{title}</h2>
      <p>{description}</p>
      {children}
    </section>
  );
}

const components: MarkdownViewerComponents = {
  Button,
  Tooltip,
};

const expandedComponents: MarkdownViewerComponents = {
  ...components,
  MermaidExpandedDialog: {
    Content: DialogContent,
    Root: DialogRoot,
    Trigger: DialogTrigger,
  },
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
  it("renders open and completed task items with distinct icon states", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={parseMarkdownToBlocks(`- [ ] Open task with [link](./next.md)
- [x] Completed task with \`inline code\`
- [X] Uppercase completed task`)}
        components={components}
      />,
    );

    expect(html).toContain('data-task-list-item="true"');
    expect(html).toContain('data-task-checked="false"');
    expect(html).toContain('data-task-checked="true"');
    expect(html).toContain("Open task: ");
    expect(html).toContain("Completed task: ");
    expect(html).toContain("line-through");
    expect(html).toContain('<a href="./next.md">link</a>');
    expect(html).toContain("<code>inline code</code>");
    expect(html).toContain('data-task-summary="true"');
    expect(html).toContain("Completed</span><strong");
    expect(html).toContain(">2</strong>");
    expect(html).toContain("Open</span><strong");
    expect(html).toContain(">1</strong>");
  });

  it("places the task summary after the first h1", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={parseMarkdownToBlocks(`# Feature tasks

Intro text.

- [ ] Open task
- [x] Completed task`)}
        components={components}
      />,
    );

    expect(html.indexOf("Feature tasks")).toBeLessThan(html.indexOf("data-task-summary"));
    expect(html.indexOf("data-task-summary")).toBeLessThan(html.indexOf("Intro text."));
  });

  it("shows scoped task counts below each h1 chapter that contains tasks", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={parseMarkdownToBlocks(`# Chapter A

- [ ] A open
- [x] A done

## A details

- [X] A nested-section done

# Chapter B

No tasks here.

# Chapter C

- [ ] C open`)}
        components={components}
      />,
    );

    const summaries = html.match(/data-task-summary="true"/g) ?? [];
    expect(summaries).toHaveLength(2);

    const chapterAStart = html.indexOf("Chapter A");
    const chapterBStart = html.indexOf("Chapter B");
    const chapterCStart = html.indexOf("Chapter C");
    const firstSummary = html.indexOf("data-task-summary", chapterAStart);
    const secondSummary = html.indexOf("data-task-summary", chapterCStart);

    expect(chapterAStart).toBeLessThan(firstSummary);
    expect(firstSummary).toBeLessThan(chapterBStart);
    expect(html.slice(firstSummary, chapterBStart)).toContain("Completed</span><strong");
    expect(html.slice(firstSummary, chapterBStart)).toContain(">2</strong>");
    expect(html.slice(firstSummary, chapterBStart)).toContain(">1</strong>");
    expect(html.slice(chapterBStart, chapterCStart)).not.toContain("data-task-summary");
    expect(chapterCStart).toBeLessThan(secondSummary);
  });

  it("places the task summary at the top when there is no h1", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={parseMarkdownToBlocks(`Intro text.

- [ ] Open task`)}
        components={components}
      />,
    );

    expect(html.indexOf("data-task-summary")).toBeLessThan(html.indexOf("Intro text."));
  });

  it("keeps ordinary bullets and task-looking code unchanged", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={parseMarkdownToBlocks(`- Ordinary bullet

\`\`\`md
- [ ] code sample
\`\`\``)}
        components={components}
      />,
    );

    expect(html).toContain("Ordinary bullet");
    expect(html).toContain("- [ ] code sample");
    expect(html.match(/data-task-list-item/g)).toBeNull();
    expect(html).not.toContain("data-task-summary");
  });

  it("renders plain and aliased wikilinks while preserving inline code", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={parseMarkdownToBlocks("[[next]] [[plan | 구현 계획]] `[[code]]`")}
        components={components}
      />,
    );

    expect(html).toContain('<a href="./next.md">next</a>');
    expect(html).toContain('<a href="./plan.md">구현 계획</a>');
    expect(html).toContain("<code>[[code]]</code>");
  });

  it("does not render complete HTML comments but preserves code content", () => {
    const html = renderToStaticMarkup(
      <MarkdownViewer
        blocks={parseMarkdownToBlocks("Visible <!-- secret-comment --> text\n\n`<!-- inline -->`\n\n```html\n<!-- fenced -->\n```")}
        components={components}
      />,
    );

    expect(html).not.toContain("secret-comment");
    expect(html).toContain("Visible  text");
    expect(html).toContain("&lt;!-- inline --&gt;");
    expect(html).toContain("&lt;!-- fenced --&gt;");
  });

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

  it("wires optional expanded controls for mermaid blocks without changing fallback timing", () => {
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
        components={expandedComponents}
      />,
    );

    expect(html).toContain("data-mermaid-expanded-view");
    expect(html).toContain('data-dialog-open="false"');
    expect(html).toContain('data-mermaid-status="loading"');
    expect(html).not.toContain('data-mermaid-expanded-trigger="true"');
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
