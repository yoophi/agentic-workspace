use crate::domain::goal::ThreadGoal;

pub trait GoalRepository {
    fn load_goals(&self) -> Result<Vec<ThreadGoal>, String>;
    fn save_goals(&self, goals: &[ThreadGoal]) -> Result<(), String>;
}
