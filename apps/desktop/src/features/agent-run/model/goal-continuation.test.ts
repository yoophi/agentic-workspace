import { describe, expect, it } from "vitest";

import type { ThreadGoal } from "@/entities/agent-run/model/types";
import {
  buildGoalContinuationPrompt,
  shouldStartGoalContinuation,
} from "@/features/agent-run/model/goal-continuation";

const activeGoal: ThreadGoal = {
  workingDirectory: "/repo/worktree",
  objective: "Ship the feature",
  status: "active",
  tokenBudget: 1000,
  tokensUsed: 120,
  timeUsedSeconds: 30,
  createdAt: "2026-06-24T00:00:00Z",
  updatedAt: "2026-06-24T00:00:00Z",
};

describe("goal continuation", () => {
  it("starts only when an active goal is idle and no user prompt is waiting", () => {
    expect(
      shouldStartGoalContinuation({
        goal: activeGoal,
        selectedAgentId: "codex",
        isRunning: false,
        hasQueuedPrompt: false,
        promptText: "",
        sessionReady: true,
      }),
    ).toBe(true);

    expect(
      shouldStartGoalContinuation({
        goal: activeGoal,
        selectedAgentId: "codex",
        isRunning: false,
        hasQueuedPrompt: false,
        promptText: "manual prompt",
        sessionReady: true,
      }),
    ).toBe(false);
  });

  it("does not continue after the token budget is exhausted", () => {
    expect(
      shouldStartGoalContinuation({
        goal: { ...activeGoal, tokensUsed: 1000 },
        selectedAgentId: "codex",
        isRunning: false,
        hasQueuedPrompt: false,
        promptText: "",
        sessionReady: true,
      }),
    ).toBe(false);
  });

  it("includes objective and progress context in the continuation prompt", () => {
    expect(buildGoalContinuationPrompt(activeGoal)).toContain("Objective: Ship the feature");
    expect(buildGoalContinuationPrompt(activeGoal)).toContain("Token budget: 120/1000");
  });
});
