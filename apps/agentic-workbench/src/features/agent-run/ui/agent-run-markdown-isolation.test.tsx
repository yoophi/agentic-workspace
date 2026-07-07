import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

vi.mock("@yoophi/markdown-annotation-react", () => ({
  MermaidExpandedView: ({ source }: { blockId: string; source: string }) =>
    source.includes("broken") ? (
      <div data-mermaid-status="failed">Mermaid failed: {source}</div>
    ) : (
      <div data-mermaid-status="rendered">{source}</div>
    ),
}));

import { StreamingMarkdown } from "./agent-run-markdown";

describe("StreamingMarkdown Mermaid isolation", () => {
  it("keeps surrounding Markdown and adjacent code readable when one Mermaid block fails", () => {
    const html = renderToStaticMarkup(
      <StreamingMarkdown
        content={[
          "Before text",
          "",
          "```mermaid",
          "broken diagram",
          "```",
          "",
          "```json",
          "{\"ok\": true}",
          "```",
          "",
          "After text",
        ].join("\n")}
      />,
    );

    expect(html).toContain('data-mermaid-status="failed"');
    expect(html).toContain("Before text");
    expect(html).toContain("{&quot;ok&quot;: true}");
    expect(html).toContain("After text");
  });
});
