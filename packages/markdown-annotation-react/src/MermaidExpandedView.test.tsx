import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  MermaidExpandedBody,
  MermaidExpandedTrigger,
  clampMermaidExpandedZoom,
} from "./MermaidExpandedView";
import type {
  MarkdownViewerComponents,
  MermaidExpandedDialogTriggerProps,
  ViewerButtonProps,
} from "./types";

function Button({ children, ...props }: ViewerButtonProps) {
  return <button {...props}>{children}</button>;
}

function DialogTrigger({ children }: MermaidExpandedDialogTriggerProps) {
  return children;
}

const components: Pick<MarkdownViewerComponents, "Button"> = {
  Button,
};

describe("MermaidExpandedView", () => {
  it("renders an accessible expand trigger for rendered Mermaid diagrams", () => {
    const html = renderToStaticMarkup(
      <MermaidExpandedTrigger
        components={components}
        DialogTrigger={DialogTrigger}
        triggerDataAttribute="data-agent-run-mermaid-expanded-trigger"
      />,
    );

    expect(html).toContain('aria-label="Open Mermaid diagram in full screen"');
    expect(html).toContain('data-mermaid-expanded-trigger="true"');
    expect(html).toContain('data-agent-run-mermaid-expanded-trigger="true"');
  });

  it("renders a fit body sized by zoom percentage for local overflow", () => {
    const html = renderToStaticMarkup(
      <MermaidExpandedBody
        blockId="block-1"
        source={"flowchart TD\n  A --> B"}
        zoomPercent={150}
      />,
    );

    expect(html).toContain("markdown-viewer__mermaid-expanded-body");
    expect(html).toContain("markdown-viewer__mermaid-expanded-zoom-frame");
    expect(html).toContain("height:150%");
    expect(html).toContain("width:150%");
    expect(html).toContain("markdown-viewer__mermaid--fit");
  });

  it("clamps expanded zoom to supported bounds", () => {
    expect(clampMermaidExpandedZoom(0.1)).toBe(0.5);
    expect(clampMermaidExpandedZoom(1.5)).toBe(1.5);
    expect(clampMermaidExpandedZoom(3)).toBe(2.5);
  });
});
