import type { GitGraphCommit } from "@/entities/repository";

export type GitGraphNodeType = "head" | "merge" | "regular";

export type GitGraphSegmentType =
  | "vertical"
  | "vertical-top"
  | "vertical-bottom"
  | "branch-in"
  | "branch-out"
  | "merge-in"
  | "merge-out";

export type GitGraphSegment = {
  type: GitGraphSegmentType;
  fromLane: number;
  toLane: number;
  color: string;
};

export type GitGraphRow = {
  commitHash: string;
  lane: number;
  color: string;
  nodeType: GitGraphNodeType;
  isMainline: boolean;
  connections: GitGraphSegment[];
};

const GRAPH_COLORS = [
  "#2563eb",
  "#dc2626",
  "#16a34a",
  "#d97706",
  "#7c3aed",
  "#db2777",
  "#0891b2",
  "#65a30d",
  "#be123c",
  "#0f766e",
];

function hashString(value: string) {
  let hash = 0;

  for (let index = 0; index < value.length; index += 1) {
    hash = (hash << 5) - hash + value.charCodeAt(index);
    hash |= 0;
  }

  return Math.abs(hash);
}

function colorFor(value: string) {
  return GRAPH_COLORS[hashString(value) % GRAPH_COLORS.length];
}

export function computeGitGraphRows(commits: GitGraphCommit[]) {
  const rows = new Map<string, GitGraphRow>();

  if (commits.length === 0) {
    return rows;
  }

  const commitByHash = new Map(commits.map((commit) => [commit.hash, commit]));
  const commitIndex = new Map(commits.map((commit, index) => [commit.hash, index]));
  const mainline = new Set<string>();
  let current = commits[0];

  while (current) {
    mainline.add(current.hash);
    const firstParent = current.parents[0];
    const next = firstParent ? commitByHash.get(firstParent) : undefined;

    if (!next) {
      break;
    }

    current = next;
  }

  const commitLanes = new Map<string, number>();
  const commitColors = new Map<string, string>();
  const activeLanes = new Set<number>();
  const pendingParents = new Map<string, { lane: number; color: string }>();

  for (const commit of commits) {
    const pending = pendingParents.get(commit.hash);
    const isMainline = mainline.has(commit.hash);
    let lane = 0;
    let color = GRAPH_COLORS[0];

    if (pending) {
      lane = pending.lane;
      color = pending.color;
      pendingParents.delete(commit.hash);
    } else if (!isMainline) {
      lane = 1;
      while (activeLanes.has(lane)) {
        lane += 1;
      }
      color = colorFor(commit.hash);
    }

    commitLanes.set(commit.hash, lane);
    commitColors.set(commit.hash, color);
    activeLanes.add(lane);
    rows.set(commit.hash, {
      commitHash: commit.hash,
      lane,
      color,
      nodeType: commit.isHead ? "head" : commit.isMerge ? "merge" : "regular",
      isMainline,
      connections: [],
    });

    commit.parents.forEach((parentHash, parentIndex) => {
      if (commitLanes.has(parentHash) || pendingParents.has(parentHash)) {
        return;
      }

      if (parentIndex === 0) {
        pendingParents.set(parentHash, { lane, color });
        return;
      }

      let parentLane = mainline.has(parentHash) ? 0 : 1;
      while (parentLane !== 0 && (activeLanes.has(parentLane) || parentLane === lane)) {
        parentLane += 1;
      }
      pendingParents.set(parentHash, {
        lane: parentLane,
        color: colorFor(parentHash),
      });
    });

    const firstParent = commit.parents[0];
    const firstParentLane =
      firstParent !== undefined
        ? commitLanes.get(firstParent) ?? pendingParents.get(firstParent)?.lane
        : undefined;

    if (!firstParent || (firstParentLane !== undefined && firstParentLane !== lane)) {
      activeLanes.delete(lane);
    }
  }

  commits.forEach((commit, index) => {
    const row = rows.get(commit.hash);
    if (!row) {
      return;
    }

    commit.parents.forEach((parentHash, parentPosition) => {
      const parentLane = commitLanes.get(parentHash);
      const parentRowIndex = commitIndex.get(parentHash);

      if (parentLane === undefined) {
        return;
      }

      const color =
        parentPosition === 0
          ? row.color
          : (commitColors.get(parentHash) ?? colorFor(parentHash));
      const type =
        row.lane === parentLane
          ? "vertical-bottom"
          : parentPosition > 0
            ? row.lane < parentLane
              ? "merge-out"
              : "merge-in"
            : row.lane < parentLane
              ? "branch-out"
              : "branch-in";

      row.connections.push({
        type,
        fromLane: row.lane,
        toLane: parentLane,
        color,
      });

      if (parentRowIndex !== undefined && parentRowIndex > index + 1) {
        for (let rowIndex = index + 1; rowIndex < parentRowIndex; rowIndex += 1) {
          const intermediate = rows.get(commits[rowIndex].hash);
          intermediate?.connections.push({
            type: "vertical",
            fromLane: parentLane,
            toLane: parentLane,
            color,
          });
        }
      }

      const parentRow = parentHash ? rows.get(parentHash) : undefined;
      parentRow?.connections.push({
        type: "vertical-top",
        fromLane: parentLane,
        toLane: parentLane,
        color,
      });
    });
  });

  return rows;
}

export function getMaxGraphLane(rows: Map<string, GitGraphRow>) {
  let maxLane = 0;

  for (const row of rows.values()) {
    maxLane = Math.max(maxLane, row.lane);
    for (const connection of row.connections) {
      maxLane = Math.max(maxLane, connection.fromLane, connection.toLane);
    }
  }

  return maxLane;
}
