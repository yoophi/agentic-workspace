import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  addExtraPanel,
  createInitialAgentRunAreaState,
} from "@/features/agent-run/model/agent-run-panel-slots";

import { WorkspacePromptComposer } from "./workspace-prompt-composer";

describe("WorkspacePromptComposer", () => {
  it("renders one accessible composer with four target modes", () => {
    const state = addExtraPanel(createInitialAgentRunAreaState());
    const html = renderToStaticMarkup(
      <WorkspacePromptComposer
        slots={state.slots}
        focusedPanelId={state.focusedPanelId}
        onSubmit={() => undefined}
      />,
    );
    expect(html).toContain('aria-label="Workspace prompt composer"');
    expect(html).toContain('role="radiogroup"');
    expect(html).toContain("포커스");
    expect(html).toContain("선택");
    expect(html).toContain("전체");
    expect(html).toContain("Main 위임");
    expect(html).toContain("⌘/Ctrl+Enter");
  });

  it("restores keyboard focus to the shared composer after submit", () => {
    const source = WorkspacePromptComposer.toString();
    expect(source).toContain("composerRef");
    expect(source).toContain(".focus()");
  });
});
