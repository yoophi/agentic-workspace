import type { GitGraphRow } from "@yoophi/git-graph";

import { graphSegmentPath, laneX } from "../model/graph-render";

/** 커밋 한 행의 graph 노드/연결선을 그리는 SVG 셀. */
export function GraphCell({
  maxLane,
  row,
  rowHeight,
}: {
  maxLane: number;
  row?: GitGraphRow;
  rowHeight: number;
}) {
  const width = 20 + (maxLane + 1) * 20;
  const nodeX = row ? laneX(row.lane) : 10;
  const centerY = rowHeight / 2;

  return (
    <svg aria-hidden className="block shrink-0" height={rowHeight} width={width}>
      {row?.connections.map((segment, index) => (
        <path
          d={graphSegmentPath(segment, rowHeight)}
          fill="none"
          key={`${segment.type}:${segment.fromLane}:${segment.toLane}:${index}`}
          stroke={segment.color}
          strokeDasharray={segment.type.startsWith("merge") ? "4 3" : undefined}
          strokeWidth="2"
        />
      ))}
      {row ? (
        row.nodeType === "head" ? (
          <>
            <circle cx={nodeX} cy={centerY} fill="none" r="6" stroke="currentColor" strokeWidth="2" />
            <circle cx={nodeX} cy={centerY} fill={row.color} r="4" />
          </>
        ) : row.nodeType === "merge" ? (
          <>
            <circle cx={nodeX} cy={centerY} fill="none" r="5" stroke={row.color} strokeWidth="1.5" />
            <circle cx={nodeX} cy={centerY} fill={row.color} r="3" />
          </>
        ) : (
          <circle cx={nodeX} cy={centerY} fill={row.color} r="4" />
        )
      ) : null}
    </svg>
  );
}
