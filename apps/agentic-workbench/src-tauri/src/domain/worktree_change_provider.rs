use crate::domain::worktree_change::WorktreeChange;

pub trait WorktreeChangeProvider {
    /// working_directory에서 HEAD 대비 변경된 파일 목록을 반환한다.
    fn list_changes(&self, working_directory: &str) -> Result<Vec<WorktreeChange>, String>;
}
