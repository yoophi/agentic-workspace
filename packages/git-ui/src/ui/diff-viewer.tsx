import { useMemo } from "react";

import { cn } from "../lib/cn";
import { diffLineClassName } from "../lib/styling";
import { parseDiffLines } from "../model/diff";

/** Unified diff를 줄별 색상 + old/new 라인 번호로 렌더한다. */
export function DiffViewer({
  content,
  className,
}: {
  content: string;
  className?: string;
}) {
  const lines = useMemo(() => parseDiffLines(content), [content]);

  return (
    <pre
      className={cn(
        "max-h-96 overflow-auto rounded-md border bg-background font-mono text-xs leading-5",
        className,
      )}
    >
      {lines.map((line, index) => (
        <div
          className={cn(
            "grid min-w-max grid-cols-[3.5rem_3.5rem_minmax(0,1fr)] whitespace-pre",
            diffLineClassName(line.content),
          )}
          key={`${index}:${line.content}`}
        >
          <span className="select-none border-r px-2 text-right text-muted-foreground/70">
            {line.oldLineNumber ?? ""}
          </span>
          <span className="select-none border-r px-2 text-right text-muted-foreground/70">
            {line.newLineNumber ?? ""}
          </span>
          <span className="px-3">{line.content || " "}</span>
        </div>
      ))}
    </pre>
  );
}
