import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const source = readFileSync(new URL("./project-worktree-session-page.tsx", import.meta.url), "utf8");

describe("ProjectWorktreeSessionPage SDD prompt routing", () => {
  it("forwards SDD delivery mode to the active agent area", () => {
    expect(source).toContain("onSendSddPrompt={(request) =>");
    expect(source).toContain("delivery: request.delivery");
  });
});

describe("ProjectWorktreeSessionPage workspace 숨김", () => {
  it("선택 없음 상태에서는 Workspace B와 바깥 분할 핸들을 렌더링하지 않는다", () => {
    // 두 요소 모두 selectedPanel 조건 안에 있어야 빈 패널 영역이 남지 않는다. (FR-006)
    expect(source).toMatch(/\{selectedPanel \?\s*\(\s*<ResizableHandle/);
    expect(source).toContain("{selectedPanel && hydrated ? (");
    expect(source).toContain('id="project-worktree-session-workspace"');
  });

  it("에이전트 영역은 선택 상태와 무관하게 항상 마운트를 유지한다", () => {
    // A는 조건부 렌더링 밖에 있어야 프롬프트 입력과 실행 상태가 끊기지 않는다. (FR-008)
    const agentIndex = source.indexOf("<WorktreeAgentRunArea");
    const firstConditionalIndex = source.indexOf("{selectedPanel ?");

    expect(agentIndex).toBeGreaterThan(-1);
    expect(agentIndex).toBeLessThan(firstConditionalIndex);
  });

  it("Workspace를 다시 표시해도 프롬프트 전달 상태를 페이지가 계속 소유한다", () => {
    // workspacePromptRequest가 페이지 상태이므로 Workspace 재마운트에도 유지된다. (FR-008, FR-009)
    expect(source).toContain("const [workspacePromptRequest, setWorkspacePromptRequest]");
    expect(source).toContain("externalPromptRequest={workspacePromptRequest}");
  });

  it("제어 버튼 영역은 분할 그룹 밖 가장 오른쪽에 둔다", () => {
    const groupEnd = source.indexOf("</ResizablePanelGroup>");
    const selectorIndex = source.indexOf("<WorkspacePanelSelector");

    expect(selectorIndex).toBeGreaterThan(groupEnd);
  });
});

describe("ProjectWorktreeSessionPage 바깥 B 폭", () => {
  it("저장된 레이아웃을 읽은 뒤에만 B를 마운트한다", () => {
    // hydrate 전에 기본 폭으로 그렸다가 다시 그리는 재마운트를 만들지 않는다. (research.md 결정 5)
    expect(source).toContain("{selectedPanel && hydrated ? (");
    expect(source).not.toContain("key={layout?.outerPanelWidthPx");
  });

  it("A에는 저장 크기를 주지 않고 B에만 저장 폭을 적용한다", () => {
    // A는 남은 공간을 채운다. (FR-012)
    expect(source).toMatch(
      /id="project-worktree-session-agent" minSize=\{`\$\{OUTER_MINIMUM_A_PX\}px`\}/,
    );
    expect(source).toContain("{...outerSplit.panelProps}");
  });

  it("분할선 조작 의도와 안정 시점 저장을 연결한다", () => {
    expect(source).toContain("{...outerSplit.separatorProps}");
    expect(source).toContain("{...outerSplit.groupProps}");
  });

  it("내부 B 폭 저장 경로를 Workspace 패널에 전달한다", () => {
    expect(source).toContain("panelWidthsPx={layout?.panelWidthsPx}");
    expect(source).toContain("onPersistPanelWidth={persistPanelWidth}");
    expect(source).toContain("layoutHydrated={hydrated}");
  });
});
