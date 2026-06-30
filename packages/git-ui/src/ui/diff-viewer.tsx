import { cn } from "../lib/cn";
import { diffLineClassName } from "../lib/styling";
import { parseDiffLines } from "../model/diff";

/**
 * Unified diff를 줄별 색상으로 렌더한다.
 * showLineNumbers=true이면 old/new 라인 번호 컬럼을 함께 표시한다.
 */
export function DiffViewer({
  content,
  showLineNumbers = true,
  className,
}: {
  content: string;
  showLineNumbers?: boolean;
  className?: string;
}) {
  const lines = parseDiffLines(content);

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
            "grid min-w-max whitespace-pre",
            showLineNumbers
              ? "grid-cols-[3.5rem_3.5rem_minmax(0,1fr)]"
              : "grid-cols-[minmax(0,1fr)]",
            diffLineClassName(line.content),
          )}
          key={`${index}:${line.content}`}
        >
          {showLineNumbers ? (
            <>
              <span className="select-none border-r px-2 text-right text-muted-foreground/70">
                {line.oldLineNumber ?? ""}
              </span>
              <span className="select-none border-r px-2 text-right text-muted-foreground/70">
                {line.newLineNumber ?? ""}
              </span>
            </>
          ) : null}
          <span className="px-3">{line.content || " "}</span>
        </div>
      ))}
    </pre>
  );
}
