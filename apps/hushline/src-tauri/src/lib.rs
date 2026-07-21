mod adapters;
mod application;
mod domain;
mod ports;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    adapters::tauri::run();
}
