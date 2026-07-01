import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@yoophi/markdown-annotation-react", () => ({
  MermaidDiagram: ({
    blockId,
    renderActions,
    source,
  }: {
    blockId: string;
    renderActions?: React.ReactNode;
    source: string;
  }) => (
    <div data-mermaid-status="rendered" data-block-id={blockId}>
      {renderActions}
      <svg role="img">{source}</svg>
    </div>
  ),
}));

import { AgentRunMermaidDiagram, StreamingMarkdown } from "./agent-run-markdown";

describe("StreamingMarkdown Mermaid expanded view", () => {
  it("renders an expanded-view trigger for rendered Mermaid diagrams", () => {
    const html = renderToStaticMarkup(
      <StreamingMarkdown content={"```mermaid\nflowchart TD\n  A --> B\n```"} />,
    );

    expect(html).toContain('data-agent-run-mermaid-expanded-trigger="true"');
    expect(html).toContain('aria-label="Open Mermaid diagram in full screen"');
    expect(html).toContain("flowchart TD");
    expect(html.match(/data-block-id=/g)).toHaveLength(1);
  });

  it("can render the expanded trigger in an open state", () => {
    const html = renderToStaticMarkup(
      <AgentRunMermaidDiagram
        blockId="test-block"
        defaultExpanded
        source={"flowchart TD\n  A --> B"}
      />,
    );

    expect(html).toContain('aria-haspopup="dialog"');
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("flowchart TD");
  });
});
