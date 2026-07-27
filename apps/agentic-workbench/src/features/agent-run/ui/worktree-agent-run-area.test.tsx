import { describe, expect, it } from "vitest";

import { readFileSync } from "node:fs";

const SOURCE = readFileSync(new URL("./worktree-agent-run-area.tsx", import.meta.url), "utf8");

describe("WorktreeAgentRunArea tile contract", () => {
  it("keeps panels keyed only by stable slot ids across projections", () => {
    expect(SOURCE).toContain("key={slot.id}");
    expect(SOURCE).toContain("state.viewMode");
    expect(SOURCE).toContain("AgentRunTileLayout");
  });
});
