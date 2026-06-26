#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri_git_explorer_lib::run()
}
