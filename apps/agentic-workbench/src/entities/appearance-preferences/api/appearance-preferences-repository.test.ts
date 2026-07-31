import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import {
  APPEARANCE_PREFERENCES_CHANGED_EVENT,
  adjustFontSizeStep,
  getAppearancePreferences,
  listenAppearancePreferences,
  setFontSizeStep,
} from "./appearance-preferences-repository";

describe("appearance preferences repository", () => {
  beforeEach(() => {
    invoke.mockReset();
    listen.mockReset();
  });

  it("maps all commands to exact invoke payloads", async () => {
    invoke.mockResolvedValue({ fontSizeStep: 0 });
    await getAppearancePreferences();
    await setFontSizeStep(2);
    await adjustFontSizeStep(-1);
    expect(invoke.mock.calls).toEqual([
      ["get_appearance_preferences"],
      ["set_font_size_step", { fontSizeStep: 2 }],
      ["adjust_font_size_step", { delta: -1 }],
    ]);
  });

  it("forwards event payloads and returns the unlisten cleanup", async () => {
    const unlisten = vi.fn();
    listen.mockImplementation(async (_name, callback) => {
      callback({ payload: { fontSizeStep: 1 } });
      return unlisten;
    });
    const onChange = vi.fn();

    const cleanup = await listenAppearancePreferences(onChange);
    cleanup();

    expect(listen).toHaveBeenCalledWith(
      APPEARANCE_PREFERENCES_CHANGED_EVENT,
      expect.any(Function),
    );
    expect(onChange).toHaveBeenCalledWith({ fontSizeStep: 1 });
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
