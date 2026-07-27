import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { splitTileLeaf, createTileLeaf } from "@/entities/agent-run/model/tile-layout";
import { AgentRunTileLayout } from "./agent-run-tile-layout";

describe("AgentRunTileLayout", () => {
  it("renders flat stable panel children and accessible separators", () => {
    const layout = splitTileLeaf(createTileLeaf("main"), "main", "extra", "right", "s1").layout;
    const html = renderToStaticMarkup(
      <AgentRunTileLayout
        layout={layout}
        panels={[
          { panelId: "main", content: <div>Main panel</div> },
          { panelId: "extra", content: <div>Extra panel</div> },
        ]}
        onResizeSplit={() => undefined}
      />,
    );
    expect(html).toContain("Main panel");
    expect(html).toContain("Extra panel");
    expect(html).toContain('role="separator"');
    expect(html).toContain("타일 너비 조절");
  });
});
