import type { TocEntry } from "@yoophi/markdown-annotation-core/types";
import { Circle, CircleCheck } from "lucide-react";
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
              className="flex w-full min-w-0 items-center gap-2 rounded-md px-2 py-1 text-left text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              data-toc-block-id={entry.blockId}
              style={{ paddingLeft: `${0.5 + (entry.level - 1) * 0.75}rem` }}
              title={entry.text}
              type="button"
              onClick={() => onEntrySelect?.(entry)}
            >
              <span className="min-w-0 flex-1 truncate">{entry.text}</span>
              {entry.taskSummary ? (
                <span
                  aria-label={`${entry.taskSummary.completed} completed tasks, ${entry.taskSummary.open} open tasks`}
                  className="flex shrink-0 items-center gap-1.5 text-xs tabular-nums"
                  data-toc-task-summary={entry.blockId}
                >
                  <span className="flex items-center gap-0.5 text-primary">
                    <CircleCheck aria-hidden="true" className="size-3" />
                    {entry.taskSummary.completed}
                  </span>
                  <span className="flex items-center gap-0.5 text-muted-foreground">
                    <Circle aria-hidden="true" className="size-3" />
                    {entry.taskSummary.open}
                  </span>
                </span>
              ) : null}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
