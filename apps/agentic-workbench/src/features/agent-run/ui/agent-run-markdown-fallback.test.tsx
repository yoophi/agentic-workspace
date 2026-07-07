import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@yoophi/markdown-annotation-react", () => ({
  MermaidExpandedView: ({ source }: { blockId: string; source: string }) =>
    source.trim() && !source.includes("broken") ? (
      <div data-mermaid-status="rendered">{source}</div>
    ) : (
      <div data-mermaid-status="failed">
        <p>Mermaid diagram failed to render</p>
        <pre>{source || "Mermaid diagram source is empty."}</pre>
      </div>
    ),
}));

import { StreamingMarkdown } from "./agent-run-markdown";

describe("StreamingMarkdown Mermaid fallback", () => {
  it("shows fallback for empty Mermaid source", () => {
    const html = renderToStaticMarkup(<StreamingMarkdown content={"```mermaid\n```"} />);

    expect(html).toContain('data-mermaid-status="failed"');
    expect(html).toContain("Mermaid diagram source is empty.");
  });

  it("shows fallback for malformed Mermaid source", () => {
    const html = renderToStaticMarkup(
      <StreamingMarkdown content={"```mermaid\nbroken diagram\n```"} />,
    );

    expect(html).toContain('data-mermaid-status="failed"');
    expect(html).toContain("Mermaid diagram failed to render");
    expect(html).toContain("broken diagram");
  });
});
