//! saved prompt 업무 규칙. AW `application/saved_prompt_service.rs`에서 이동, 오류를 `SavedPromptError`로.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    domain::{
        errors::SavedPromptError,
        saved_prompt::{SavedPrompt, SavedPromptDraft},
    },
    ports::saved_prompt_repository::SavedPromptRepository,
};

pub fn list_saved_prompts(
    repository: &dyn SavedPromptRepository,
) -> Result<Vec<SavedPrompt>, SavedPromptError> {
    repository.load_saved_prompts()
}

pub fn create_saved_prompt(
    repository: &dyn SavedPromptRepository,
    draft: SavedPromptDraft,
) -> Result<SavedPrompt, SavedPromptError> {
    create_saved_prompt_with_id(repository, new_saved_prompt_id()?, draft)
}

/// id를 밖에서 정한 생성. `savedPrompt.create` handler가 ledger에 예약한 id로 부른다.
pub fn create_saved_prompt_with_id(
    repository: &dyn SavedPromptRepository,
    id: String,
    draft: SavedPromptDraft,
) -> Result<SavedPrompt, SavedPromptError> {
    let draft = normalize_draft(draft)?;
    let mut prompts = repository.load_saved_prompts()?;
    let prompt = SavedPrompt {
        id,
        label: draft.label,
        prompt: draft.prompt,
    };

    prompts.push(prompt.clone());
    repository.save_saved_prompts(&prompts)?;

    Ok(prompt)
}

pub fn update_saved_prompt(
    repository: &dyn SavedPromptRepository,
    id: String,
    draft: SavedPromptDraft,
) -> Result<SavedPrompt, SavedPromptError> {
    let draft = normalize_draft(draft)?;
    let mut prompts = repository.load_saved_prompts()?;
    let prompt = prompts
        .iter_mut()
        .find(|prompt| prompt.id == id)
        .ok_or(SavedPromptError::NotFound)?;

    prompt.label = draft.label;
    prompt.prompt = draft.prompt;

    let updated_prompt = prompt.clone();
    repository.save_saved_prompts(&prompts)?;

    Ok(updated_prompt)
}

pub fn delete_saved_prompt(
    repository: &dyn SavedPromptRepository,
    id: String,
) -> Result<(), SavedPromptError> {
    let mut prompts = repository.load_saved_prompts()?;
    let original_len = prompts.len();

    prompts.retain(|prompt| prompt.id != id);

    if prompts.len() == original_len {
        return Err(SavedPromptError::NotFound);
    }

    repository.save_saved_prompts(&prompts)
}

pub fn normalize_draft(draft: SavedPromptDraft) -> Result<SavedPromptDraft, SavedPromptError> {
    let label = draft.label.trim().to_owned();
    let prompt = draft.prompt.trim().to_owned();

    if label.is_empty() {
        return Err(SavedPromptError::Required("Button label"));
    }

    if prompt.is_empty() {
        return Err(SavedPromptError::Required("Prompt"));
    }

    Ok(SavedPromptDraft { label, prompt })
}

/// 기존 id 형식 유지(저장 파일 호환).
pub fn new_saved_prompt_id() -> Result<String, SavedPromptError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            SavedPromptError::Clock(format!("Failed to generate saved prompt id: {error}"))
        })?
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
        fn load_saved_prompts(&self) -> Result<Vec<SavedPrompt>, SavedPromptError> {
            Ok(self.prompts.lock().unwrap().clone())
        }

        fn save_saved_prompts(&self, prompts: &[SavedPrompt]) -> Result<(), SavedPromptError> {
            *self.prompts.lock().unwrap() = prompts.to_vec();
            Ok(())
        }

        fn recover_from_backup(&self) -> Result<(), SavedPromptError> {
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
        assert!(prompt.id.starts_with("saved-prompt-"));
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
            Err(SavedPromptError::Required("Button label")),
        );
        assert_eq!(
            create_saved_prompt(
                &repository,
                SavedPromptDraft {
                    label: "Continue".into(),
                    prompt: " ".into(),
                },
            ),
            Err(SavedPromptError::Required("Prompt")),
        );
    }

    #[test]
    fn update_and_delete_report_not_found() {
        let repository = FakeRepository::default();
        assert_eq!(
            update_saved_prompt(
                &repository,
                "missing".into(),
                SavedPromptDraft {
                    label: "a".into(),
                    prompt: "b".into()
                }
            ),
            Err(SavedPromptError::NotFound)
        );
        assert_eq!(
            delete_saved_prompt(&repository, "missing".into()),
            Err(SavedPromptError::NotFound)
        );
    }
}
