use crate::domain::worktree_file::{WorktreeFileEntry, WorktreeFileListScope, WorktreeTextFile};

pub trait WorktreeFileProvider {
    fn list_files(
        &self,
        working_directory: &str,
        scope: &WorktreeFileListScope,
    ) -> Result<Vec<WorktreeFileEntry>, String>;
    fn read_text_file(
        &self,
        working_directory: &str,
        relative_path: &str,
    ) -> Result<WorktreeTextFile, String>;
}
