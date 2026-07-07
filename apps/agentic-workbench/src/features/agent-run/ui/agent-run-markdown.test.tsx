import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@yoophi/markdown-annotation-react", () => ({
  MermaidExpandedView: ({ blockId, source }: { blockId: string; source: string }) => (
    <div data-mermaid-status="rendered" data-block-id={blockId}>
      {source}
    </div>
  ),
}));

import { StreamingMarkdown } from "./agent-run-markdown";

describe("StreamingMarkdown", () => {
  it("renders valid Mermaid blocks while preserving ordinary code blocks", () => {
    const html = renderToStaticMarkup(
      <StreamingMarkdown
        content={[
          "Before",
          "",
          "```mermaid",
          "flowchart TD",
          "  A --> B",
          "```",
          "",
          "```ts",
          "const ready = true;",
          "```",
        ].join("\n")}
      />,
    );

    expect(html).toContain('data-mermaid-status="rendered"');
    expect(html).toContain("flowchart TD");
    expect(html).toContain("<pre");
    expect(html).toContain("const ready = true;");
    expect(html).toContain("Before");
  });

  it("renders multiple Mermaid blocks independently", () => {
    const html = renderToStaticMarkup(
      <StreamingMarkdown
        content={[
          "```mermaid",
          "flowchart TD",
          "  A --> B",
          "```",
          "",
          "```mermaid",
          "sequenceDiagram",
          "  Alice->>Bob: Hi",
          "```",
        ].join("\n")}
      />,
    );

    expect(html.match(/data-mermaid-status="rendered"/g)).toHaveLength(2);
    expect(html).toContain("flowchart TD");
    expect(html).toContain("sequenceDiagram");
  });

  it("uses the latest Mermaid source when content changes", () => {
    const firstHtml = renderToStaticMarkup(
      <StreamingMarkdown content={"```mermaid\nflowchart TD\n  A --> B\n```"} />,
    );
    const secondHtml = renderToStaticMarkup(
      <StreamingMarkdown content={"```mermaid\nsequenceDiagram\n  A->>B: Hi\n```"} />,
    );

    expect(firstHtml).toContain("flowchart TD");
    expect(firstHtml).not.toContain("sequenceDiagram");
    expect(secondHtml).toContain("sequenceDiagram");
    expect(secondHtml).not.toContain("flowchart TD");
  });
});
