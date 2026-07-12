import type { MarkdownBlock, TocEntry, TocLevel } from "../types";
import { stripInlineMarkdown } from "./strip-inline-markdown";

function isTocLevel(level: number | undefined): level is TocLevel {
  return level === 1 || level === 2 || level === 3;
}

export function extractTocEntries(blocks: MarkdownBlock[]): TocEntry[] {
  const entries: TocEntry[] = [];
  let activeH1Entry: TocEntry | undefined;

  for (const block of blocks) {
    if (block.type === "heading" && isTocLevel(block.level)) {
      const entry: TocEntry = {
        blockId: block.id,
        level: block.level,
        text: stripInlineMarkdown(block.content),
        startLine: block.startLine,
      };

      entries.push(entry);

      if (block.level === 1) {
        activeH1Entry = entry;
      }

      continue;
    }

    if (block.type !== "list-item" || block.checked === undefined || !activeH1Entry) {
      continue;
    }

    const taskSummary = activeH1Entry.taskSummary ?? { completed: 0, open: 0 };

    if (block.checked) {
      taskSummary.completed += 1;
    } else {
      taskSummary.open += 1;
    }

    activeH1Entry.taskSummary = taskSummary;
  }

  return entries;
}
