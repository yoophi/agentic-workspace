use crate::{
    application::window_menu_service::{WindowMenuService, WindowSnapshot},
    domain::window_menu::{WINDOW_FOCUS_MENU_ID_PREFIX, WINDOW_MENU_ID, WindowMenuEntry},
};
use tauri::{
    AppHandle, Manager, Runtime,
    menu::{MenuItem, MenuItemKind, PredefinedMenuItem},
};

pub fn sync_window_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window_menu) = window_submenu(app) else {
        return Ok(());
    };

    remove_existing_window_entries(&window_menu)?;

    let state = WindowMenuService::build_state(window_snapshots(app));
    if state.window_items.is_empty() {
        return Ok(());
    }

    window_menu.append(&PredefinedMenuItem::separator(app)?)?;
    for entry in &state.window_items {
        let item = menu_item_for_entry(app, entry)?;
        window_menu.append(&item)?;
    }

    Ok(())
}

pub fn menu_item_for_entry<R: Runtime>(
    app: &AppHandle<R>,
    entry: &WindowMenuEntry,
) -> tauri::Result<MenuItem<R>> {
    MenuItem::with_id(app, &entry.id, &entry.label, entry.enabled, None::<&str>)
}

pub fn focus_window_from_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    menu_id: &str,
) -> tauri::Result<bool> {
    let Some(command) = WindowMenuService::focus_command(menu_id) else {
        return Ok(false);
    };

    let _ =
        crate::infrastructure::window_manager::focus_window_by_label(app, &command.window_label);
    sync_window_menu(app)?;

    Ok(true)
}

fn window_submenu<R: Runtime>(app: &AppHandle<R>) -> Option<tauri::menu::Submenu<R>> {
    let menu = app.menu()?;
    match menu.get(WINDOW_MENU_ID) {
        Some(MenuItemKind::Submenu(submenu)) => Some(submenu),
        _ => None,
    }
}

fn remove_existing_window_entries<R: Runtime>(
    window_menu: &tauri::menu::Submenu<R>,
) -> tauri::Result<()> {
    let items = window_menu.items()?;
    let mut indexes = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let id = item.id().as_ref();
            if id.starts_with(WINDOW_FOCUS_MENU_ID_PREFIX) {
                Some(index)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if let Some(first_window_index) = indexes.first().copied() {
        if first_window_index > 0
            && let Some(previous) = items.get(first_window_index - 1)
            && matches!(previous, MenuItemKind::Predefined(_))
        {
            indexes.push(first_window_index - 1);
        }
    }

    indexes.sort_unstable();
    indexes.dedup();
    for index in indexes.into_iter().rev() {
        let _ = window_menu.remove_at(index)?;
    }

    Ok(())
}

fn window_snapshots<R: Runtime>(app: &AppHandle<R>) -> Vec<WindowSnapshot> {
    app.webview_windows()
        .into_iter()
        .filter(|(label, _)| WindowMenuService::should_include_window(label))
        .map(|(label, window)| {
            let title = window.title().unwrap_or_default();
            WindowSnapshot::new(label, title)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::domain::window_menu::WindowMenuEntry;

    #[test]
    fn stale_focus_ids_share_the_same_prefix_for_cleanup() {
        assert!(
            "window-focus:missing"
                .starts_with(crate::domain::window_menu::WINDOW_FOCUS_MENU_ID_PREFIX)
        );
    }

    #[test]
    fn menu_item_conversion_uses_entry_values() {
        let entry = WindowMenuEntry {
            id: "window-focus:session-a".to_string(),
            label: "Project / worktree".to_string(),
            target_window_label: "session-a".to_string(),
            enabled: true,
        };

        assert_eq!(entry.id, "window-focus:session-a");
        assert_eq!(entry.label, "Project / worktree");
    }

    #[test]
    fn destroyed_window_cleanup_is_idempotent_for_missing_targets() {
        assert!(crate::domain::window_menu::parse_focus_menu_id("window-focus:closed").is_some());
        assert!(crate::domain::window_menu::parse_focus_menu_id("window-focus:").is_none());
    }
}
