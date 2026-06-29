use crate::domain::{
    worktree_file::{WorktreeFileEntry, WorktreeTextFile},
    worktree_file_provider::WorktreeFileProvider,
};

pub fn list_worktree_files(
    provider: &impl WorktreeFileProvider,
    working_directory: String,
) -> Result<Vec<WorktreeFileEntry>, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    provider.list_files(&working_directory)
}

pub fn read_worktree_text_file(
    provider: &impl WorktreeFileProvider,
    working_directory: String,
    path: String,
) -> Result<WorktreeTextFile, String> {
    let working_directory = normalize_required(working_directory, "Working directory")?;
    let path = normalize_required(path, "File path")?;
    provider.read_text_file(&working_directory, &path)
}

fn normalize_required(value: String, field_name: &str) -> Result<String, String> {
    let value = value.trim().to_owned();

    if value.is_empty() {
        return Err(format!("{field_name} is required."));
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeWorktreeFileProvider;

    impl WorktreeFileProvider for FakeWorktreeFileProvider {
        fn list_files(&self, working_directory: &str) -> Result<Vec<WorktreeFileEntry>, String> {
            Ok(vec![WorktreeFileEntry {
                name: "src".into(),
                path: format!("{working_directory}/src"),
                relative_path: "src".into(),
                is_dir: true,
                size: 0,
                modified_ms: None,
            }])
        }

        fn read_text_file(
            &self,
            working_directory: &str,
            relative_path: &str,
        ) -> Result<WorktreeTextFile, String> {
            Ok(WorktreeTextFile {
                path: format!("{working_directory}/{relative_path}"),
                relative_path: relative_path.into(),
                content: "hello".into(),
                size: 5,
                truncated: false,
            })
        }
    }

    #[test]
    fn trims_working_directory_for_file_listing() {
        let files = list_worktree_files(&FakeWorktreeFileProvider, " /repo/worktree ".into())
            .expect("files should load");

        assert_eq!(files[0].path, "/repo/worktree/src");
    }

    #[test]
    fn rejects_blank_preview_path_before_provider_call() {
        let error = read_worktree_text_file(&FakeWorktreeFileProvider, "/repo".into(), " ".into())
            .expect_err("blank path should fail");

        assert_eq!(error, "File path is required.");
    }
}
