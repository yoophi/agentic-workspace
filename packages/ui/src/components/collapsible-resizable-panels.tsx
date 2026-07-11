import { useEffect, useId, useRef, useState, type ReactNode } from "react";
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
  contentClassName?: string;
  showTitle?: boolean;
  defaultSize?: PanelSize;
  minSize?: PanelSize;
  maxSize?: PanelSize;
}

export type CollapsibleResizablePanelsState = PanelPairPublicState;

export interface CollapsibleResizablePanelsProps {
  leftPanel: CollapsiblePanelDefinition;
  rightPanel: CollapsiblePanelDefinition;
  collapsedSize?: PanelSize;
  className?: string;
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
  if (collapsed) {
    return (
      <div
        className="h-full w-full min-w-0 overflow-hidden"
        data-panel-side={side}
      >
        <button
          aria-controls={contentId}
          aria-expanded={false}
          className="relative h-full w-full min-w-0 cursor-pointer overflow-hidden rounded-sm bg-background px-2 py-2 font-medium text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none"
          onClick={onTitleActivate}
          type="button"
        >
          <span className="absolute top-2 right-8 origin-top-right -rotate-90 whitespace-nowrap">
            {panel.title}
          </span>
        </button>
      </div>
    );
  }

  if (panel.showTitle === false) {
    return (
      <div
        className={cn("h-full min-h-0 overflow-auto p-3", panel.contentClassName)}
        id={contentId}
      >
        {panel.content}
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col" data-panel-side={side}>
      <button
        aria-controls={contentId}
        aria-expanded={true}
        className="shrink-0 truncate border-b bg-background px-3 py-2 text-left font-medium text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset focus-visible:outline-none disabled:cursor-default disabled:text-muted-foreground"
        disabled={disableCollapse || panel.collapsible === false}
        onClick={onTitleActivate}
        type="button"
      >
        {panel.title}
      </button>
      <div
        className={cn("min-h-0 flex-1 overflow-auto p-3", panel.contentClassName)}
        id={contentId}
      >
        {panel.content}
      </div>
    </div>
  );
}

function CollapsibleResizablePanels({
  className,
  collapsedSize = "40px",
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
  const contentIdPrefix = useId();
  const leftContentId = `${contentIdPrefix}-${leftPanel.id}-content`;
  const rightContentId = `${contentIdPrefix}-${rightPanel.id}-content`;

  useEffect(() => {
    if (panelState.left.collapsed) {
      leftPanelRef.current?.collapse();
    } else if (panelState.right.collapsed) {
      rightPanelRef.current?.collapse();
    }
  }, [panelState.left.collapsed, panelState.right.collapsed]);

  const commitState = (nextState: PanelPairState) => {
    panelStateRef.current = nextState;
    setPanelState(nextState);
    onStateChange?.(toPanelPairPublicState(nextState));
  };

  const handleLayoutChanged = (layout: Layout) => {
    const leftSize = layout[leftPanel.id];
    const rightSize = layout[rightPanel.id];
    if (leftSize === undefined || rightSize === undefined) {
      return;
    }
    commitState(updatePanelLayout(panelStateRef.current, leftSize, rightSize));
  };

  const activatePanelTitle = (side: PanelSide) => {
    const currentState = panelStateRef.current;
    const panelRef = side === "left" ? leftPanelRef.current : rightPanelRef.current;

    if (currentState[side].collapsed) {
      const nextState = expandPanel(currentState, side);
      panelRef?.expand();
      panelRef?.resize(`${getPanelRestoreSize(nextState, side)}%`);
      commitState(nextState);
      return;
    }
    if (!canCollapsePanel(currentState, side)) {
      return;
    }

    const currentSize = panelRef?.getSize().asPercentage ?? currentState[side].lastExpandedSize;
    const nextState = collapsePanel(currentState, side, currentSize);
    panelRef?.collapse();
    commitState(nextState);
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
          onTitleActivate={() => activatePanelTitle("left")}
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
          onTitleActivate={() => activatePanelTitle("right")}
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
