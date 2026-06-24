import { invoke } from "@tauri-apps/api/core";

import type {
  SavedPrompt,
  SavedPromptInput,
} from "@/entities/saved-prompt/model/types";

export async function listSavedPrompts() {
  return invoke<SavedPrompt[]>("list_saved_prompts");
}

export async function createSavedPrompt(input: SavedPromptInput) {
  return invoke<SavedPrompt>("create_saved_prompt", { input });
}

export async function updateSavedPrompt(id: string, input: SavedPromptInput) {
  return invoke<SavedPrompt>("update_saved_prompt", { id, input });
}

export async function deleteSavedPrompt(id: string) {
  return invoke<void>("delete_saved_prompt", { id });
}
