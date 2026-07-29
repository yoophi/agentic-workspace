import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  toggleWorkspacePanel,
  type WorkspacePanelId,
} from "@/features/worktree-workspace/model/workspace-layout";
import { WorkspacePanelSelector } from "./workspace-panel-selector";

function render(selectedPanel: Parameters<typeof WorkspacePanelSelector>[0]["selectedPanel"]) {
  return renderToStaticMarkup(
    <WorkspacePanelSelector selectedPanel={selectedPanel} onSelect={() => undefined} />,
  );
}

describe("WorkspacePanelSelector", () => {
  it("네 패널 제어 버튼에 접근 가능한 이름을 제공한다", () => {
    const html = render("git");

    expect(html).toContain('aria-label="Git 패널"');
    expect(html).toContain('aria-label="Files 패널"');
    expect(html).toContain('aria-label="Markdown 패널"');
    expect(html).toContain('aria-label="Speckit 패널"');
  });

  it("제어 영역을 이름 있는 그룹으로 노출한다", () => {
    const html = render("git");

    expect(html).toContain('role="group"');
    expect(html).toContain('aria-label="Workspace 패널"');
    // 선택 없음 상태를 표현할 수 없는 tablist는 쓰지 않는다. (research.md 결정 4)
    expect(html).not.toContain('role="tablist"');
    expect(html).not.toContain('role="tab"');
  });

  it("선택 상태를 aria-pressed로 알린다", () => {
    const html = render("markdown");
    const pressed = html.match(/aria-pressed="true"/g) ?? [];

    expect(pressed).toHaveLength(1);
    expect(html).toMatch(/aria-label="Markdown 패널"[^>]*aria-pressed="true"/);
  });

  it("선택 없음 상태에서도 네 버튼을 계속 표시하고 눌린 버튼은 없다", () => {
    const html = render(null);

    expect(html.match(/aria-pressed="false"/g) ?? []).toHaveLength(4);
    expect(html).not.toContain('aria-pressed="true"');
  });

  it("식별 표시를 90도 회전해 배치하고 아이콘은 보조 기술에서 숨긴다", () => {
    const html = render("git");

    expect(html).toContain("rotate-90");
    expect(html).toContain('aria-hidden="true"');
  });

  it("같은 버튼을 빠르게 반복 선택해도 표시/숨김 하나의 상태로만 수렴한다", () => {
    let state: WorkspacePanelId | null = "git";
    const seen: Array<WorkspacePanelId | null> = [];
    for (let click = 0; click < 4; click += 1) {
      state = toggleWorkspacePanel(state, "git");
      seen.push(state);
    }

    expect(seen).toEqual([null, "git", null, "git"]);
  });
});
