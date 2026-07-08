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
