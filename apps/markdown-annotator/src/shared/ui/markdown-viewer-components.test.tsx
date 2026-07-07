import { describe, expect, it } from "vitest";

import { markdownViewerComponents } from "./markdown-viewer-components";

describe("markdownViewerComponents", () => {
  it("provides Base UI dialog adapters for expanded Mermaid diagrams", () => {
    expect(markdownViewerComponents.MermaidExpandedDialog).toBeDefined();
    expect(markdownViewerComponents.MermaidExpandedDialog?.Root).toBeTypeOf("function");
    expect(markdownViewerComponents.MermaidExpandedDialog?.Trigger).toBeTypeOf("function");
    expect(markdownViewerComponents.MermaidExpandedDialog?.Content).toBeTypeOf("function");
  });
});
