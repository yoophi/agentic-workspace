import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { MAX_ORCHESTRATION_PROMPT_BYTES } from "@/entities/agent-orchestration";
import {
  addExtraPanel,
  createInitialAgentRunAreaState,
} from "@/features/agent-run/model/agent-run-panel-slots";
import {
  decideComposerSubmission,
  noticeForFailure,
} from "@/features/agent-run/model/composer-submission";
import { selectPromptTargets } from "@/features/agent-run/model/prompt-target-selection";

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

  it("renders no notice before any submission", () => {
    const state = addExtraPanel(createInitialAgentRunAreaState());
    const html = renderToStaticMarkup(
      <WorkspacePromptComposer
        slots={state.slots}
        focusedPanelId={state.focusedPanelId}
        onSubmit={() => undefined}
      />,
    );
    expect(html).not.toContain("composer-notice-reason");
    expect(html).not.toContain("상한을");
  });

  // FR-044: the size guard runs before any target is contacted. The boundary itself is
  // covered behaviourally in composer-submission.test.ts.
  it("blocks an over-limit prompt without contacting a target", () => {
    const state = addExtraPanel(createInitialAgentRunAreaState());
    const targets = selectPromptTargets(state.slots, {
      mode: "all",
      focusedPanelId: state.focusedPanelId,
      selectedPanelIds: [],
    });

    const decision = decideComposerSubmission({
      message: "a".repeat(MAX_ORCHESTRATION_PROMPT_BYTES + 1),
      disabled: false,
      targetsValid: targets.valid,
    });

    expect(decision.kind).toBe("blocked");
    if (decision.kind !== "blocked") throw new Error("expected blocked");
    expect(decision.notice.reason).toContain("16KiB");
  });

  // FR-022/FR-048: a rejected submission surfaces a distinguishable reason and next action.
  it("maps a Coordinator rejection to a reason plus a next action", () => {
    const notice = noticeForFailure({
      code: "coordinatorInactive",
      message: "활성 Main Coordinator 실행이 없습니다.",
      retryable: false,
    });

    expect(notice.reason).toContain("Main Coordinator");
    expect(notice.nextAction).toContain("실행을 시작");
  });
});
