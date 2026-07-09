import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const AGENT_RUN_PANEL_SOURCE = readFileSync(
  new URL("./agent-run-panel.tsx", import.meta.url),
  "utf8",
);
const WORKTREE_AGENT_RUN_AREA_SOURCE = readFileSync(
  new URL("./worktree-agent-run-area.tsx", import.meta.url),
  "utf8",
);

describe("agent run settings entrypoints", () => {
  it("keeps the override failure action delegated to onOpenSettings", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("onClick={onOpenSettings}");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("isOverrideCommandFailure(error)");
  });

  it("passes onOpenSettings through without changing panel state", () => {
    expect(WORKTREE_AGENT_RUN_AREA_SOURCE).toContain("onOpenSettings={onOpenSettings}");
    expect(WORKTREE_AGENT_RUN_AREA_SOURCE).not.toContain("openSettingsWindow");
    expect(WORKTREE_AGENT_RUN_AREA_SOURCE).not.toContain("navigate(");
  });
});

describe("agent thread status indicator", () => {
  it("renders compact accessible labels for active, idle, and unknown thread status", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("function AgentThreadStatusBadge");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Agent active");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Agent idle");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Agent status unknown");
  });

  it("handles session_info_update without appending raw timeline items", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("isSessionInfoUpdateEvent(timelineEvent)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("readSessionInfoUpdateMetadata(timelineEvent)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("readAgentThreadStatus(timelineEvent)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("dispatchMcpWindowTitle(metadata.title)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("setSessionUpdatedAt(nextSessionUpdatedAt)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("return;");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain("function isIdleThreadStatusEvent");
  });

  it("renders session freshness in the run header without putting it in the timeline", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("formatSessionFreshnessLabel(sessionUpdatedAt)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("aria-label=\"Session updated at\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("<AgentThreadStatusBadge status={agentThreadStatus} />");
  });
});
