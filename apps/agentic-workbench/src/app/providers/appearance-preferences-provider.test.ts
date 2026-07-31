import { describe, expect, it, vi } from "vitest";

import {
  applyFontSizeStep,
  hydrateAppearancePreferences,
  installFontSizeShortcut,
} from "./appearance-preferences-provider";

describe("appearance preferences provider helpers", () => {
  it("applies the dataset before declaring hydration ready", async () => {
    const order: string[] = [];
    const target = { dataset: {} as { fontSizeStep?: string } };
    await hydrateAppearancePreferences({
      listen: async () => () => undefined,
      get: async () => ({ fontSizeStep: 2 }),
      apply: (preferences) => {
        applyFontSizeStep(target, preferences.fontSizeStep);
        order.push("apply");
      },
      ready: () => order.push("ready"),
    });
    expect(target.dataset.fontSizeStep).toBe("2");
    expect(order).toEqual(["apply", "ready"]);
  });

  it("keeps an event received during hydration ahead of the stale get result", async () => {
    const applied: number[] = [];
    let event: ((value: { fontSizeStep: 1 }) => void) | undefined;
    await hydrateAppearancePreferences({
      listen: async (onChange) => {
        event = onChange as typeof event;
        return () => undefined;
      },
      get: async () => {
        event?.({ fontSizeStep: 1 });
        return { fontSizeStep: 0 };
      },
      apply: (preferences) => applied.push(preferences.fontSizeStep),
      ready: vi.fn(),
    });
    expect(applied).toEqual([1]);
  });

  it("falls back to zero on get failure and exposes the error", async () => {
    const applied: number[] = [];
    const ready = vi.fn();
    await hydrateAppearancePreferences({
      listen: async () => () => undefined,
      get: async () => {
        throw new Error("offline");
      },
      apply: (preferences) => applied.push(preferences.fontSizeStep),
      ready,
    });
    expect(applied).toEqual([0]);
    expect(ready).toHaveBeenCalledWith(expect.stringContaining("offline"));
  });

  it("releases the initial render gate when event listening is unavailable", async () => {
    const applied: number[] = [];
    const ready = vi.fn();

    const cleanup = await hydrateAppearancePreferences({
      listen: async () => {
        throw new Error("event.listen not allowed on session window");
      },
      get: async () => ({ fontSizeStep: 1 }),
      apply: (preferences) => applied.push(preferences.fontSizeStep),
      ready,
    });

    expect(applied).toEqual([1]);
    expect(ready).toHaveBeenCalledWith(
      expect.stringContaining("event.listen not allowed"),
    );
    expect(cleanup).toBeTypeOf("function");
  });

  it("captures only recognized shortcuts and removes its listener", async () => {
    let handler: ((event: KeyboardEvent) => void) | undefined;
    const target = {
      addEventListener: vi.fn((_name, callback) => {
        handler = callback as (event: KeyboardEvent) => void;
      }),
      removeEventListener: vi.fn(),
    };
    const adjust = vi.fn().mockResolvedValue(undefined);
    const cleanup = installFontSizeShortcut(target, adjust, vi.fn());
    const preventDefault = vi.fn();

    handler?.({
      key: "+",
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      preventDefault,
      target: { tagName: "INPUT" },
    } as unknown as KeyboardEvent);
    handler?.({
      key: ",",
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      preventDefault,
    } as unknown as KeyboardEvent);
    cleanup();

    expect(adjust).toHaveBeenCalledOnce();
    expect(adjust).toHaveBeenCalledWith(1);
    expect(preventDefault).toHaveBeenCalledOnce();
    expect(target.removeEventListener).toHaveBeenCalledOnce();
  });
});
