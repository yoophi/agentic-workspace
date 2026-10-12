import { useId, useImperativeHandle, useRef, useState, type ReactNode, type Ref } from "react";
import type { Layout, PanelImperativeHandle } from "react-resizable-panels";

import { cn } from "@yoophi/ui/lib/utils";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@yoophi/ui/components/resizable";

import type {
  PanelPairPublicState,
  PanelSide,
} from "./collapsible-resizable-panels-state";
import {
  canCollapsePanel,
  collapsePanel,
  createPanelPairState,
  expandPanel,
  getPanelRestoreSize,
  toPanelPairPublicState,
  updatePanelLayout,
  type PanelPairState,
} from "./collapsible-resizable-panels-state";

type PanelSize = number | string;

export interface CollapsiblePanelDefinition {
  id: string;
  title: ReactNode;
  content: ReactNode;
  collapsible?: boolean;
  /** Preserve content identity and local state while hidden and inert. */
  keepMounted?: boolean;
  contentClassName?: string;
  showTitle?: boolean;
  defaultSize?: PanelSize;
  minSize?: PanelSize;
  maxSize?: PanelSize;
}

export type CollapsibleResizablePanelsState = PanelPairPublicState;

export interface CollapsibleResizablePanelsHandle {
  collapse: (side: PanelSide) => void;
  expand: (side: PanelSide) => void;
  toggle: (side: PanelSide) => void;
  getState: () => CollapsibleResizablePanelsState;
}

export interface CollapsibleResizablePanelsProps {
  leftPanel: CollapsiblePanelDefinition;
  rightPanel: CollapsiblePanelDefinition;
  collapsedSize?: PanelSize;
  className?: string;
  defaultCollapsedPanel?: PanelSide | null;
  controllerRef?: Ref<CollapsibleResizablePanelsHandle>;
  onStateChange?: (state: CollapsibleResizablePanelsState) => void;
}

function PanelContents({
  collapsed,
  contentId,
  disableCollapse,
  onTitleActivate,
  panel,
  side,
}: {
  collapsed: boolean;
  contentId: string;
  disableCollapse: boolean;
  onTitleActivate: () => void;
  panel: CollapsiblePanelDefinition;
  side: PanelSide;
}) {
  return (
    <div
      className={cn("h-full min-h-0 min-w-0 overflow-hidden", !collapsed && "flex flex-col")}
      data-panel-side={side}
    >
      {panel.showTitle !== false && (
        <button
          aria-controls={contentId}
          aria-expanded={!collapsed}
          className={collapsed
            ? "relative h-full w-full min-w-0 cursor-pointer overflow-hidden rounded-sm bg-background px-2 py-2 font-medium text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none"
            : "shrink-0 truncate border-b bg-background px-3 py-2 text-left font-medium text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none disabled:cursor-default disabled:text-muted-foreground"}
          disabled={!collapsed && (disableCollapse || panel.collapsible === false)}
          onClick={onTitleActivate}
          type="button"
        >
          <span className={collapsed ? "absolute top-2 right-8 origin-top-right -rotate-90 whitespace-nowrap" : undefined}>
            {panel.title}
          </span>
        </button>
      )}
      {(panel.keepMounted || !collapsed) && (
        <div
          className={cn("min-h-0 flex-1 overflow-auto p-3", panel.contentClassName)}
          hidden={collapsed}
          inert={collapsed || undefined}
          // Inline display keeps hidden effective even when contentClassName uses flex/grid.
          style={collapsed ? { display: "none" } : undefined}
          id={contentId}
        >
          {panel.content}
        </div>
      )}
    </div>
  );
}

function CollapsibleResizablePanels({
  className,
  collapsedSize = "40px",
  controllerRef,
  defaultCollapsedPanel = null,
  leftPanel,
  onStateChange,
  rightPanel,
}: CollapsibleResizablePanelsProps) {
  if (!leftPanel.id.trim() || !rightPanel.id.trim()) {
    throw new Error("CollapsibleResizablePanels requires non-empty panel ids.");
  }
  if (leftPanel.id === rightPanel.id) {
    throw new Error("CollapsibleResizablePanels requires unique panel ids.");
  }

  const initialStateRef = useRef<PanelPairState | null>(null);
  if (initialStateRef.current === null) {
    initialStateRef.current = createPanelPairState({
      leftSize: sizeAsPercentage(leftPanel.defaultSize, 50),
      rightSize: sizeAsPercentage(rightPanel.defaultSize, 50),
    });
  }
  const [panelState, setPanelState] = useState(initialStateRef.current);
  const panelStateRef = useRef(panelState);
  const leftPanelRef = useRef<PanelImperativeHandle | null>(null);
  const rightPanelRef = useRef<PanelImperativeHandle | null>(null);
  const initialCollapseRef = useRef(defaultCollapsedPanel);
  const measuredRef = useRef(false);
  const contentIdPrefix = useId();
  const leftContentId = `${contentIdPrefix}-${leftPanel.id}-content`;
  const rightContentId = `${contentIdPrefix}-${rightPanel.id}-content`;

  const commitState = (nextState: PanelPairState) => {
    if (nextState === panelStateRef.current) return;
    panelStateRef.current = nextState;
    setPanelState(nextState);
    onStateChange?.(toPanelPairPublicState(nextState));
  };

  const definitionFor = (side: PanelSide) => side === "left" ? leftPanel : rightPanel;
  const refFor = (side: PanelSide) => side === "left" ? leftPanelRef.current : rightPanelRef.current;
  const isCollapsible = (side: PanelSide) => {
    const panel = definitionFor(side);
    return panel.collapsible !== false && panel.showTitle !== false;
  };

  const collapse = (side: PanelSide) => {
    const current = panelStateRef.current;
    const ref = refFor(side);
    if (!measuredRef.current || !ref || !isCollapsible(side) || !canCollapsePanel(current, side)) return;
    // Commit first: synchronous layout callbacks must not overwrite the remembered width.
    commitState(collapsePanel(current, side, ref.getSize().asPercentage));
    ref.collapse();
  };

  const expand = (side: PanelSide) => {
    const current = panelStateRef.current;
    const ref = refFor(side);
    if (!ref || !current[side].collapsed) return;
    const next = expandPanel(current, side);
    const restoreSize = getPanelRestoreSize(next, side);
    commitState(next);
    // The primitive applies the current pixel/percentage min/max constraints.
    ref.expand();
    ref.resize(`${restoreSize}%`);
  };

  const toggle = (side: PanelSide) => {
    if (panelStateRef.current[side].collapsed) expand(side);
    else collapse(side);
  };

  useImperativeHandle(controllerRef, () => ({
    collapse,
    expand,
    toggle,
    getState: () => toPanelPairPublicState(panelStateRef.current),
  }));

  const handleLayoutChanged = (layout: Layout) => {
    const leftSize = layout[leftPanel.id];
    const rightSize = layout[rightPanel.id];
    if (leftSize === undefined || rightSize === undefined) return;
    if (!measuredRef.current) {
      // A hidden/zero-width group has not measured its px defaults yet.
      if (!leftPanelRef.current || !rightPanelRef.current ||
          leftPanelRef.current.getSize().inPixels + rightPanelRef.current.getSize().inPixels <= 0) return;
      measuredRef.current = true;
      commitState(updatePanelLayout(panelStateRef.current, leftSize, rightSize));
      const initialSide = initialCollapseRef.current;
      initialCollapseRef.current = null;
      if (initialSide) collapse(initialSide);
      return;
    }
    const current = panelStateRef.current;
    if (current.collapsedPanel !== null) return;
    commitState(updatePanelLayout(current, leftSize, rightSize));
  };

  return (
    <ResizablePanelGroup
      className={cn("min-h-0", className)}
      disabled={!panelState.resizeEnabled}
      onLayoutChanged={handleLayoutChanged}
      orientation="horizontal"
    >
      <ResizablePanel
        className="min-w-0 overflow-hidden"
        collapsedSize={collapsedSize}
        collapsible={leftPanel.collapsible !== false && leftPanel.showTitle !== false}
        defaultSize={leftPanel.defaultSize}
        id={leftPanel.id}
        maxSize={panelState.right.collapsed ? "100%" : leftPanel.maxSize}
        minSize={leftPanel.minSize}
        panelRef={leftPanelRef}
      >
        <PanelContents
          collapsed={panelState.left.collapsed}
          contentId={leftContentId}
          disableCollapse={!canCollapsePanel(panelState, "left")}
          onTitleActivate={() => toggle("left")}
          panel={leftPanel}
          side="left"
        />
      </ResizablePanel>
      <ResizableHandle
        aria-disabled={!panelState.resizeEnabled}
        data-disabled={!panelState.resizeEnabled ? "" : undefined}
        disabled={!panelState.resizeEnabled}
      />
      <ResizablePanel
        className="min-w-0 overflow-hidden"
        collapsedSize={collapsedSize}
        collapsible={rightPanel.collapsible !== false && rightPanel.showTitle !== false}
        defaultSize={rightPanel.defaultSize}
        id={rightPanel.id}
        maxSize={panelState.left.collapsed ? "100%" : rightPanel.maxSize}
        minSize={rightPanel.minSize}
        panelRef={rightPanelRef}
      >
        <PanelContents
          collapsed={panelState.right.collapsed}
          contentId={rightContentId}
          disableCollapse={!canCollapsePanel(panelState, "right")}
          onTitleActivate={() => toggle("right")}
          panel={rightPanel}
          side="right"
        />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}

function sizeAsPercentage(size: PanelSize | undefined, fallback: number): number {
  if (typeof size !== "string") {
    return fallback;
  }
  const match = size.trim().match(/^(\d+(?:\.\d+)?)%?$/);
  return match ? Number(match[1]) : fallback;
}

export { CollapsibleResizablePanels };
