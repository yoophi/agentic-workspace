use crate::domain::{
    errors::WorktreeFileError,
    worktree_file::{WorktreeFileEntry, WorktreeFileListScope, WorktreeTextFile},
};

pub trait WorktreeFileProvider {
    fn list_files(
        &self,
        working_directory: &str,
        scope: &WorktreeFileListScope,
    ) -> Result<Vec<WorktreeFileEntry>, WorktreeFileError>;
    fn read_text_file(
        &self,
        working_directory: &str,
        relative_path: &str,
    ) -> Result<WorktreeTextFile, WorktreeFileError>;
}
