use crate::domain::{
    worktree_workspace_layout::WorkspaceLayoutSettings,
    worktree_workspace_layout_repository::WorkspaceLayoutRepository,
};

pub fn get_layout(
    repository: &impl WorkspaceLayoutRepository,
    working_directory: String,
) -> Result<Option<WorkspaceLayoutSettings>, String> {
    let working_directory = normalize_directory(working_directory)?;
    Ok(repository
        .load_layouts()?
        .into_iter()
        .find(|layout| layout.working_directory == working_directory))
}

/// 같은 Worktree 레코드만 교체한다. 들어온 레코드에 없는 폭은 기존 값을 유지하므로
/// 한 분할을 저장해도 다른 분할 설정이 사라지지 않는다.
pub fn save_layout(
    repository: &impl WorkspaceLayoutRepository,
    layout: WorkspaceLayoutSettings,
) -> Result<WorkspaceLayoutSettings, String> {
    let mut layout = normalize_layout(layout)?;
    let mut layouts = repository.load_layouts()?;
    if let Some(existing) = layouts
        .iter()
        .find(|item| item.working_directory == layout.working_directory)
    {
        layout.outer_panel_width_px = layout
            .outer_panel_width_px
            .or(existing.outer_panel_width_px);
        for (panel, width) in &existing.panel_widths_px {
            layout
                .panel_widths_px
                .entry(panel.clone())
                .or_insert(*width);
        }
    }
    layouts.retain(|item| item.working_directory != layout.working_directory);
    layouts.push(layout.clone());
    repository.save_layouts(&layouts)?;
    Ok(layout)
}

fn normalize_directory(value: String) -> Result<String, String> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err("Working directory is required.".to_string())
    } else {
        Ok(value)
    }
}

fn normalize_layout(
    mut layout: WorkspaceLayoutSettings,
) -> Result<WorkspaceLayoutSettings, String> {
    layout.working_directory = normalize_directory(layout.working_directory)?;
    layout.outer_panel_width_px = layout.outer_panel_width_px.filter(|width| *width > 0);
    layout
        .panel_widths_px
        .retain(|key, width| !key.trim().is_empty() && *width > 0);
    Ok(layout)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeMap};

    use super::*;

    #[derive(Default)]
    struct InMemoryRepository {
        layouts: RefCell<Vec<WorkspaceLayoutSettings>>,
    }

    impl WorkspaceLayoutRepository for InMemoryRepository {
        fn load_layouts(&self) -> Result<Vec<WorkspaceLayoutSettings>, String> {
            Ok(self.layouts.borrow().clone())
        }

        fn save_layouts(&self, layouts: &[WorkspaceLayoutSettings]) -> Result<(), String> {
            *self.layouts.borrow_mut() = layouts.to_vec();
            Ok(())
        }
    }

    fn layout(working_directory: &str) -> WorkspaceLayoutSettings {
        WorkspaceLayoutSettings {
            working_directory: working_directory.to_string(),
            ..Default::default()
        }
    }

    fn panel_widths(entries: &[(&str, u32)]) -> BTreeMap<String, u32> {
        entries
            .iter()
            .map(|(panel, width)| ((*panel).to_string(), *width))
            .collect()
    }

    #[test]
    fn trims_working_directory_and_rejects_blank() {
        let repository = InMemoryRepository::default();

        let saved = save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(640),
                ..layout("  /repo/tree-a  ")
            },
        )
        .expect("save");

        assert_eq!(saved.working_directory, "/repo/tree-a");
        assert!(save_layout(&repository, layout("   ")).is_err());
        assert!(get_layout(&repository, "   ".to_string()).is_err());
    }

    #[test]
    fn keeps_only_positive_widths() {
        let repository = InMemoryRepository::default();

        let saved = save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(0),
                panel_widths_px: panel_widths(&[("git", 520), ("files", 0), ("  ", 300)]),
                ..layout("/repo/tree-a")
            },
        )
        .expect("save");

        assert_eq!(saved.outer_panel_width_px, None);
        assert_eq!(saved.panel_widths_px, panel_widths(&[("git", 520)]));
    }

    #[test]
    fn upserts_the_same_worktree_record() {
        let repository = InMemoryRepository::default();

        save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(640),
                ..layout("/repo/tree-a")
            },
        )
        .expect("first save");
        let saved = save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(900),
                ..layout("/repo/tree-a")
            },
        )
        .expect("second save");

        assert_eq!(saved.outer_panel_width_px, Some(900));
        assert_eq!(repository.load_layouts().expect("load").len(), 1);
    }

    #[test]
    fn isolates_worktrees_from_each_other() {
        let repository = InMemoryRepository::default();

        save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(640),
                panel_widths_px: panel_widths(&[("git", 520)]),
                ..layout("/repo/tree-a")
            },
        )
        .expect("save a");
        save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(1000),
                panel_widths_px: panel_widths(&[("git", 700)]),
                ..layout("/repo/tree-b")
            },
        )
        .expect("save b");

        let a = get_layout(&repository, "/repo/tree-a".to_string())
            .expect("read a")
            .expect("record a");
        let b = get_layout(&repository, "/repo/tree-b".to_string())
            .expect("read b")
            .expect("record b");

        assert_eq!(a.outer_panel_width_px, Some(640));
        assert_eq!(a.panel_widths_px, panel_widths(&[("git", 520)]));
        assert_eq!(b.outer_panel_width_px, Some(1000));
        assert_eq!(b.panel_widths_px, panel_widths(&[("git", 700)]));
    }

    #[test]
    fn saving_one_panel_width_keeps_the_others() {
        let repository = InMemoryRepository::default();

        save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(640),
                panel_widths_px: panel_widths(&[("git", 520), ("markdown", 700)]),
                ..layout("/repo/tree-a")
            },
        )
        .expect("first save");
        let saved = save_layout(
            &repository,
            WorkspaceLayoutSettings {
                panel_widths_px: panel_widths(&[("files", 480)]),
                ..layout("/repo/tree-a")
            },
        )
        .expect("second save");

        // 다른 패널 종류의 내부 B 폭과 바깥 폭은 유지된다.
        assert_eq!(
            saved.panel_widths_px,
            panel_widths(&[("git", 520), ("markdown", 700), ("files", 480)])
        );
        assert_eq!(saved.outer_panel_width_px, Some(640));
    }

    #[test]
    fn saving_outer_width_keeps_inner_panel_widths() {
        let repository = InMemoryRepository::default();

        save_layout(
            &repository,
            WorkspaceLayoutSettings {
                panel_widths_px: panel_widths(&[("git", 520)]),
                ..layout("/repo/tree-a")
            },
        )
        .expect("first save");
        let saved = save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(880),
                ..layout("/repo/tree-a")
            },
        )
        .expect("second save");

        assert_eq!(saved.outer_panel_width_px, Some(880));
        assert_eq!(saved.panel_widths_px, panel_widths(&[("git", 520)]));
    }

    #[test]
    fn unknown_worktree_has_no_layout() {
        let repository = InMemoryRepository::default();

        assert_eq!(
            get_layout(&repository, "/repo/missing".to_string()).expect("read"),
            None
        );
    }

    #[test]
    fn rewriting_one_worktree_does_not_touch_another() {
        // 한 Worktree 레코드를 폭 없이 다시 써도 다른 Worktree 설정은 남는다. (T028)
        let repository = InMemoryRepository::default();
        save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(640),
                ..layout("/repo/tree-a")
            },
        )
        .expect("save a");
        save_layout(
            &repository,
            WorkspaceLayoutSettings {
                outer_panel_width_px: Some(1000),
                ..layout("/repo/tree-b")
            },
        )
        .expect("save b");

        save_layout(&repository, layout("/repo/tree-a")).expect("rewrite a without widths");

        let b = get_layout(&repository, "/repo/tree-b".to_string())
            .expect("read b")
            .expect("record b");
        assert_eq!(b.outer_panel_width_px, Some(1000));
    }
}
