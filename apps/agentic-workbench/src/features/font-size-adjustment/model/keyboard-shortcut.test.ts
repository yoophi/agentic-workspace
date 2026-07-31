import { describe, expect, it } from "vitest";

import { fontSizeAdjustmentForShortcut } from "./keyboard-shortcut";

function key(
  value: string,
  modifiers: Partial<{ metaKey: boolean; ctrlKey: boolean; altKey: boolean }> = {},
) {
  return {
    key: value,
    metaKey: false,
    ctrlKey: false,
    altKey: false,
    ...modifiers,
  };
}

describe("fontSizeAdjustmentForShortcut", () => {
  it("maps logical Meta plus, equals, and minus keys", () => {
    expect(fontSizeAdjustmentForShortcut(key("+", { metaKey: true }))).toBe(1);
    expect(fontSizeAdjustmentForShortcut(key("=", { metaKey: true }))).toBe(1);
    expect(fontSizeAdjustmentForShortcut(key("-", { metaKey: true }))).toBe(-1);
  });

  it("does not intercept Preferences or incompatible modifiers and keys", () => {
    expect(fontSizeAdjustmentForShortcut(key(",", { metaKey: true }))).toBeNull();
    expect(fontSizeAdjustmentForShortcut(key("+", { ctrlKey: true }))).toBeNull();
    expect(
      fontSizeAdjustmentForShortcut(key("+", { metaKey: true, altKey: true })),
    ).toBeNull();
    expect(fontSizeAdjustmentForShortcut(key("a", { metaKey: true }))).toBeNull();
  });
});
