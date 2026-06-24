use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::{
    saved_prompt::{SavedPrompt, SavedPromptDraft},
    saved_prompt_repository::SavedPromptRepository,
};

pub fn list_saved_prompts(
    repository: &impl SavedPromptRepository,
) -> Result<Vec<SavedPrompt>, String> {
    repository.load_saved_prompts()
}

pub fn create_saved_prompt(
    repository: &impl SavedPromptRepository,
    draft: SavedPromptDraft,
) -> Result<SavedPrompt, String> {
    let draft = normalize_draft(draft)?;
    let mut prompts = repository.load_saved_prompts()?;
    let prompt = SavedPrompt {
        id: new_saved_prompt_id()?,
        label: draft.label,
        prompt: draft.prompt,
    };

    prompts.push(prompt.clone());
    repository.save_saved_prompts(&prompts)?;

    Ok(prompt)
}

pub fn update_saved_prompt(
    repository: &impl SavedPromptRepository,
    id: String,
    draft: SavedPromptDraft,
) -> Result<SavedPrompt, String> {
    let draft = normalize_draft(draft)?;
    let mut prompts = repository.load_saved_prompts()?;
    let prompt = prompts
        .iter_mut()
        .find(|prompt| prompt.id == id)
        .ok_or_else(|| "Saved prompt not found.".to_owned())?;

    prompt.label = draft.label;
    prompt.prompt = draft.prompt;

    let updated_prompt = prompt.clone();
    repository.save_saved_prompts(&prompts)?;

    Ok(updated_prompt)
}

pub fn delete_saved_prompt(
    repository: &impl SavedPromptRepository,
    id: String,
) -> Result<(), String> {
    let mut prompts = repository.load_saved_prompts()?;
    let original_len = prompts.len();

    prompts.retain(|prompt| prompt.id != id);

    if prompts.len() == original_len {
        return Err("Saved prompt not found.".to_owned());
    }

    repository.save_saved_prompts(&prompts)
}

fn normalize_draft(draft: SavedPromptDraft) -> Result<SavedPromptDraft, String> {
    let label = draft.label.trim().to_owned();
    let prompt = draft.prompt.trim().to_owned();

    if label.is_empty() {
        return Err("Button label is required.".to_owned());
    }

    if prompt.is_empty() {
        return Err("Prompt is required.".to_owned());
    }

    Ok(SavedPromptDraft { label, prompt })
}

fn new_saved_prompt_id() -> Result<String, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("Failed to generate saved prompt id: {error}"))?
        .as_nanos();

    Ok(format!("saved-prompt-{nanos}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct FakeRepository {
        prompts: Mutex<Vec<SavedPrompt>>,
    }

    impl SavedPromptRepository for FakeRepository {
        fn load_saved_prompts(&self) -> Result<Vec<SavedPrompt>, String> {
            Ok(self.prompts.lock().unwrap().clone())
        }

        fn save_saved_prompts(&self, prompts: &[SavedPrompt]) -> Result<(), String> {
            *self.prompts.lock().unwrap() = prompts.to_vec();
            Ok(())
        }
    }

    #[test]
    fn create_saved_prompt_trims_values() {
        let repository = FakeRepository::default();

        let prompt = create_saved_prompt(
            &repository,
            SavedPromptDraft {
                label: " Continue ".into(),
                prompt: " keep going ".into(),
            },
        )
        .expect("prompt created");

        assert_eq!(prompt.label, "Continue");
        assert_eq!(prompt.prompt, "keep going");
        assert_eq!(repository.load_saved_prompts().unwrap(), vec![prompt]);
    }

    #[test]
    fn rejects_empty_saved_prompt_fields() {
        let repository = FakeRepository::default();

        assert_eq!(
            create_saved_prompt(
                &repository,
                SavedPromptDraft {
                    label: " ".into(),
                    prompt: "continue".into(),
                },
            ),
            Err("Button label is required.".to_owned()),
        );
        assert_eq!(
            create_saved_prompt(
                &repository,
                SavedPromptDraft {
                    label: "Continue".into(),
                    prompt: " ".into(),
                },
            ),
            Err("Prompt is required.".to_owned()),
        );
    }
}
