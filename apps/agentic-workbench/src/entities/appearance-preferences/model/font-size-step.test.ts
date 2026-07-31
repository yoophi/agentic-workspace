import { describe, expect, it } from "vitest";

import {
  FONT_SIZE_STEPS,
  formatFontSizeStep,
  normalizeFontSizeStep,
} from "./font-size-step";

describe("font size step", () => {
  it("accepts exactly five values and normalizes everything else to zero", () => {
    expect(FONT_SIZE_STEPS.map(normalizeFontSizeStep)).toEqual(FONT_SIZE_STEPS);
    expect(normalizeFontSizeStep(-3)).toBe(0);
    expect(normalizeFontSizeStep(3)).toBe(0);
    expect(normalizeFontSizeStep(1.5)).toBe(0);
  });

  it("formats signed labels", () => {
    expect(FONT_SIZE_STEPS.map(formatFontSizeStep)).toEqual([
      "-2",
      "-1",
      "0",
      "+1",
      "+2",
    ]);
  });
});
