import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(
  new URL("./worktree-agent-run-area.tsx", import.meta.url),
  "utf8",
);

describe("WorktreeAgentRunArea orchestration runtime ownership", () => {
  it("shares one controller registry and binds Child panels to their orchestration runs", () => {
    expect(SOURCE).toContain("new AgentRunControllerRegistry()");
    expect(SOURCE).toContain("existingRunId=");
    expect(SOURCE).toContain("listRecoverableOrchestrationWorkspaces");
    expect(SOURCE).toContain("이전 작업 복구");
    expect(SOURCE).toContain("새로 시작");
    expect(SOURCE).toContain("slot.isRunning ? slot.activeRunId : null");
    expect(SOURCE).toContain("prepareMainCoordinatorRun");
    expect(SOURCE).toContain("onBeforeRunStart=");
    expect(SOURCE).toContain("pendingMainStartRef");
    expect(SOURCE).toContain('orchestrationNode?.kind === "child" ? "readOnly"');
    expect(SOURCE).toContain("worktreeRunConfiguration={worktreeRunConfiguration}");
    expect(SOURCE).toContain(
      "onWorktreeRunConfigurationChange={setWorktreeRunConfiguration}",
    );
  });

  it("lets every panel own its prompt input instead of a workspace-wide composer", () => {
    expect(SOURCE).not.toContain("WorkspacePromptComposer");
    expect(SOURCE).not.toContain("showPromptComposer");
    expect(SOURCE).not.toContain("runConfigurationPortal");
    expect(SOURCE).not.toContain("PromptDispatchStatus");
  });
});
