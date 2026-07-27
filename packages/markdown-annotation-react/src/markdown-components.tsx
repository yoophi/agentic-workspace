import type { ReactNode } from "react";

/**
 * Markdown 목록의 유효한 DOM 경계를 한 곳에서 유지한다. 주석 toolbar는 각
 * `li` 내부에 배치하므로 `ul`/`ol`의 직접 자식은 언제나 `li`다.
 */
export function MarkdownList({
  children,
  ordered,
  start,
}: {
  children: ReactNode;
  ordered?: boolean;
  start?: number;
}) {
  return ordered ? <ol start={start}>{children}</ol> : <ul>{children}</ul>;
}
