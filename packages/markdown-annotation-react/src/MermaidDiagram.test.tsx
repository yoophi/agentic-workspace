import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  MermaidDiagram,
  createMermaidRenderId,
  createMermaidSourceHash,
  emptyMermaidFailure,
  removeMermaidRenderArtifacts,
  toMermaidFailure,
} from "./MermaidDiagram";

describe("MermaidDiagram", () => {
  it("renders a loading state for valid mermaid sources before async rendering completes", () => {
    const html = renderToStaticMarkup(
      <MermaidDiagram blockId="block-1" source={"flowchart TD\n  A --> B"} />,
    );

    expect(html).toContain('data-mermaid-status="loading"');
    expect(html).toContain("Rendering Mermaid diagram");
  });

  it("renders a block-local fallback for empty sources", () => {
    const html = renderToStaticMarkup(<MermaidDiagram blockId="block-1" source={" \n "} />);

    expect(html).toContain('data-mermaid-status="failed"');
    expect(html).toContain("Mermaid diagram source is empty.");
    expect(html).toContain("empty-source");
  });

  it("does not render actions before the diagram reaches rendered state", () => {
    const loadingHtml = renderToStaticMarkup(
      <MermaidDiagram
        blockId="block-1"
        source={"flowchart TD\n  A --> B"}
        renderActions={<button type="button">Expand</button>}
      />,
    );
    const failedHtml = renderToStaticMarkup(
      <MermaidDiagram
        blockId="block-1"
        source={" \n "}
        renderActions={<button type="button">Expand</button>}
      />,
    );

    expect(loadingHtml).not.toContain("Expand");
    expect(failedHtml).not.toContain("Expand");
  });

  it("maps parser-like errors to syntax failure reasons", () => {
    expect(toMermaidFailure(new Error("Parse error on line 2"))).toEqual({
      category: "syntax-or-parse-error",
      reason: "Parse error on line 2",
    });
  });

  it("maps unknown renderer errors to runtime failure reasons", () => {
    expect(toMermaidFailure(undefined)).toEqual({
      category: "renderer-runtime-error",
      reason: "Mermaid diagram rendering failed.",
    });
  });

  it("provides a stable empty-source failure object", () => {
    expect(emptyMermaidFailure()).toEqual({
      category: "empty-source",
      reason: "Mermaid diagram source is empty.",
    });
  });

  it("normalizes render ids for Mermaid", () => {
    expect(createMermaidRenderId(":r1:", "block/1", "flowchart TD")).toBe(
      `mermaid-r1-block-1-${createMermaidSourceHash("flowchart TD")}`,
    );
  });

  it("changes render ids when source changes", () => {
    expect(createMermaidRenderId(":r1:", "block-1", "flowchart TD")).not.toBe(
      createMermaidRenderId(":r1:", "block-1", "sequenceDiagram"),
    );
  });

  it("removes Mermaid render artifacts created outside the React tree", () => {
    const existingIds = new Set(["diagram-1", "ddiagram-1", "idiagram-1", "other-node"]);
    const removedIds: string[] = [];
    const root = {
      getElementById(id: string) {
        if (!existingIds.has(id)) {
          return null;
        }

        return {
          remove() {
            existingIds.delete(id);
            removedIds.push(id);
          },
        };
      },
    };

    removeMermaidRenderArtifacts("diagram-1", root);

    expect(removedIds).toEqual(["diagram-1", "ddiagram-1", "idiagram-1"]);
    expect(existingIds).toEqual(new Set(["other-node"]));
  });
});
