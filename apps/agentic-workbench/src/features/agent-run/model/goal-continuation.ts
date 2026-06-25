import type { ThreadGoal } from "@/entities/agent-run/model/types";

export type GoalContinuationState = {
  goal: ThreadGoal | null;
  selectedAgentId: string;
  isRunning: boolean;
  hasQueuedPrompt: boolean;
  promptText: string;
  sessionReady: boolean;
};

export function shouldStartGoalContinuation(state: GoalContinuationState) {
  return Boolean(
    state.goal?.status === "active" &&
      state.selectedAgentId &&
      !state.isRunning &&
      !state.hasQueuedPrompt &&
      !state.promptText.trim() &&
      state.sessionReady &&
      !isGoalBudgetExhausted(state.goal),
  );
}

export function buildGoalContinuationPrompt(goal: ThreadGoal) {
  const budgetLine =
    goal.tokenBudget && goal.tokenBudget > 0
      ? `Token budget: ${goal.tokensUsed}/${goal.tokenBudget}`
      : `Tokens used: ${goal.tokensUsed}`;

  return [
    "Continue the active thread goal.",
    "",
    `Objective: ${goal.objective}`,
    `Status: ${goal.status}`,
    budgetLine,
    `Elapsed goal time: ${goal.timeUsedSeconds}s`,
    "",
    "Continue from the current repository state and make concrete progress toward the objective.",
    "Only treat the goal as complete when the implementation and verification evidence support it.",
    "If goal tools are available, use get_goal to inspect the current goal and update_goal only with complete or blocked when the evidence supports that terminal status.",
    "If the same blocker prevents further progress, report the blocker clearly instead of repeating the same action.",
  ].join("\n");
}

function isGoalBudgetExhausted(goal: ThreadGoal) {
  return Boolean(goal.tokenBudget && goal.tokenBudget > 0 && goal.tokensUsed >= goal.tokenBudget);
}
