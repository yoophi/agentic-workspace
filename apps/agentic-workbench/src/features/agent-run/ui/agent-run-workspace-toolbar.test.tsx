import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AgentRunWorkspaceToolbar } from "./agent-run-workspace-toolbar";

describe("AgentRunWorkspaceToolbar", () => {
  it("exposes an accessible single-select view mode", () => {
    const html = renderToStaticMarkup(
      <AgentRunWorkspaceToolbar
        viewMode="tiles"
        onViewModeChange={() => undefined}
        onAddPanel={() => undefined}
      />,
    );
    expect(html).toContain('role="group"');
    expect(html).toContain('aria-label="Agent run 보기 방식"');
    expect(html).toContain('aria-pressed="true"');
    expect(html).toContain("탭");
    expect(html).toContain("타일");
    expect(html).toContain("새 에이전트 패널");
    expect(html).toContain("현재 타일 오른쪽에 새 에이전트 패널 열기");
  });

  it("only shows the tile creation action in tile mode", () => {
    const html = renderToStaticMarkup(
      <AgentRunWorkspaceToolbar
        viewMode="tabs"
        onViewModeChange={() => undefined}
        onAddPanel={() => undefined}
      />,
    );
    expect(html).not.toContain("새 에이전트 패널");
  });
});
