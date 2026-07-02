import type { MarkdownBlock, TocEntry, TocLevel } from "../types";
import { stripInlineMarkdown } from "./strip-inline-markdown";

function isTocLevel(level: number | undefined): level is TocLevel {
  return level === 1 || level === 2 || level === 3;
}

export function extractTocEntries(blocks: MarkdownBlock[]): TocEntry[] {
  const entries: TocEntry[] = [];

  for (const block of blocks) {
    if (block.type !== "heading" || !isTocLevel(block.level)) {
      continue;
    }

    entries.push({
      blockId: block.id,
      level: block.level,
      text: stripInlineMarkdown(block.content),
      startLine: block.startLine,
    });
  }

  return entries;
}
