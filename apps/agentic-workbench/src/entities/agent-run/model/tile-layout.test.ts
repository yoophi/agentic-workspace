import { describe, expect, it } from "vitest";

import {
  calculateTileLayoutFrames,
  createEqualHorizontalTileLayout,
  createTileLeaf,
  flattenTilePanelIds,
  getTileDepth,
  removeTileLeaf,
  resizeTileSplit,
  splitTileLeaf,
  validateTileLayout,
} from "./tile-layout";

describe("tile layout", () => {
  it("creates equal horizontal frames for every panel", () => {
    let sequence = 1;
    const layout = createEqualHorizontalTileLayout(
      ["main", "extra-1", "extra-2"],
      () => `split-${sequence++}`,
    );
    const frames = calculateTileLayoutFrames(layout).leaves;

    expect(frames.main.left).toBeCloseTo(0);
    expect(frames.main.width).toBeCloseTo(1 / 3);
    expect(frames["extra-1"].left).toBeCloseTo(1 / 3);
    expect(frames["extra-1"].width).toBeCloseTo(1 / 3);
    expect(frames["extra-2"].left).toBeCloseTo(2 / 3);
    expect(frames["extra-2"].width).toBeCloseTo(1 / 3);

    const maximumLayout = createEqualHorizontalTileLayout(
      Array.from({ length: 8 }, (_, index) => `panel-${index}`),
      () => `split-${sequence++}`,
    );
    expect(getTileDepth(maximumLayout)).toBe(4);
  });

  it("splits a target leaf to the right and below", () => {
    const right = splitTileLeaf(createTileLeaf("main"), "main", "extra-1", "right", "split-1");
    expect(right.changed).toBe(true);
    expect(right.layout).toEqual({
      type: "split",
      id: "split-1",
      orientation: "horizontal",
      ratio: 0.5,
      first: { type: "leaf", panelId: "main" },
      second: { type: "leaf", panelId: "extra-1" },
    });

    const below = splitTileLeaf(right.layout, "extra-1", "extra-2", "below", "split-2");
    expect(flattenTilePanelIds(below.layout)).toEqual(["main", "extra-1", "extra-2"]);
    expect(getTileDepth(below.layout)).toBe(3);
    expect(validateTileLayout(below.layout, ["main", "extra-1", "extra-2"])).toEqual([]);
  });

  it("collapses the parent split when a leaf is removed", () => {
    const split = splitTileLeaf(
      createTileLeaf("main"),
      "main",
      "extra-1",
      "right",
      "split-1",
    ).layout;
    const removed = removeTileLeaf(split, "extra-1");

    expect(removed.changed).toBe(true);
    expect(removed.layout).toEqual({ type: "leaf", panelId: "main" });
    expect(removed.focusFallbackPanelId).toBe("main");
  });

  it("projects nested tree leaves into stable normalized frames", () => {
    const right = splitTileLeaf(createTileLeaf("main"), "main", "extra-1", "right", "s1");
    const below = splitTileLeaf(right.layout, "extra-1", "extra-2", "below", "s2");
    const frames = calculateTileLayoutFrames(below.layout);

    expect(frames.leaves.main).toEqual({ left: 0, top: 0, width: 0.5, height: 1 });
    expect(frames.leaves["extra-1"]).toEqual({
      left: 0.5,
      top: 0,
      width: 0.5,
      height: 0.5,
    });
    expect(frames.leaves["extra-2"]).toEqual({
      left: 0.5,
      top: 0.5,
      width: 0.5,
      height: 0.5,
    });
  });

  it("clamps split ratios and leaves unknown ids unchanged", () => {
    const split = splitTileLeaf(
      createTileLeaf("main"),
      "main",
      "extra-1",
      "right",
      "split-1",
    ).layout;
    expect(resizeTileSplit(split, "split-1", 0.99)).toEqual({
      ...split,
      ratio: 0.85,
    });
    expect(resizeTileSplit(split, "missing", 0.2)).toBe(split);
  });

  it("reports duplicate, missing and unexpected leaves", () => {
    const invalid = {
      type: "split" as const,
      id: "split-1",
      orientation: "horizontal" as const,
      ratio: 0.5,
      first: createTileLeaf("main"),
      second: createTileLeaf("main"),
    };

    expect(validateTileLayout(invalid, ["main", "extra-1"])).toEqual(
      expect.arrayContaining([
        "duplicate panel leaf: main",
        "missing panel leaf: extra-1",
      ]),
    );
  });
});
