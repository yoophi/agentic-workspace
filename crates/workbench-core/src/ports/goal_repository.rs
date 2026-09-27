//! goal 저장소 port. AW `domain/goal_repository.rs`에서 이동.

use crate::domain::{errors::GoalError, goal::ThreadGoal};

pub trait GoalRepository: Send + Sync {
    fn load_goals(&self) -> Result<Vec<ThreadGoal>, GoalError>;
    fn save_goals(&self, goals: &[ThreadGoal]) -> Result<(), GoalError>;
    fn recover_from_backup(&self) -> Result<(), GoalError>;
}
