mod application;
pub mod cli_launcher;
mod domain;
mod inbound;
mod infrastructure;
mod ports;

use inbound::file_browser_commands::{
    RootScanState, RootWatcherState, cancel_root_scan, read_root_markdown_document,
    resolve_launch_target, start_root_scan, start_root_watcher, stop_root_watcher,
};
use inbound::review_commands::{
    export_review_feedback, load_review_session, save_feedback_file, save_review_session,
};
use inbound::settings_commands::{
    load_preferences, reset_preferences, save_preferences, trash_review_data,
};
use inbound::tauri_commands::{
    DocumentWatcherState, check_cli_installed, focus_any_window, get_build_info, initial_cli_args,
    install_cli, load_recent_targets, open_about_window, open_document_from_cli_args,
    open_document_in_default_app, open_external_https, open_settings_window, open_welcome_window,
    read_markdown_file, remove_cli, request_open_document_tab, request_open_document_window,
    reveal_document_in_finder, start_markdown_document_watcher, stop_markdown_document_watcher,
    validated_document_path,
};
use tauri::{Manager, WindowEvent};

pub fn run() {
    let initial_cli_args = initial_cli_args()
        .inspect_err(|error| eprintln!("failed to read initial CLI arguments: {error}"))
        .ok()
        .flatten();

    tauri::Builder::default()
        .menu(inbound::native_menu::build_native_menu)
        .on_menu_event(inbound::native_menu::handle_menu_event)
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            let cwd = std::path::PathBuf::from(cwd);
            match open_document_from_cli_args(app, &argv, &cwd) {
                Ok(true) => {}
                Ok(false) => focus_any_window(app),
                Err(error) => {
                    eprintln!("failed to open document from CLI: {error}");
                    focus_any_window(app);
                }
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            match initial_cli_args.as_ref() {
                Some((argv, cwd)) => {
                    if let Err(error) = open_document_from_cli_args(app.handle(), argv, cwd) {
                        eprintln!("failed to open initial CLI document: {error}");
                        open_welcome_window(app.handle());
                    }
                }
                None => open_welcome_window(app.handle()),
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                let state = window.state::<DocumentWatcherState>();
                let _ = state.stop_for_window(window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![
            resolve_launch_target,
            read_root_markdown_document,
            start_root_scan,
            cancel_root_scan,
            start_root_watcher,
            stop_root_watcher,
            check_cli_installed,
            install_cli,
            remove_cli,
            read_markdown_file,
            get_build_info,
            open_settings_window,
            open_about_window,
            load_recent_targets,
            start_markdown_document_watcher,
            stop_markdown_document_watcher,
            request_open_document_tab,
            request_open_document_window,
            reveal_document_in_finder,
            open_document_in_default_app,
            validated_document_path,
            open_external_https,
            load_review_session,
            save_review_session,
            export_review_feedback,
            save_feedback_file,
            load_preferences,
            save_preferences,
            reset_preferences,
            trash_review_data
        ])
        .manage(DocumentWatcherState::new())
        .manage(RootScanState::new())
        .manage(RootWatcherState::new())
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_, _| {});
}
