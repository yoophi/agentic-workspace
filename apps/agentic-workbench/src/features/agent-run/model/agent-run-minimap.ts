import type { EventGroup } from "@/entities/agent-run/model";

export type TimelineItemLayout = {
  id: string;
  start: number;
  end: number;
  height: number;
  measured: boolean;
};

export type TimelineLayoutSnapshot = {
  timelineOffsetInScroller: number;
  totalHeight: number;
  visibleStart: number;
  visibleEnd: number;
  viewportHeight: number;
  itemLayouts: TimelineItemLayout[];
  revision: number;
};

export type ViewportIndicator = {
  startRatio: number;
  sizeRatio: number;
  visualSizeRatio: number;
  disabled: boolean;
};

export type MinimapSeekInput = "pointer" | "keyboard";

export type PendingSeek = {
  targetRatio: number;
  requestedRevision: number;
  status: "waitingForAll";
};

export const EMPTY_TIMELINE_LAYOUT_SNAPSHOT: TimelineLayoutSnapshot = {
  timelineOffsetInScroller: 0,
  totalHeight: 0,
  visibleStart: 0,
  visibleEnd: 0,
  viewportHeight: 0,
  itemLayouts: [],
  revision: 0,
};

const MIN_VISUAL_INDICATOR_RATIO = 0.08;
const ARROW_STEP_RATIO = 0.05;

export function clampUnit(value: number) {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.min(1, Math.max(0, value));
}

export function createViewportIndicator(snapshot: TimelineLayoutSnapshot): ViewportIndicator {
  if (
    snapshot.totalHeight <= 0 ||
    snapshot.viewportHeight <= 0 ||
    snapshot.totalHeight <= snapshot.viewportHeight
  ) {
    return { startRatio: 0, sizeRatio: 1, visualSizeRatio: 1, disabled: true };
  }

  const scrollableHeight = snapshot.totalHeight - snapshot.viewportHeight;
  const startRatio = clampUnit(snapshot.visibleStart / scrollableHeight);
  const sizeRatio = clampUnit(snapshot.viewportHeight / snapshot.totalHeight);
  return {
    startRatio,
    sizeRatio,
    visualSizeRatio: Math.max(MIN_VISUAL_INDICATOR_RATIO, sizeRatio),
    disabled: false,
  };
}

export function pointerSeekRatio({
  pointerY,
  trackTop,
  trackHeight,
  indicatorHeight,
  grabOffset,
}: {
  pointerY: number;
  trackTop: number;
  trackHeight: number;
  indicatorHeight: number;
  grabOffset: number;
}) {
  const availableDistance = Math.max(0, trackHeight - indicatorHeight);
  if (availableDistance === 0) {
    return 0;
  }
  return clampUnit((pointerY - trackTop - grabOffset) / availableDistance);
}

export function scrollTopForTimelineRatio(
  snapshot: TimelineLayoutSnapshot,
  targetRatio: number,
  maxScrollerScrollTop: number,
) {
  const maxTimelineStart = Math.max(0, snapshot.totalHeight - snapshot.viewportHeight);
  const targetTimelineStart = clampUnit(targetRatio) * maxTimelineStart;
  return Math.min(
    Math.max(0, maxScrollerScrollTop),
    Math.max(0, snapshot.timelineOffsetInScroller + targetTimelineStart),
  );
}

export function keyboardSeekRatio({
  key,
  currentRatio,
  snapshot,
}: {
  key: string;
  currentRatio: number;
  snapshot: TimelineLayoutSnapshot;
}) {
  const pageStep = clampUnit(
    snapshot.viewportHeight / Math.max(1, snapshot.totalHeight - snapshot.viewportHeight),
  );
  if (key === "Home") return 0;
  if (key === "End") return 1;
  if (key === "ArrowUp") return clampUnit(currentRatio - ARROW_STEP_RATIO);
  if (key === "ArrowDown") return clampUnit(currentRatio + ARROW_STEP_RATIO);
  if (key === "PageUp") return clampUnit(currentRatio - pageStep);
  if (key === "PageDown") return clampUnit(currentRatio + pageStep);
  return null;
}

export function createPendingSeek(targetRatio: number, requestedRevision: number): PendingSeek {
  return {
    targetRatio: clampUnit(targetRatio),
    requestedRevision,
    status: "waitingForAll",
  };
}

export function applyPendingSeek(
  pending: PendingSeek | null,
  filter: EventGroup | "all",
  snapshot: TimelineLayoutSnapshot,
): { pending: PendingSeek | null; targetRatio: number | null } {
  if (!pending || filter !== "all" || snapshot.revision <= pending.requestedRevision) {
    return { pending, targetRatio: null };
  }
  return { pending: null, targetRatio: pending.targetRatio };
}

export function isMinimapAlignmentWithinTolerance(
  actualRatio: number,
  expectedRatio: number,
  tolerance = 0.05,
) {
  return Math.abs(clampUnit(actualRatio) - clampUnit(expectedRatio)) <= tolerance;
}
