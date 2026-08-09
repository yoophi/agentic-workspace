use tauri::{
    Manager, WebviewUrl, WebviewWindowBuilder,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
};
const ABOUT: &str = "about-markdown-annotator";
const SETTINGS: &str = "settings-markdown-annotator";
pub fn build_native_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let about = MenuItem::with_id(app, ABOUT, "Markdown Annotator 정보", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS, "설정…", true, Some("CmdOrCtrl+,"))?;
    Menu::with_items(
        app,
        &[
            &Submenu::with_items(
                app,
                "Markdown Annotator",
                true,
                &[
                    &about,
                    &PredefinedMenuItem::separator(app)?,
                    &settings,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?,
            &Submenu::with_items(
                app,
                "편집",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?,
        ],
    )
}
pub fn handle_menu_event<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event: tauri::menu::MenuEvent,
) {
    let (id, title) = if event.id().as_ref() == ABOUT {
        ("about", "Markdown Annotator 정보")
    } else if event.id().as_ref() == SETTINGS {
        ("settings", "Markdown Annotator 설정")
    } else {
        return;
    };
    if let Some(window) = app.get_webview_window(id) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    if let Ok(window) = WebviewWindowBuilder::new(
        app,
        id,
        WebviewUrl::App(format!("index.html?page={id}").into()),
    )
    .title(title)
    .inner_size(720.0, 640.0)
    .build()
    {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
