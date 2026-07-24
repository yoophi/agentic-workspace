use crate::domain::{worktree_workspace_layout::WorkspaceLayoutSettings, worktree_workspace_layout_repository::WorkspaceLayoutRepository};

pub fn get_layout(repository: &impl WorkspaceLayoutRepository, working_directory: String) -> Result<Option<WorkspaceLayoutSettings>, String> {
    let working_directory = normalize_directory(working_directory)?;
    Ok(repository.load_layouts()?.into_iter().find(|layout| layout.working_directory == working_directory))
}

pub fn save_layout(repository: &impl WorkspaceLayoutRepository, layout: WorkspaceLayoutSettings) -> Result<WorkspaceLayoutSettings, String> {
    let mut layout = normalize_layout(layout)?;
    let mut layouts = repository.load_layouts()?;
    if let Some(existing) = layouts.iter().find(|item| item.working_directory == layout.working_directory) {
        layout.window_x = layout.window_x.or(existing.window_x);
        layout.window_y = layout.window_y.or(existing.window_y);
        layout.window_width = layout.window_width.or(existing.window_width);
        layout.window_height = layout.window_height.or(existing.window_height);
        for (panel, width) in &existing.panel_widths_px {
            layout.panel_widths_px.entry(panel.clone()).or_insert(*width);
        }
    }
    layouts.retain(|item| item.working_directory != layout.working_directory);
    layouts.push(layout.clone());
    repository.save_layouts(&layouts)?;
    Ok(layout)
}

fn normalize_directory(value: String) -> Result<String, String> {
    let value = value.trim().to_owned();
    if value.is_empty() { Err("Working directory is required.".to_string()) } else { Ok(value) }
}

fn normalize_layout(mut layout: WorkspaceLayoutSettings) -> Result<WorkspaceLayoutSettings, String> {
    layout.working_directory = normalize_directory(layout.working_directory)?;
    layout.outer_panel_width_px = layout.outer_panel_width_px.filter(|width| *width > 0);
    layout.panel_widths_px.retain(|key, width| !key.trim().is_empty() && *width > 0);
    Ok(layout)
}
