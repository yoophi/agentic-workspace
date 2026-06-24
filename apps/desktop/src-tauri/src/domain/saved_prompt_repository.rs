use crate::domain::saved_prompt::SavedPrompt;

pub trait SavedPromptRepository {
    fn load_saved_prompts(&self) -> Result<Vec<SavedPrompt>, String>;
    fn save_saved_prompts(&self, prompts: &[SavedPrompt]) -> Result<(), String>;
}
