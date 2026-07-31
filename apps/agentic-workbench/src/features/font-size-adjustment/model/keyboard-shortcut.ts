import type { FontSizeAdjustment } from "@/entities/appearance-preferences/model/types";

export type FontSizeShortcutEvent = Pick<
  KeyboardEvent,
  "key" | "metaKey" | "ctrlKey" | "altKey"
>;

export function fontSizeAdjustmentForShortcut(
  event: FontSizeShortcutEvent,
): FontSizeAdjustment | null {
  if (!event.metaKey || event.ctrlKey || event.altKey) {
    return null;
  }
  if (event.key === "+" || event.key === "=") {
    return 1;
  }
  if (event.key === "-") {
    return -1;
  }
  return null;
}
