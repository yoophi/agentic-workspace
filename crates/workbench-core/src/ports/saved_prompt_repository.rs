//! saved prompt 저장소 port. AW `domain/saved_prompt_repository.rs`에서 이동, 오류 enum과 복구 메서드를 더했다.

use crate::domain::{errors::SavedPromptError, saved_prompt::SavedPrompt};

pub trait SavedPromptRepository: Send + Sync {
    /// 읽기 전용. 파일이 없으면 빈 벡터, 파싱 실패면 `StoreCorrupt`. 어떤 경우에도 쓰지 않는다.
    fn load_saved_prompts(&self) -> Result<Vec<SavedPrompt>, SavedPromptError>;
    fn save_saved_prompts(&self, prompts: &[SavedPrompt]) -> Result<(), SavedPromptError>;
    /// **aggregate lock을 잡은 호출자만** 부른다.
    fn recover_from_backup(&self) -> Result<(), SavedPromptError>;
}
