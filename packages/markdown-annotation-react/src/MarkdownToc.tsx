import type { TocEntry } from "@yoophi/markdown-annotation-core/types";
import { cn } from "./cn";

export type MarkdownTocProps = {
  entries: TocEntry[];
  onEntrySelect?: (entry: TocEntry) => void;
  className?: string;
};

export function MarkdownToc({ entries, onEntrySelect, className }: MarkdownTocProps) {
  if (entries.length === 0) {
    return null;
  }

  return (
    <nav aria-label="Table of contents" className={cn("markdown-toc text-sm", className)}>
      <ul className="flex flex-col gap-0.5">
        {entries.map((entry) => (
          <li key={entry.blockId}>
            <button
              className="w-full truncate rounded-md px-2 py-1 text-left text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              data-toc-block-id={entry.blockId}
              style={{ paddingLeft: `${0.5 + (entry.level - 1) * 0.75}rem` }}
              title={entry.text}
              type="button"
              onClick={() => onEntrySelect?.(entry)}
            >
              {entry.text}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
