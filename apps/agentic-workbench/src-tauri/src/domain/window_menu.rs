pub const WINDOW_MENU_ID: &str = "Window";
pub const WINDOW_FOCUS_MENU_ID_PREFIX: &str = "window-focus:";
pub const DEFAULT_MAIN_WINDOW_TITLE: &str = "Agentic Workbench";
pub const DEFAULT_SETTINGS_WINDOW_TITLE: &str = "Settings";
pub const DEFAULT_SESSION_WINDOW_TITLE: &str = "ACP Worktree Session";
pub const DEFAULT_WINDOW_TITLE: &str = "Agentic Workbench Window";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum WindowKind {
    Main,
    Settings,
    Session,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AwWindow {
    pub label: String,
    pub title: String,
    pub kind: WindowKind,
}

impl AwWindow {
    pub fn new(label: impl Into<String>, title: impl Into<String>) -> Option<Self> {
        let label = label.into();
        let label = label.trim();
        if label.is_empty() {
            return None;
        }

        let kind = WindowKind::from_label(label);
        let title = display_title(&title.into(), &kind);

        Some(Self {
            label: label.to_string(),
            title,
            kind,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowMenuEntry {
    pub id: String,
    pub label: String,
    pub target_window_label: String,
    pub enabled: bool,
}

impl WindowMenuEntry {
    pub fn for_window(window: &AwWindow) -> Self {
        Self {
            id: focus_menu_id(&window.label),
            label: window.title.clone(),
            target_window_label: window.label.clone(),
            enabled: true,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WindowMenuState {
    pub window_items: Vec<WindowMenuEntry>,
}

impl WindowKind {
    pub fn from_label(label: &str) -> Self {
        if label == "main" {
            Self::Main
        } else if label == "settings" {
            Self::Settings
        } else if label.starts_with("session-") {
            Self::Session
        } else {
            Self::Other
        }
    }
}

pub fn focus_menu_id(window_label: &str) -> String {
    format!("{WINDOW_FOCUS_MENU_ID_PREFIX}{window_label}")
}

pub fn parse_focus_menu_id(menu_id: &str) -> Option<String> {
    let label = menu_id.strip_prefix(WINDOW_FOCUS_MENU_ID_PREFIX)?.trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

pub fn display_title(raw_title: &str, kind: &WindowKind) -> String {
    let title = raw_title.trim();
    if !title.is_empty() && !title.chars().any(char::is_control) {
        return title.to_string();
    }

    match kind {
        WindowKind::Main => DEFAULT_MAIN_WINDOW_TITLE,
        WindowKind::Settings => DEFAULT_SETTINGS_WINDOW_TITLE,
        WindowKind::Session => DEFAULT_SESSION_WINDOW_TITLE,
        WindowKind::Other => DEFAULT_WINDOW_TITLE,
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        AwWindow, DEFAULT_SESSION_WINDOW_TITLE, WindowKind, display_title, focus_menu_id,
        parse_focus_menu_id,
    };

    #[test]
    fn focus_menu_id_round_trips_window_label() {
        let id = focus_menu_id("session-a");

        assert_eq!(id, "window-focus:session-a");
        assert_eq!(parse_focus_menu_id(&id), Some("session-a".to_string()));
    }

    #[test]
    fn focus_menu_id_parser_rejects_unknown_or_empty_ids() {
        assert_eq!(parse_focus_menu_id("preferences-agentic-workbench"), None);
        assert_eq!(parse_focus_menu_id("window-focus:"), None);
        assert_eq!(parse_focus_menu_id("window-focus:   "), None);
    }

    #[test]
    fn display_title_uses_trimmed_readable_title() {
        assert_eq!(
            display_title("  Project / worktree  ", &WindowKind::Session),
            "Project / worktree"
        );
    }

    #[test]
    fn display_title_falls_back_for_blank_or_control_characters() {
        assert_eq!(
            display_title("   ", &WindowKind::Session),
            DEFAULT_SESSION_WINDOW_TITLE
        );
        assert_eq!(
            display_title("bad\u{0007}title", &WindowKind::Session),
            DEFAULT_SESSION_WINDOW_TITLE
        );
    }

    #[test]
    fn aw_window_rejects_blank_labels_and_classifies_known_labels() {
        assert_eq!(AwWindow::new("   ", "Title"), None);
        assert_eq!(AwWindow::new("main", "").unwrap().kind, WindowKind::Main);
        assert_eq!(
            AwWindow::new("session-abc", "").unwrap().kind,
            WindowKind::Session
        );
    }
}
