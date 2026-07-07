import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@yoophi/markdown-annotation-react", () => ({
  MermaidExpandedView: ({
    source,
  }: {
    blockId: string;
    components: unknown;
    source: string;
  }) =>
    source.trim() && !source.includes("broken") ? (
      <div data-mermaid-status="rendered">
        <button
          type="button"
          aria-label="Open Mermaid diagram in full screen"
          data-agent-run-mermaid-expanded-trigger="true"
        >
          Expand
        </button>
        {source}
      </div>
    ) : (
      <div data-mermaid-status="failed">
        <p>Mermaid diagram failed to render</p>
        <pre>{source || "Mermaid diagram source is empty."}</pre>
      </div>
    ),
}));

import { StreamingMarkdown } from "./agent-run-markdown";

describe("StreamingMarkdown Mermaid expanded fallback states", () => {
  it("does not expose expanded view for empty Mermaid fallback states", () => {
    const html = renderToStaticMarkup(<StreamingMarkdown content={"```mermaid\n```"} />);

    expect(html).toContain('data-mermaid-status="failed"');
    expect(html).not.toContain('data-agent-run-mermaid-expanded-trigger="true"');
    expect(html).not.toContain('aria-label="Open Mermaid diagram in full screen"');
  });

  it("does not expose expanded view for failed Mermaid fallback states", () => {
    const html = renderToStaticMarkup(
      <StreamingMarkdown content={"```mermaid\nbroken diagram\n```"} />,
    );

    expect(html).toContain('data-mermaid-status="failed"');
    expect(html).toContain("broken diagram");
    expect(html).not.toContain('data-agent-run-mermaid-expanded-trigger="true"');
    expect(html).not.toContain('aria-label="Open Mermaid diagram in full screen"');
  });
});
