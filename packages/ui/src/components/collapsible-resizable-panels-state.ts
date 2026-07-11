export type PanelSide = "left" | "right";

export interface PanelRuntimeState {
  side: PanelSide;
  collapsed: boolean;
  lastExpandedSize: number;
  minSize: number;
  maxSize: number;
}

export interface PanelPairState {
  collapsedPanel: PanelSide | null;
  left: PanelRuntimeState;
  right: PanelRuntimeState;
  resizeEnabled: boolean;
}

export interface CreatePanelPairStateOptions {
  leftSize?: number;
  rightSize?: number;
  leftMinSize?: number;
  rightMinSize?: number;
  leftMaxSize?: number;
  rightMaxSize?: number;
}

export interface PanelPairPublicState {
  collapsedPanel: PanelSide | null;
  leftSize: number;
  rightSize: number;
}

export type PanelPairEvent =
  | { type: "layoutChanged"; leftSize: number; rightSize: number }
  | { type: "collapse"; side: PanelSide; size: number }
  | { type: "expand"; side: PanelSide };

export function clampPanelSize(size: number, minSize: number, maxSize: number): number {
  const lowerBound = Math.min(minSize, maxSize);
  const upperBound = Math.max(minSize, maxSize);
  const safeSize = Number.isFinite(size) ? size : lowerBound;
  return Math.min(Math.max(safeSize, lowerBound), upperBound);
}

export function createPanelPairState(
  options: CreatePanelPairStateOptions = {},
): PanelPairState {
  const leftMinSize = options.leftMinSize ?? 0;
  const rightMinSize = options.rightMinSize ?? 0;
  const leftMaxSize = options.leftMaxSize ?? 100;
  const rightMaxSize = options.rightMaxSize ?? 100;

  return {
    collapsedPanel: null,
    left: {
      side: "left",
      collapsed: false,
      lastExpandedSize: clampPanelSize(options.leftSize ?? 50, leftMinSize, leftMaxSize),
      minSize: leftMinSize,
      maxSize: leftMaxSize,
    },
    right: {
      side: "right",
      collapsed: false,
      lastExpandedSize: clampPanelSize(options.rightSize ?? 50, rightMinSize, rightMaxSize),
      minSize: rightMinSize,
      maxSize: rightMaxSize,
    },
    resizeEnabled: true,
  };
}

export function updatePanelLayout(
  state: PanelPairState,
  leftSize: number,
  rightSize: number,
): PanelPairState {
  if (!state.resizeEnabled || state.collapsedPanel !== null) {
    return state;
  }

  return {
    ...state,
    left: {
      ...state.left,
      lastExpandedSize: clampPanelSize(leftSize, state.left.minSize, state.left.maxSize),
    },
    right: {
      ...state.right,
      lastExpandedSize: clampPanelSize(rightSize, state.right.minSize, state.right.maxSize),
    },
  };
}

export function canCollapsePanel(state: PanelPairState, side: PanelSide): boolean {
  const otherSide: PanelSide = side === "left" ? "right" : "left";
  return !state[side].collapsed && !state[otherSide].collapsed;
}

export function collapsePanel(
  state: PanelPairState,
  side: PanelSide,
  currentSize: number,
): PanelPairState {
  if (!canCollapsePanel(state, side)) {
    return state;
  }

  return {
    ...state,
    collapsedPanel: side,
    [side]: {
      ...state[side],
      collapsed: true,
      lastExpandedSize: clampPanelSize(
        currentSize,
        state[side].minSize,
        state[side].maxSize,
      ),
    },
    resizeEnabled: false,
  };
}

export function expandPanel(state: PanelPairState, side: PanelSide): PanelPairState {
  if (!state[side].collapsed) {
    return state;
  }

  return {
    ...state,
    collapsedPanel: null,
    [side]: {
      ...state[side],
      collapsed: false,
    },
    resizeEnabled: true,
  };
}

export function getPanelRestoreSize(
  state: PanelPairState,
  side: PanelSide,
  minSize = state[side].minSize,
  maxSize = state[side].maxSize,
): number {
  return clampPanelSize(state[side].lastExpandedSize, minSize, maxSize);
}

export function reducePanelPairState(
  state: PanelPairState,
  event: PanelPairEvent,
): PanelPairState {
  if (event.type === "layoutChanged") {
    return updatePanelLayout(state, event.leftSize, event.rightSize);
  }
  if (event.type === "collapse") {
    return collapsePanel(state, event.side, event.size);
  }
  return expandPanel(state, event.side);
}

export function toPanelPairPublicState(state: PanelPairState): PanelPairPublicState {
  return {
    collapsedPanel: state.collapsedPanel,
    leftSize: state.left.lastExpandedSize,
    rightSize: state.right.lastExpandedSize,
  };
}
