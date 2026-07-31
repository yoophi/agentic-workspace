import type { FontSizeStep } from "./types";

export const FONT_SIZE_STEPS = [-2, -1, 0, 1, 2] as const;
export const DEFAULT_FONT_SIZE_STEP: FontSizeStep = 0;

export function normalizeFontSizeStep(value: number): FontSizeStep {
  return FONT_SIZE_STEPS.includes(value as FontSizeStep)
    ? (value as FontSizeStep)
    : DEFAULT_FONT_SIZE_STEP;
}

export function formatFontSizeStep(value: FontSizeStep): string {
  return value > 0 ? `+${value}` : String(value);
}
