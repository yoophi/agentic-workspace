import type { FileBrowserRow } from "@yoophi/file-browser-core";
import type { KeyboardEvent } from "react";

export function nextTreeIndex(
  event: Pick<KeyboardEvent, "key">,
  rows: readonly FileBrowserRow[],
  currentIndex: number,
) {
  if (rows.length === 0) return -1;
  switch (event.key) {
    case "ArrowDown": return Math.min(currentIndex + 1, rows.length - 1);
    case "ArrowUp": return Math.max(currentIndex - 1, 0);
    case "Home": return 0;
    case "End": return rows.length - 1;
    default: return currentIndex;
  }
}

export function parentRowIndex(rows: readonly FileBrowserRow[], currentIndex: number) {
  const current = rows[currentIndex];
  if (!current || current.depth === 0) return currentIndex;
  for (let index = currentIndex - 1; index >= 0; index -= 1) {
    if (rows[index]?.depth === current.depth - 1) return index;
  }
  return currentIndex;
}
