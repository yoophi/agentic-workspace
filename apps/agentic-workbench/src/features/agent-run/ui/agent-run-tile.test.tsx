import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AgentRunTile } from "./agent-run-tile";

describe("AgentRunTile", () => {
  it("exposes focus, adjacent-open and close commands", () => {
    const html = renderToStaticMarkup(
      <AgentRunTile
        panelId="extra-1"
        title="Reviewer"
        isFocused
        isRunning
        canClose
        pendingExchangeCount={2}
        onFocus={() => undefined}
        onOpenAdjacent={() => undefined}
        onClose={() => undefined}
        onMessagePeer={() => undefined}
      >
        <div>Panel</div>
      </AgentRunTile>,
    );
    expect(html).toContain('aria-current="true"');
    expect(html).toContain("오른쪽에 새 타일 열기");
    expect(html).toContain("아래에 새 타일 열기");
    expect(html).toContain("다른 에이전트에게 메시지");
    expect(html).toContain("Reviewer 닫기");
  });
});
