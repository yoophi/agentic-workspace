use chrono::Utc;

use crate::domain::{
    goal::{GoalDraft, GoalProgressUpdate, GoalStatus, GoalUpdate, ThreadGoal},
    goal_repository::GoalRepository,
};

pub fn get_goal(
    repository: &impl GoalRepository,
    working_directory: String,
) -> Result<Option<ThreadGoal>, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    Ok(repository
        .load_goals()?
        .into_iter()
        .find(|goal| goal.working_directory == working_directory))
}

pub fn create_goal(
    repository: &impl GoalRepository,
    draft: GoalDraft,
) -> Result<ThreadGoal, String> {
    let draft = normalize_draft(draft)?;
    let mut goals = repository.load_goals()?;
    let now = now_timestamp();
    let goal = ThreadGoal {
        working_directory: draft.working_directory,
        objective: draft.objective,
        status: crate::domain::goal::GoalStatus::Active,
        token_budget: draft.token_budget,
        tokens_used: 0,
        time_used_seconds: 0,
        created_at: now.clone(),
        updated_at: now,
    };

    goals.retain(|existing| existing.working_directory != goal.working_directory);
    goals.push(goal.clone());
    repository.save_goals(&goals)?;

    Ok(goal)
}

pub fn update_goal(
    repository: &impl GoalRepository,
    working_directory: String,
    update: GoalUpdate,
) -> Result<ThreadGoal, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let update = normalize_update(update)?;
    let mut goals = repository.load_goals()?;
    let goal = goals
        .iter_mut()
        .find(|goal| goal.working_directory == working_directory)
        .ok_or_else(|| "Goal not found.".to_owned())?;

    if let Some(objective) = update.objective {
        goal.objective = objective;
    }
    if let Some(status) = update.status {
        goal.status = status;
    }
    if let Some(token_budget) = update.token_budget {
        goal.token_budget = token_budget;
    }
    goal.updated_at = now_timestamp();

    let updated_goal = goal.clone();
    repository.save_goals(&goals)?;

    Ok(updated_goal)
}

/// agent run에서 관측한 token/시간 사용량을 해당 worktree의 goal에 누적한다.
/// goal이 없으면 변경 없이 `Ok(None)`을 반환한다(goal 없는 run은 기존 동작 유지).
pub fn record_goal_usage(
    repository: &impl GoalRepository,
    working_directory: String,
    tokens_used_delta: usize,
    time_used_seconds_delta: u64,
) -> Result<Option<ThreadGoal>, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let mut goals = repository.load_goals()?;

    let Some(goal) = goals
        .iter_mut()
        .find(|goal| goal.working_directory == working_directory)
    else {
        return Ok(None);
    };

    goal.apply_run_usage(tokens_used_delta, time_used_seconds_delta);
    goal.updated_at = now_timestamp();

    let updated_goal = goal.clone();
    repository.save_goals(&goals)?;

    Ok(Some(updated_goal))
}

pub fn clear_goal(
    repository: &impl GoalRepository,
    working_directory: String,
) -> Result<(), String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let mut goals = repository.load_goals()?;
    let original_len = goals.len();

    goals.retain(|goal| goal.working_directory != working_directory);

    if goals.len() == original_len {
        return Err("Goal not found.".to_owned());
    }

    repository.save_goals(&goals)
}

pub fn record_goal_progress(
    repository: &impl GoalRepository,
    working_directory: String,
    progress: GoalProgressUpdate,
) -> Result<ThreadGoal, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let mut goals = repository.load_goals()?;
    let goal = goals
        .iter_mut()
        .find(|goal| goal.working_directory == working_directory)
        .ok_or_else(|| "Goal not found.".to_owned())?;

    goal.tokens_used = goal.tokens_used.max(progress.tokens_used);
    goal.time_used_seconds = goal
        .time_used_seconds
        .saturating_add(progress.time_used_seconds);
    if goal
        .token_budget
        .is_some_and(|budget| budget > 0 && goal.tokens_used >= budget)
    {
        goal.status = GoalStatus::BudgetLimited;
    }
    goal.updated_at = now_timestamp();

    let updated_goal = goal.clone();
    repository.save_goals(&goals)?;

    Ok(updated_goal)
}

fn normalize_draft(draft: GoalDraft) -> Result<GoalDraft, String> {
    Ok(GoalDraft {
        working_directory: normalize_required(draft.working_directory, "Working directory")?,
        objective: normalize_required(draft.objective, "Goal objective")?,
        token_budget: draft.token_budget,
    })
}

fn normalize_update(update: GoalUpdate) -> Result<GoalUpdate, String> {
    let objective = update
        .objective
        .map(|objective| normalize_required(objective, "Goal objective"))
        .transpose()?;

    Ok(GoalUpdate {
        objective,
        status: update.status,
        token_budget: update.token_budget,
    })
}

fn normalize_required(value: String, label: &str) -> Result<String, String> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(format!("{label} is required."));
    }
    Ok(trimmed)
}

fn now_timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::goal::GoalStatus;

    #[derive(Default)]
    struct MemoryGoalRepository {
        goals: RefCell<Vec<ThreadGoal>>,
    }

    impl GoalRepository for MemoryGoalRepository {
        fn load_goals(&self) -> Result<Vec<ThreadGoal>, String> {
            Ok(self.goals.borrow().clone())
        }

        fn save_goals(&self, goals: &[ThreadGoal]) -> Result<(), String> {
            self.goals.replace(goals.to_vec());
            Ok(())
        }
    }

    fn draft(objective: &str) -> GoalDraft {
        GoalDraft {
            working_directory: " /repo/worktree ".into(),
            objective: objective.into(),
            token_budget: Some(100),
        }
    }

    #[test]
    fn create_goal_replaces_existing_goal_for_worktree() {
        let repository = MemoryGoalRepository::default();

        create_goal(&repository, draft("first")).expect("first goal should be created");
        let goal = create_goal(&repository, draft(" second ")).expect("goal should be replaced");

        assert_eq!(goal.working_directory, "/repo/worktree");
        assert_eq!(goal.objective, "second");
        assert_eq!(goal.status, GoalStatus::Active);
        assert_eq!(repository.load_goals().expect("load goals").len(), 1);
    }

    #[test]
    fn update_goal_changes_user_managed_fields() {
        let repository = MemoryGoalRepository::default();
        create_goal(&repository, draft("first")).expect("goal should be created");

        let goal = update_goal(
            &repository,
            "/repo/worktree".into(),
            GoalUpdate {
                objective: Some("revised".into()),
                status: Some(GoalStatus::Paused),
                token_budget: Some(None),
            },
        )
        .expect("goal should update");

        assert_eq!(goal.objective, "revised");
        assert_eq!(goal.status, GoalStatus::Paused);
        assert_eq!(goal.token_budget, None);
    }

    #[test]
    fn record_goal_usage_accumulates_tokens_and_time() {
        let repository = MemoryGoalRepository::default();
        create_goal(&repository, draft("first")).expect("goal should be created");

        let first = record_goal_usage(&repository, "/repo/worktree".into(), 30, 12)
            .expect("usage should record")
            .expect("goal should exist");
        assert_eq!(first.tokens_used, 30);
        assert_eq!(first.time_used_seconds, 12);
        assert_eq!(first.status, GoalStatus::Active);

        let second = record_goal_usage(&repository, "/repo/worktree".into(), 40, 8)
            .expect("usage should record")
            .expect("goal should exist");
        assert_eq!(second.tokens_used, 70);
        assert_eq!(second.time_used_seconds, 20);
    }

    #[test]
    fn record_goal_usage_transitions_to_budget_limited_when_budget_reached() {
        let repository = MemoryGoalRepository::default();
        // draft sets token_budget to Some(100).
        create_goal(&repository, draft("first")).expect("goal should be created");

        let goal = record_goal_usage(&repository, "/repo/worktree".into(), 100, 5)
            .expect("usage should record")
            .expect("goal should exist");

        assert_eq!(goal.tokens_used, 100);
        assert_eq!(goal.status, GoalStatus::BudgetLimited);
    }

    #[test]
    fn record_goal_usage_is_noop_without_goal() {
        let repository = MemoryGoalRepository::default();

        let result = record_goal_usage(&repository, "/repo/worktree".into(), 10, 1)
            .expect("usage recording should succeed");

        assert!(result.is_none());
    }

    #[test]
    fn clear_goal_removes_worktree_goal() {
        let repository = MemoryGoalRepository::default();
        create_goal(&repository, draft("first")).expect("goal should be created");

        clear_goal(&repository, "/repo/worktree".into()).expect("goal should be cleared");

        assert!(
            get_goal(&repository, "/repo/worktree".into())
                .expect("goal lookup should succeed")
                .is_none()
        );
    }

    #[test]
    fn record_goal_progress_tracks_usage_and_budget_limit() {
        let repository = MemoryGoalRepository::default();
        create_goal(&repository, draft("first")).expect("goal should be created");

        let goal = record_goal_progress(
            &repository,
            "/repo/worktree".into(),
            GoalProgressUpdate {
                tokens_used: 120,
                time_used_seconds: 7,
            },
        )
        .expect("goal progress should be recorded");

        assert_eq!(goal.tokens_used, 120);
        assert_eq!(goal.time_used_seconds, 7);
        assert_eq!(goal.status, GoalStatus::BudgetLimited);
    }
}
