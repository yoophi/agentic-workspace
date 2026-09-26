//! `goals.json` 어댑터. AW `infrastructure/json_goal_repository.rs`에서 이동(`new(&DataPaths)`).

use crate::{
    domain::{errors::GoalError, goal::ThreadGoal},
    infrastructure::{
        data_paths::DataPaths, json_collection_store::JsonCollectionStore, json_store::StoreError,
    },
    ports::goal_repository::GoalRepository,
};

const LABEL: &str = "goals";

pub struct JsonGoalRepository {
    store: JsonCollectionStore<ThreadGoal>,
}

impl JsonGoalRepository {
    pub fn new(paths: &DataPaths) -> Self {
        Self {
            store: JsonCollectionStore::new(paths.goals_file(), LABEL),
        }
    }
}

fn map_store_error(error: StoreError) -> GoalError {
    match error {
        StoreError::PrimaryCorrupt { .. } => GoalError::StoreCorrupt(error.to_string()),
        other => GoalError::Storage(other.to_string()),
    }
}

impl GoalRepository for JsonGoalRepository {
    fn load_goals(&self) -> Result<Vec<ThreadGoal>, GoalError> {
        self.store.load().map_err(map_store_error)
    }

    fn save_goals(&self, goals: &[ThreadGoal]) -> Result<(), GoalError> {
        self.store.save(goals).map_err(map_store_error)
    }

    fn recover_from_backup(&self) -> Result<(), GoalError> {
        self.store
            .recover_from_backup()
            .map(|_| ())
            .map_err(map_store_error)
    }
}
