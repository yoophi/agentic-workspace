import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppearancePreferences,
  FontSizeAdjustment,
  FontSizeStep,
} from "../model/types";

export const APPEARANCE_PREFERENCES_CHANGED_EVENT =
  "app://appearance-preferences-changed";

export function getAppearancePreferences() {
  return invoke<AppearancePreferences>("get_appearance_preferences");
}

export function setFontSizeStep(fontSizeStep: FontSizeStep) {
  return invoke<AppearancePreferences>("set_font_size_step", { fontSizeStep });
}

export function adjustFontSizeStep(delta: FontSizeAdjustment) {
  return invoke<AppearancePreferences>("adjust_font_size_step", { delta });
}

export function listenAppearancePreferences(
  onChange: (preferences: AppearancePreferences) => void,
): Promise<UnlistenFn> {
  return listen<AppearancePreferences>(
    APPEARANCE_PREFERENCES_CHANGED_EVENT,
    (event) => onChange(event.payload),
  );
}
