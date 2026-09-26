//! `saved-prompts.json` 어댑터. AW `infrastructure/json_saved_prompt_repository.rs`에서 이동(`new(&DataPaths)`).

use crate::{
    domain::{errors::SavedPromptError, saved_prompt::SavedPrompt},
    infrastructure::{
        data_paths::DataPaths, json_collection_store::JsonCollectionStore, json_store::StoreError,
    },
    ports::saved_prompt_repository::SavedPromptRepository,
};

/// AW 원본과 같은 label — 오류 문구 "Failed to read saved prompts store …"를 유지한다.
const LABEL: &str = "saved prompts";

pub struct JsonSavedPromptRepository {
    store: JsonCollectionStore<SavedPrompt>,
}

impl JsonSavedPromptRepository {
    pub fn new(paths: &DataPaths) -> Self {
        Self {
            store: JsonCollectionStore::new(paths.saved_prompts_file(), LABEL),
        }
    }
}

fn map_store_error(error: StoreError) -> SavedPromptError {
    match error {
        StoreError::PrimaryCorrupt { .. } => SavedPromptError::StoreCorrupt(error.to_string()),
        other => SavedPromptError::Storage(other.to_string()),
    }
}

impl SavedPromptRepository for JsonSavedPromptRepository {
    fn load_saved_prompts(&self) -> Result<Vec<SavedPrompt>, SavedPromptError> {
        self.store.load().map_err(map_store_error)
    }

    fn save_saved_prompts(&self, prompts: &[SavedPrompt]) -> Result<(), SavedPromptError> {
        self.store.save(prompts).map_err(map_store_error)
    }

    fn recover_from_backup(&self) -> Result<(), SavedPromptError> {
        self.store
            .recover_from_backup()
            .map(|_| ())
            .map_err(map_store_error)
    }
}
