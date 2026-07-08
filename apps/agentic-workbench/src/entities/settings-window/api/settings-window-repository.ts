import { invoke } from "@tauri-apps/api/core";

export async function openSettingsWindow(): Promise<void> {
  return invoke<void>("open_settings_window");
}
