import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from "react";
import type { GitCommitGraph, GitGraphCommit, GitGraphRef, GitGraphRow } from "@yoophi/git-graph";

import { GraphCell } from "./graph-cell";
import { InfiniteLoadSentinel } from "./infinite-load-sentinel";
import { useVirtualRows } from "./use-virtual-rows";

/** 사용자가 드래그로 조정할 수 있는 고정 폭 컬럼. Commit(1fr)이 남는 폭을 흡수한다. */
type ResizableColumn = "author" | "date";
type ColumnWidths = Record<ResizableColumn, number>;

const DEFAULT_COLUMN_WIDTHS: ColumnWidths = { author: 144, date: 192 };
const COLUMN_MIN_WIDTH: ColumnWidths = { author: 80, date: 120 };
const COLUMN_MAX_WIDTH = 480;
const DEFAULT_COLUMN_WIDTH_STORAGE_KEY = "git-ui:history-graph:column-widths";

function clampColumnWidth(column: ResizableColumn, value: number): number {
  return Math.min(COLUMN_MAX_WIDTH, Math.max(COLUMN_MIN_WIDTH[column], Math.round(value)));
}

function readStoredColumnWidths(storageKey: string): ColumnWidths {
  if (typeof window === "undefined") {
    return DEFAULT_COLUMN_WIDTHS;
  }

  try {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      return DEFAULT_COLUMN_WIDTHS;
    }
    const parsed = JSON.parse(raw) as Partial<ColumnWidths>;
    return {
      author: clampColumnWidth("author", parsed.author ?? DEFAULT_COLUMN_WIDTHS.author),
      date: clampColumnWidth("date", parsed.date ?? DEFAULT_COLUMN_WIDTHS.date),
    };
  } catch {
    return DEFAULT_COLUMN_WIDTHS;
  }
}

/** 4개 컬럼(Graph auto / Commit 1fr / Author / Date)의 grid-template을 만든다. */
function buildGridTemplateColumns(columnWidths: ColumnWidths): string {
  return `auto minmax(0, 1fr) ${columnWidths.author}px ${columnWidths.date}px`;
}

export type HistoryGraphViewProps = {
  graph: GitCommitGraph;
  graphRefs: Map<string, GitGraphRef[]>;
  graphRows: Map<string, GitGraphRow>;
  maxGraphLane: number;
  selectedCommitHash?: string;
  onSelectCommit: (commitHash: string) => void;
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  onLoadMore: () => void;
  /** Author/Date 컬럼 폭을 저장할 localStorage 키(소비처별 분리용). */
  columnWidthStorageKey?: string;
};

/** 커밋 그래프(노드/연결선 + 메시지 + Author/Date)를 무한 스크롤로 렌더한다. */
export function HistoryGraphView({
  graph,
  graphRefs,
  graphRows,
  maxGraphLane,
  selectedCommitHash,
  onSelectCommit,
  hasNextPage,
  isFetchingNextPage,
  onLoadMore,
  columnWidthStorageKey = DEFAULT_COLUMN_WIDTH_STORAGE_KEY,
}: HistoryGraphViewProps) {
  const rowHeight = graph.layoutHints.rowHeight || 32;
  // 로드된 row 전체를 그리지 않고 viewport 근처만 렌더한다(AW specs/007 R11).
  const { containerRef, startIndex, endExclusive, totalHeight } = useVirtualRows({
    rowCount: graph.commits.length,
    rowHeight,
  });
  const visibleCommits = graph.commits.slice(startIndex, endExclusive);

  const [columnWidths, setColumnWidths] = useState<ColumnWidths>(() =>
    readStoredColumnWidths(columnWidthStorageKey),
  );
  const gridTemplateColumns = buildGridTemplateColumns(columnWidths);

  // 드래그 중 폭은 자주 갱신되므로 pointerup 때만 persist 한다.
  const persistColumnWidths = useCallback(
    (widths: ColumnWidths) => {
      if (typeof window === "undefined") {
        return;
      }
      try {
        window.localStorage.setItem(columnWidthStorageKey, JSON.stringify(widths));
      } catch {
        // localStorage 접근 실패(프라이빗 모드 등)는 무시한다.
      }
    },
    [columnWidthStorageKey],
  );

  const resizeStateRef = useRef<{
    column: ResizableColumn;
    startX: number;
    startWidth: number;
  } | null>(null);

  const handleResizePointerMove = useCallback((event: PointerEvent) => {
    const state = resizeStateRef.current;
    if (!state) {
      return;
    }
    const nextWidth = clampColumnWidth(state.column, state.startWidth + (event.clientX - state.startX));
    setColumnWidths((current) =>
      current[state.column] === nextWidth ? current : { ...current, [state.column]: nextWidth },
    );
  }, []);

  const endResize = useCallback(() => {
    if (!resizeStateRef.current) {
      return;
    }
    resizeStateRef.current = null;
    window.removeEventListener("pointermove", handleResizePointerMove);
    window.removeEventListener("pointerup", endResize);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    setColumnWidths((current) => {
      persistColumnWidths(current);
      return current;
    });
  }, [handleResizePointerMove, persistColumnWidths]);

  const startResize = useCallback(
    (column: ResizableColumn) => (event: ReactPointerEvent<HTMLSpanElement>) => {
      event.preventDefault();
      event.stopPropagation();
      resizeStateRef.current = {
        column,
        startX: event.clientX,
        startWidth: columnWidths[column],
      };
      window.addEventListener("pointermove", handleResizePointerMove);
      window.addEventListener("pointerup", endResize);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [columnWidths, endResize, handleResizePointerMove],
  );

  // 언마운트 시 남은 리스너/스타일 정리(드래그 중 언마운트 방어).
  useEffect(
    () => () => {
      window.removeEventListener("pointermove", handleResizePointerMove);
      window.removeEventListener("pointerup", endResize);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    },
    [endResize, handleResizePointerMove],
  );

  return (
    <div className="overflow-hidden rounded-md border">
      <div
        className="grid border-b bg-muted/40 px-2 py-2 text-xs font-medium text-muted-foreground"
        style={{ gridTemplateColumns }}
      >
        <span>Graph</span>
        <span>Commit</span>
        <span className="relative pr-2">
          Author
          <ColumnResizeHandle column="author" onPointerDown={startResize("author")} />
        </span>
        <span className="relative pr-2">
          Date
          <ColumnResizeHandle column="date" onPointerDown={startResize("date")} />
        </span>
      </div>
      <div className="relative" ref={containerRef} style={{ height: totalHeight }}>
        {visibleCommits.map((commit, index) => (
          <HistoryGraphRow
            commit={commit}
            graphRefs={graphRefs.get(commit.hash) ?? []}
            graphRow={graphRows.get(commit.hash)}
            gridTemplateColumns={gridTemplateColumns}
            isSelected={commit.hash === selectedCommitHash}
            key={commit.hash}
            maxGraphLane={maxGraphLane}
            onSelectCommit={onSelectCommit}
            rowHeight={rowHeight}
            top={(startIndex + index) * rowHeight}
          />
        ))}
      </div>
      <div className="border-t px-3 py-2 text-xs text-muted-foreground">
        <InfiniteLoadSentinel
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          onLoadMore={onLoadMore}
        />
        {graph.commits.length} / {graph.page.totalCount ?? graph.commits.length} commits loaded
        {hasNextPage ? " · more commits available" : ""}
        {isFetchingNextPage ? " · loading older commits" : ""}
      </div>
    </div>
  );
}

/** 컬럼 우측 경계에 놓이는 드래그 리사이즈 핸들. */
function ColumnResizeHandle({
  column,
  onPointerDown,
}: {
  column: ResizableColumn;
  onPointerDown: (event: ReactPointerEvent<HTMLSpanElement>) => void;
}) {
  return (
    <span
      role="separator"
      aria-orientation="vertical"
      aria-label={`${column === "author" ? "Author" : "Date"} 열 너비 조정`}
      className="absolute right-1 top-0 z-10 h-full w-2 cursor-col-resize touch-none select-none after:absolute after:right-0 after:top-0 after:h-full after:w-px after:bg-border after:transition-colors hover:after:bg-muted-foreground/60"
      onPointerDown={onPointerDown}
    />
  );
}

function HistoryGraphRow({
  commit,
  graphRefs,
  graphRow,
  gridTemplateColumns,
  isSelected,
  maxGraphLane,
  onSelectCommit,
  rowHeight,
  top,
}: {
  commit: GitGraphCommit;
  graphRefs: GitGraphRef[];
  graphRow?: GitGraphRow;
  gridTemplateColumns: string;
  isSelected: boolean;
  maxGraphLane: number;
  onSelectCommit: (commitHash: string) => void;
  rowHeight: number;
  top: number;
}) {
  const rowStyle: CSSProperties = { height: rowHeight, top, gridTemplateColumns };

  return (
    <button
      aria-label={`Commit ${commit.shortHash} by ${commit.author}: ${commit.message}`}
      className="absolute inset-x-0 grid w-full items-center border-b px-2 text-left text-sm hover:bg-muted/50 data-[selected=true]:bg-muted"
      data-selected={isSelected}
      onClick={() => onSelectCommit(commit.hash)}
      style={rowStyle}
      type="button"
    >
      <GraphCell maxLane={maxGraphLane} row={graphRow} rowHeight={rowHeight} />
      <span className="flex min-w-0 items-center gap-2 pr-2">
        <span className="font-mono text-xs text-muted-foreground">{commit.shortHash}</span>
        {graphRefs.map((ref) => (
          <span
            className="max-w-40 truncate rounded-sm border bg-background px-1.5 py-0.5 text-[10px] leading-none text-muted-foreground"
            key={`${ref.kind}:${ref.name}`}
            title={ref.name}
          >
            {ref.kind === "tag" ? "tag:" : ""}
            {ref.name}
          </span>
        ))}
        <span className="min-w-0 truncate">{commit.message}</span>
      </span>
      <span className="truncate pr-2 text-xs text-muted-foreground">{commit.author}</span>
      <span className="truncate font-mono text-xs text-muted-foreground">{commit.date}</span>
    </button>
  );
}
