import type { GitCommitGraph, GitGraphCommit, GitGraphRef, GitGraphRow } from "@yoophi/git-graph";

import { GraphCell } from "./graph-cell";
import { InfiniteLoadSentinel } from "./infinite-load-sentinel";

const GRID_COLUMNS = "grid-cols-[auto_minmax(0,1fr)_9rem_12rem]";

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
}: HistoryGraphViewProps) {
  const rowHeight = graph.layoutHints.rowHeight || 32;

  return (
    <div className="overflow-hidden rounded-md border">
      <div
        className={`grid ${GRID_COLUMNS} border-b bg-muted/40 px-2 py-2 text-xs font-medium text-muted-foreground`}
      >
        <span>Graph</span>
        <span>Commit</span>
        <span>Author</span>
        <span>Date</span>
      </div>
      <div>
        {graph.commits.map((commit) => (
          <HistoryGraphRow
            commit={commit}
            graphRefs={graphRefs.get(commit.hash) ?? []}
            graphRow={graphRows.get(commit.hash)}
            isSelected={commit.hash === selectedCommitHash}
            key={commit.hash}
            maxGraphLane={maxGraphLane}
            onSelectCommit={onSelectCommit}
            rowHeight={rowHeight}
          />
        ))}
      </div>
      <div className="border-t px-3 py-2 text-xs text-muted-foreground">
        <InfiniteLoadSentinel
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          onLoadMore={onLoadMore}
        />
        {graph.commits.length} / {graph.page.totalCount} commits loaded
        {hasNextPage ? " · more commits available" : ""}
        {isFetchingNextPage ? " · loading older commits" : ""}
      </div>
    </div>
  );
}

function HistoryGraphRow({
  commit,
  graphRefs,
  graphRow,
  isSelected,
  maxGraphLane,
  onSelectCommit,
  rowHeight,
}: {
  commit: GitGraphCommit;
  graphRefs: GitGraphRef[];
  graphRow?: GitGraphRow;
  isSelected: boolean;
  maxGraphLane: number;
  onSelectCommit: (commitHash: string) => void;
  rowHeight: number;
}) {
  return (
    <button
      aria-label={`Commit ${commit.shortHash} by ${commit.author}: ${commit.message}`}
      className={`grid w-full ${GRID_COLUMNS} items-center border-b px-2 text-left text-sm last:border-b-0 hover:bg-muted/50 data-[selected=true]:bg-muted`}
      data-selected={isSelected}
      onClick={() => onSelectCommit(commit.hash)}
      style={{ minHeight: rowHeight }}
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
