mod application;
pub mod cli_launcher;
mod domain;
mod inbound;
mod infrastructure;

use inbound::tauri_commands::{
    check_cli_installed, focus_any_window, initial_cli_args, install_cli,
    open_document_from_cli_args, open_welcome_window, read_markdown_file,
    request_open_document_tab, request_open_document_window,
};
use tauri::{Manager, RunEvent};

pub fn run() {
    let initial_cli_args = initial_cli_args()
        .inspect_err(|error| eprintln!("failed to read initial CLI arguments: {error}"))
        .ok()
        .flatten();

    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![
            check_cli_installed,
            install_cli,
            read_markdown_file,
            request_open_document_tab,
            request_open_document_window
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |app, event| {
            if let RunEvent::Ready = event
                && app.webview_windows().is_empty()
            {
                match initial_cli_args.as_ref() {
                    Some((argv, cwd)) => {
                        if let Err(error) = open_document_from_cli_args(app, argv, cwd) {
                            eprintln!("failed to open initial CLI document: {error}");
                            open_welcome_window(app);
                        }
                    }
                    None => open_welcome_window(app),
                }
            }
        });
}
