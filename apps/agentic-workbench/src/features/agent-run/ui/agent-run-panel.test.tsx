import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(new URL("./agent-run-panel.tsx", import.meta.url), "utf8");

describe("AgentRunPanel orchestration view binding", () => {
  it("uses the authoritative existing run and blocks empty mount callbacks during hydration", () => {
    expect(SOURCE).toContain("existingRunId");
    expect(SOURCE).toContain("runtimeHydrated");
    expect(SOURCE).toContain("replayedEvents.slice");
    expect(SOURCE).toContain(
      "if (existingRunId !== undefined && !runtimeHydrated) return",
    );
  });

  it("prepares the Main Coordinator before launching the ACP run", () => {
    expect(SOURCE).toContain("onBeforeRunStart?: (runId: string) => Promise<void>");
    expect(SOURCE).toContain("await onBeforeRunStart(runId)");
    expect(SOURCE).toContain("panelId,");
    expect(SOURCE.indexOf("await onBeforeRunStart(runId)")).toBeLessThan(
      SOURCE.indexOf("await startAgentRun("),
    );
  });

  it("keeps permission mode control available when the shared composer hides panel composers", () => {
    expect(SOURCE).toContain('data-testid="agent-run-permission-mode"');
    expect(SOURCE).toContain("initialPermissionMode?: PermissionMode");
    expect(SOURCE).toContain("if (!activeRunId)");
  });
});
