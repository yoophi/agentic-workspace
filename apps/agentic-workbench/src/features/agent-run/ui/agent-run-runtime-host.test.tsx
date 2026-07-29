import { describe, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";

import { AgentRunControllerRegistry } from "@/features/agent-run/model/agent-run-controller";
import { hydrateAgentRunController } from "./agent-run-runtime-host";

const { replay } = vi.hoisted(() => ({
  replay: vi.fn().mockResolvedValue({
    runId: "run-child",
    events: [
      {
        runId: "run-child",
        sequence: 1,
        event: { kind: "message", text: "before promotion" },
        terminal: false,
      },
    ],
    lastSequence: 1,
    terminal: false,
    gapDetected: false,
  }),
}));

vi.mock("@/entities/agent-orchestration", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("@/entities/agent-orchestration")>();
  return { ...actual, replayOrchestrationRuntimeEvents: replay };
});

describe("AgentRunRuntimeHost", () => {
  it("hydrates the shared controller with replay payload for background and panel views", async () => {
    const registry = new AgentRunControllerRegistry();
    await hydrateAgentRunController(registry, "child-1", "run-child");
    expect(registry.get("run-child")?.snapshot.events).toEqual([
      { kind: "message", text: "before promotion" },
    ]);
    const background = registry.get("run-child");
    await hydrateAgentRunController(registry, "child-1", "run-child");
    expect(registry.get("run-child")).toBe(background);
    expect(replay).toHaveBeenLastCalledWith("run-child", 1);
  });

  it("does not restart hydration when only panel visibility changes", () => {
    const source = readFileSync(
      new URL("./agent-run-runtime-host.tsx", import.meta.url),
      "utf8",
    );
    expect(source).not.toContain("visiblePanelIds");
    expect(source.match(/controller\.applySnapshot\(snapshot\)/g)).toHaveLength(
      1,
    );
  });
});
