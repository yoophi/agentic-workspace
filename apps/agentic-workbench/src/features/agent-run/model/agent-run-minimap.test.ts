import { describe, expect, it } from "vitest";

import {
  applyPendingSeek,
  clampUnit,
  createPendingSeek,
  createViewportIndicator,
  isMinimapAlignmentWithinTolerance,
  keyboardSeekRatio,
  pointerSeekRatio,
  scrollTopForTimelineRatio,
  type TimelineLayoutSnapshot,
} from "./agent-run-minimap";

const snapshot: TimelineLayoutSnapshot = {
  timelineOffsetInScroller: 180,
  totalHeight: 1_000,
  visibleStart: 250,
  visibleEnd: 450,
  viewportHeight: 200,
  itemLayouts: [
    { id: "u1", start: 0, end: 100, height: 100, measured: true },
    { id: "a1", start: 112, end: 300, height: 188, measured: false },
  ],
  revision: 3,
};

describe("agent run minimap geometry", () => {
  it("clamps arbitrary values into the unit interval", () => {
    expect(clampUnit(-1)).toBe(0);
    expect(clampUnit(0.4)).toBe(0.4);
    expect(clampUnit(2)).toBe(1);
  });

  it("derives viewport start and size from timeline-local coordinates", () => {
    expect(createViewportIndicator(snapshot)).toMatchObject({
      startRatio: 0.3125,
      sizeRatio: 0.2,
      disabled: false,
    });
  });

  it("uses a full disabled indicator when content fits", () => {
    expect(
      createViewportIndicator({
        ...snapshot,
        totalHeight: 160,
        visibleStart: 0,
        visibleEnd: 160,
        viewportHeight: 200,
      }),
    ).toEqual({ startRatio: 0, sizeRatio: 1, visualSizeRatio: 1, disabled: true });
  });

  it("maps pointer positions with grab offset and clamps both ends", () => {
    expect(
      pointerSeekRatio({
        pointerY: -20,
        trackTop: 100,
        trackHeight: 400,
        indicatorHeight: 80,
        grabOffset: 20,
      }),
    ).toBe(0);
    expect(
      pointerSeekRatio({
        pointerY: 600,
        trackTop: 100,
        trackHeight: 400,
        indicatorHeight: 80,
        grabOffset: 20,
      }),
    ).toBe(1);
  });

  it("converts a timeline ratio through its parent scroller offset", () => {
    expect(scrollTopForTimelineRatio(snapshot, 0.5, 900)).toBe(580);
    expect(scrollTopForTimelineRatio(snapshot, 1, 700)).toBe(700);
  });

  it("handles arrow, page, home, and end keyboard seeks", () => {
    expect(keyboardSeekRatio({ key: "ArrowUp", currentRatio: 0.5, snapshot })).toBe(0.45);
    expect(keyboardSeekRatio({ key: "ArrowDown", currentRatio: 0.5, snapshot })).toBe(0.55);
    expect(keyboardSeekRatio({ key: "PageUp", currentRatio: 0.5, snapshot })).toBe(0.25);
    expect(keyboardSeekRatio({ key: "PageDown", currentRatio: 0.5, snapshot })).toBe(0.75);
    expect(keyboardSeekRatio({ key: "Home", currentRatio: 0.5, snapshot })).toBe(0);
    expect(keyboardSeekRatio({ key: "End", currentRatio: 0.5, snapshot })).toBe(1);
    expect(keyboardSeekRatio({ key: "Enter", currentRatio: 0.5, snapshot })).toBeNull();
  });

  it("applies a pending seek once a newer All-filter layout is available", () => {
    const pending = createPendingSeek(0.7, snapshot.revision);
    expect(applyPendingSeek(pending, "tool_call/tool_result", snapshot)).toEqual({
      pending,
      targetRatio: null,
    });
    expect(applyPendingSeek(pending, "all", { ...snapshot, revision: 4 })).toEqual({
      pending: null,
      targetRatio: 0.7,
    });
  });

  it("tracks direct-scroll and measured revisions within the five-percent target", () => {
    const directScroll = createViewportIndicator({
      ...snapshot,
      visibleStart: 400,
      visibleEnd: 600,
      revision: 8,
      itemLayouts: snapshot.itemLayouts.map((item) => ({ ...item, measured: true })),
    });
    expect(directScroll.startRatio).toBe(0.5);
    expect(isMinimapAlignmentWithinTolerance(directScroll.startRatio, 0.54)).toBe(true);
    expect(isMinimapAlignmentWithinTolerance(directScroll.startRatio, 0.56)).toBe(false);
  });

  it("preserves a past ratio and reaches the end as streaming content grows", () => {
    const pastBefore = createViewportIndicator(snapshot);
    const pastAfter = createViewportIndicator({ ...snapshot, totalHeight: 1_400, revision: 4 });
    const endAfter = createViewportIndicator({
      ...snapshot,
      totalHeight: 1_400,
      visibleStart: 1_200,
      visibleEnd: 1_400,
      revision: 5,
    });

    expect(pastBefore.startRatio).toBeGreaterThan(0);
    expect(pastAfter.startRatio).toBeLessThan(pastBefore.startRatio);
    expect(endAfter.startRatio).toBe(1);
  });

  it("computes 500 viewport revisions within the interaction budget", () => {
    const startedAt = performance.now();
    const indicators = Array.from({ length: 500 }, (_, index) =>
      createViewportIndicator({
        ...snapshot,
        totalHeight: 50_000,
        visibleStart: index * 80,
        visibleEnd: index * 80 + 200,
        revision: index,
      }),
    );
    const elapsed = performance.now() - startedAt;

    expect(indicators).toHaveLength(500);
    expect(indicators.every((indicator) => indicator.startRatio >= 0)).toBe(true);
    expect(elapsed).toBeLessThan(100);
  });
});
