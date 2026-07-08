import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";

import { openSettingsWindow } from "./settings-window-repository";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue(undefined),
}));

describe("openSettingsWindow", () => {
  it("invokes the settings window command without payload", async () => {
    await openSettingsWindow();

    expect(invoke).toHaveBeenCalledWith("open_settings_window");
  });
});
