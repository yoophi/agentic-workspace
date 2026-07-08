use crate::domain::window_menu::{
    AwWindow, WindowKind, WindowMenuEntry, WindowMenuState, parse_focus_menu_id,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowSnapshot {
    pub label: String,
    pub title: String,
}

impl WindowSnapshot {
    pub fn new(label: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            title: title.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusWindowCommand {
    pub window_label: String,
}

pub struct WindowMenuService;

impl WindowMenuService {
    pub fn build_state(windows: impl IntoIterator<Item = WindowSnapshot>) -> WindowMenuState {
        let mut windows = windows
            .into_iter()
            .filter_map(|snapshot| AwWindow::new(snapshot.label, snapshot.title))
            .collect::<Vec<_>>();

        windows.sort_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.label.cmp(&right.label))
        });

        WindowMenuState {
            window_items: windows.iter().map(WindowMenuEntry::for_window).collect(),
        }
    }

    pub fn focus_command(menu_id: &str) -> Option<FocusWindowCommand> {
        parse_focus_menu_id(menu_id).map(|window_label| FocusWindowCommand { window_label })
    }

    pub fn should_include_window(label: &str) -> bool {
        matches!(
            WindowKind::from_label(label),
            WindowKind::Main | WindowKind::Settings | WindowKind::Session
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{WindowMenuService, WindowSnapshot};

    #[test]
    fn maps_window_snapshots_to_stable_menu_items() {
        let state = WindowMenuService::build_state([
            WindowSnapshot::new("session-b", "Project / b"),
            WindowSnapshot::new("main", "Agentic Workbench"),
            WindowSnapshot::new("session-a", "Project / a"),
        ]);

        let ids = state
            .window_items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "window-focus:main",
                "window-focus:session-a",
                "window-focus:session-b"
            ]
        );
    }

    #[test]
    fn includes_main_settings_and_session_window_labels() {
        assert!(WindowMenuService::should_include_window("main"));
        assert!(WindowMenuService::should_include_window("settings"));
        assert!(WindowMenuService::should_include_window("session-abc"));
        assert!(!WindowMenuService::should_include_window("devtools"));
    }

    #[test]
    fn builds_focus_command_only_for_window_focus_ids() {
        assert_eq!(
            WindowMenuService::focus_command("window-focus:session-a")
                .unwrap()
                .window_label,
            "session-a"
        );
        assert_eq!(
            WindowMenuService::focus_command("preferences-agentic-workbench"),
            None
        );
        assert_eq!(WindowMenuService::focus_command("window-focus:"), None);
    }

    #[test]
    fn keeps_duplicate_titles_as_distinct_targets() {
        let state = WindowMenuService::build_state([
            WindowSnapshot::new("session-a", "Same"),
            WindowSnapshot::new("session-b", "Same"),
        ]);

        assert_eq!(state.window_items[0].label, "Same");
        assert_eq!(state.window_items[1].label, "Same");
        assert_ne!(state.window_items[0].id, state.window_items[1].id);
        assert_ne!(
            state.window_items[0].target_window_label,
            state.window_items[1].target_window_label
        );
    }
}
