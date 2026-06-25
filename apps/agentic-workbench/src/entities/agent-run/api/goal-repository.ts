import { invoke } from "@tauri-apps/api/core";

import type {
  GoalInput,
  GoalProgressInput,
  GoalUpdateInput,
  ThreadGoal,
} from "@/entities/agent-run/model/types";

export async function getGoal(workingDirectory: string) {
  return invoke<ThreadGoal | null>("get_goal", { workingDirectory });
}

export async function createGoal(input: GoalInput) {
  return invoke<ThreadGoal>("create_goal", { input });
}

export async function updateGoal(
  workingDirectory: string,
  input: GoalUpdateInput,
) {
  return invoke<ThreadGoal>("update_goal", { workingDirectory, input });
}

export async function clearGoal(workingDirectory: string) {
  return invoke<void>("clear_goal", { workingDirectory });
}

export async function recordGoalProgress(
  workingDirectory: string,
  input: GoalProgressInput,
) {
  return invoke<ThreadGoal>("record_goal_progress", { workingDirectory, input });
}
