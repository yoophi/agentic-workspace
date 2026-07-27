export type TilePlacement = "right" | "below";
export type TileOrientation = "horizontal" | "vertical";

export type TileLeaf = {
  type: "leaf";
  panelId: string;
};

export type TileSplit = {
  type: "split";
  id: string;
  orientation: TileOrientation;
  ratio: number;
  first: TileLayoutNode;
  second: TileLayoutNode;
};

export type TileLayoutNode = TileLeaf | TileSplit;

export type TileBounds = {
  left: number;
  top: number;
  width: number;
  height: number;
};

export type TileLayoutFrames = {
  leaves: Record<string, TileBounds>;
  splits: Array<TileBounds & Pick<TileSplit, "id" | "orientation" | "ratio">>;
};

export function createTileLeaf(panelId: string): TileLeaf {
  return { type: "leaf", panelId };
}

export function createEqualHorizontalTileLayout(
  panelIds: string[],
  createSplitId: () => string,
): TileLayoutNode {
  if (panelIds.length === 0) {
    throw new Error("At least one panel is required to create a tile layout.");
  }
  if (panelIds.length === 1) {
    return createTileLeaf(panelIds[0]);
  }

  const firstCount = Math.floor(panelIds.length / 2);
  return {
    type: "split",
    id: createSplitId(),
    orientation: "horizontal",
    ratio: firstCount / panelIds.length,
    first: createEqualHorizontalTileLayout(
      panelIds.slice(0, firstCount),
      createSplitId,
    ),
    second: createEqualHorizontalTileLayout(
      panelIds.slice(firstCount),
      createSplitId,
    ),
  };
}

export function flattenTilePanelIds(layout: TileLayoutNode): string[] {
  return layout.type === "leaf"
    ? [layout.panelId]
    : [...flattenTilePanelIds(layout.first), ...flattenTilePanelIds(layout.second)];
}

export function getTileDepth(layout: TileLayoutNode): number {
  return layout.type === "leaf"
    ? 1
    : 1 + Math.max(getTileDepth(layout.first), getTileDepth(layout.second));
}

export function calculateTileLayoutFrames(
  layout: TileLayoutNode,
  bounds: TileBounds = { left: 0, top: 0, width: 1, height: 1 },
): TileLayoutFrames {
  if (layout.type === "leaf") {
    return { leaves: { [layout.panelId]: bounds }, splits: [] };
  }

  const firstBounds =
    layout.orientation === "horizontal"
      ? { ...bounds, width: bounds.width * layout.ratio }
      : { ...bounds, height: bounds.height * layout.ratio };
  const secondBounds =
    layout.orientation === "horizontal"
      ? {
          ...bounds,
          left: bounds.left + bounds.width * layout.ratio,
          width: bounds.width * (1 - layout.ratio),
        }
      : {
          ...bounds,
          top: bounds.top + bounds.height * layout.ratio,
          height: bounds.height * (1 - layout.ratio),
        };
  const first = calculateTileLayoutFrames(layout.first, firstBounds);
  const second = calculateTileLayoutFrames(layout.second, secondBounds);
  return {
    leaves: { ...first.leaves, ...second.leaves },
    splits: [
      ...first.splits,
      ...second.splits,
      { ...bounds, id: layout.id, orientation: layout.orientation, ratio: layout.ratio },
    ],
  };
}

export function splitTileLeaf(
  layout: TileLayoutNode,
  targetPanelId: string,
  newPanelId: string,
  placement: TilePlacement,
  splitId: string,
): { layout: TileLayoutNode; changed: boolean } {
  if (layout.type === "leaf") {
    if (layout.panelId !== targetPanelId) {
      return { layout, changed: false };
    }
    return {
      changed: true,
      layout: {
        type: "split",
        id: splitId,
        orientation: placement === "right" ? "horizontal" : "vertical",
        ratio: 0.5,
        first: layout,
        second: createTileLeaf(newPanelId),
      },
    };
  }

  const first = splitTileLeaf(
    layout.first,
    targetPanelId,
    newPanelId,
    placement,
    splitId,
  );
  if (first.changed) {
    return { changed: true, layout: { ...layout, first: first.layout } };
  }
  const second = splitTileLeaf(
    layout.second,
    targetPanelId,
    newPanelId,
    placement,
    splitId,
  );
  return second.changed
    ? { changed: true, layout: { ...layout, second: second.layout } }
    : { changed: false, layout };
}

export function removeTileLeaf(
  layout: TileLayoutNode,
  panelId: string,
): {
  layout: TileLayoutNode;
  changed: boolean;
  focusFallbackPanelId: string | null;
} {
  if (layout.type === "leaf") {
    return { layout, changed: false, focusFallbackPanelId: null };
  }

  if (layout.first.type === "leaf" && layout.first.panelId === panelId) {
    return {
      layout: layout.second,
      changed: true,
      focusFallbackPanelId: flattenTilePanelIds(layout.second)[0] ?? null,
    };
  }
  if (layout.second.type === "leaf" && layout.second.panelId === panelId) {
    return {
      layout: layout.first,
      changed: true,
      focusFallbackPanelId: flattenTilePanelIds(layout.first)[0] ?? null,
    };
  }

  const first = removeTileLeaf(layout.first, panelId);
  if (first.changed) {
    return {
      layout: { ...layout, first: first.layout },
      changed: true,
      focusFallbackPanelId: first.focusFallbackPanelId,
    };
  }
  const second = removeTileLeaf(layout.second, panelId);
  return second.changed
    ? {
        layout: { ...layout, second: second.layout },
        changed: true,
        focusFallbackPanelId: second.focusFallbackPanelId,
      }
    : { layout, changed: false, focusFallbackPanelId: null };
}

export function resizeTileSplit(
  layout: TileLayoutNode,
  splitId: string,
  ratio: number,
): TileLayoutNode {
  if (layout.type === "leaf") {
    return layout;
  }
  if (layout.id === splitId) {
    const nextRatio = Math.min(0.85, Math.max(0.15, ratio));
    return nextRatio === layout.ratio ? layout : { ...layout, ratio: nextRatio };
  }
  const first = resizeTileSplit(layout.first, splitId, ratio);
  const second = first === layout.first
    ? resizeTileSplit(layout.second, splitId, ratio)
    : layout.second;
  if (first === layout.first && second === layout.second) {
    return layout;
  }
  return { ...layout, first, second };
}

export function validateTileLayout(
  layout: TileLayoutNode,
  expectedPanelIds: readonly string[],
): string[] {
  const errors: string[] = [];
  const ids = flattenTilePanelIds(layout);
  const seen = new Set<string>();
  for (const id of ids) {
    if (seen.has(id)) {
      errors.push(`duplicate panel leaf: ${id}`);
    }
    seen.add(id);
  }
  for (const id of expectedPanelIds) {
    if (!seen.has(id)) {
      errors.push(`missing panel leaf: ${id}`);
    }
  }
  const expected = new Set(expectedPanelIds);
  for (const id of seen) {
    if (!expected.has(id)) {
      errors.push(`unexpected panel leaf: ${id}`);
    }
  }
  return errors;
}
