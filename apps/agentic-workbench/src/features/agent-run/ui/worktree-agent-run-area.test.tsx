import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(
  new URL("./worktree-agent-run-area.tsx", import.meta.url),
  "utf8",
);

describe("WorktreeAgentRunArea orchestration runtime ownership", () => {
  it("shares one controller registry and sends Child prompts through the backend command path", () => {
    expect(SOURCE).toContain("new AgentRunControllerRegistry()");
    expect(SOURCE).toContain("existingRunId=");
    expect(SOURCE).toContain("dispatchOrchestrationPrompt");
    expect(SOURCE).toContain("sendOrchestrationChildCommand");
    expect(SOURCE).toContain("listRecoverableOrchestrationWorkspaces");
    expect(SOURCE).toContain("이전 작업 복구");
    expect(SOURCE).toContain("새로 시작");
    expect(SOURCE).toContain("slot.isRunning ? slot.activeRunId : null");
    expect(SOURCE).toContain("prepareMainCoordinatorRun");
    expect(SOURCE).toContain("onBeforeRunStart=");
    expect(SOURCE).toContain("pendingMainStartRef");
    expect(SOURCE).toContain('orchestrationNode?.kind === "child" ? "readOnly"');
  });
});
