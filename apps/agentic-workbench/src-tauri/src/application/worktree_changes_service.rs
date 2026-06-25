use crate::domain::{
    worktree_change::WorktreeChange, worktree_change_provider::WorktreeChangeProvider,
};

pub fn list_worktree_changes(
    provider: &impl WorktreeChangeProvider,
    working_directory: String,
) -> Result<Vec<WorktreeChange>, String> {
    let working_directory = working_directory.trim();

    if working_directory.is_empty() {
        return Err("Working directory is required.".to_owned());
    }

    provider.list_changes(working_directory)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::domain::worktree_change::WorktreeChangeType;

    struct StubProvider {
        calls: RefCell<Vec<String>>,
        changes: Vec<WorktreeChange>,
    }

    impl WorktreeChangeProvider for StubProvider {
        fn list_changes(&self, working_directory: &str) -> Result<Vec<WorktreeChange>, String> {
            self.calls.borrow_mut().push(working_directory.to_owned());
            Ok(self.changes.clone())
        }
    }

    fn change(path: &str) -> WorktreeChange {
        WorktreeChange {
            path: path.to_owned(),
            old_path: None,
            change_type: WorktreeChangeType::Modified,
            binary: false,
            diff: Some("diff".to_owned()),
            content: None,
            truncated: false,
        }
    }

    #[test]
    fn rejects_blank_working_directory() {
        let provider = StubProvider {
            calls: RefCell::new(Vec::new()),
            changes: Vec::new(),
        };

        let result = list_worktree_changes(&provider, "   ".to_owned());

        assert!(result.is_err());
        assert!(provider.calls.borrow().is_empty());
    }

    #[test]
    fn trims_directory_and_delegates_to_provider() {
        let provider = StubProvider {
            calls: RefCell::new(Vec::new()),
            changes: vec![change("src/lib.rs")],
        };

        let result = list_worktree_changes(&provider, " /repo/worktree ".to_owned())
            .expect("changes should be listed");

        assert_eq!(result.len(), 1);
        assert_eq!(provider.calls.borrow().as_slice(), ["/repo/worktree"]);
    }
}
