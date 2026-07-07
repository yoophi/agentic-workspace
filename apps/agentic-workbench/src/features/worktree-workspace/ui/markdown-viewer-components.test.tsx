import { describe, expect, it } from "vitest";

import { markdownViewerComponents } from "./markdown-viewer-components";

describe("worktree markdownViewerComponents", () => {
  it("provides Radix dialog adapters for expanded Mermaid diagrams", () => {
    expect(markdownViewerComponents.MermaidExpandedDialog).toBeDefined();
    expect(markdownViewerComponents.MermaidExpandedDialog?.Root).toBeTypeOf("function");
    expect(markdownViewerComponents.MermaidExpandedDialog?.Trigger).toBeTypeOf("function");
    expect(markdownViewerComponents.MermaidExpandedDialog?.Content).toBeTypeOf("function");
  });
});
