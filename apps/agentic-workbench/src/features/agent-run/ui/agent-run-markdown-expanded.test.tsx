import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@yoophi/markdown-annotation-react", () => ({
  MermaidExpandedView: ({
    blockId,
    defaultExpanded,
    source,
  }: {
    blockId: string;
    components: unknown;
    defaultExpanded?: boolean;
    source: string;
    triggerDataAttribute?: string;
  }) => (
    <div data-mermaid-status="rendered" data-block-id={blockId}>
      <button
        type="button"
        aria-haspopup="dialog"
        aria-expanded={defaultExpanded ? "true" : "false"}
        aria-label="Open Mermaid diagram in full screen"
        data-agent-run-mermaid-expanded-trigger="true"
      >
        Expand
      </button>
      <svg role="img">{source}</svg>
    </div>
  ),
  MermaidExpandedBody: ({
    blockId,
    source,
    zoomPercent,
  }: {
    blockId: string;
    source: string;
    zoomPercent: number;
  }) => (
    <div data-mermaid-status="rendered" data-block-id={`${blockId}-expanded`} data-mermaid-fit="true">
      <div style={{ height: `${zoomPercent}%`, width: `${zoomPercent}%` }}>{source}</div>
    </div>
  ),
}));

import {
  AgentRunMermaidDiagram,
  AgentRunMermaidExpandedBody,
  StreamingMarkdown,
} from "./agent-run-markdown";

describe("StreamingMarkdown Mermaid expanded view", () => {
  it("renders an expanded-view trigger for rendered Mermaid diagrams", () => {
    const html = renderToStaticMarkup(
      <StreamingMarkdown content={"```mermaid\nflowchart TD\n  A --> B\n```"} />,
    );

    expect(html).toContain('data-agent-run-mermaid-expanded-trigger="true"');
    expect(html).toContain('aria-label="Open Mermaid diagram in full screen"');
    expect(html).toContain("flowchart TD");
    expect(html.match(/data-block-id=/g)).toHaveLength(1);
    expect(html).not.toContain('data-mermaid-fit="true"');
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

  it("renders the expanded body with a fit diagram sized by the zoom level", () => {
    const html = renderToStaticMarkup(
      <AgentRunMermaidExpandedBody
        blockId="test-block"
        source={"flowchart TD\n  A --> B"}
        zoomPercent={150}
      />,
    );

    expect(html).toContain('data-mermaid-fit="true"');
    expect(html).toContain('data-block-id="test-block-expanded"');
    expect(html).toContain("height:150%");
    expect(html).toContain("width:150%");
  });
});
