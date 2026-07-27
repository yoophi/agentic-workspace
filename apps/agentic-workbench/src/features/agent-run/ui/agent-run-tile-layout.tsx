import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import type {
  AgentRunPanelSlot,
  AgentRunViewMode,
} from "@/entities/agent-run/model/agent-run-workspace";
import {
  calculateTileLayoutFrames,
  type TileLayoutNode,
  type TilePlacement,
} from "@/entities/agent-run/model/tile-layout";
import { AgentRunTile } from "./agent-run-tile";

type TilePanel = {
  panelId: string;
  slot?: AgentRunPanelSlot;
  content: ReactNode;
};

type AgentRunTileLayoutProps = {
  layout: TileLayoutNode;
  panels: TilePanel[];
  viewMode?: AgentRunViewMode;
  focusedPanelId?: string;
  onFocusPanel?: (panelId: string) => void;
  onOpenAdjacent?: (panelId: string, placement: TilePlacement) => void;
  onClosePanel?: (panelId: string) => void;
  onMessagePeer?: (panelId: string) => void;
  onResizeSplit: (splitId: string, ratio: number) => void;
};

type DragState = {
  splitId: string;
  orientation: "horizontal" | "vertical";
  startCoordinate: number;
  startRatio: number;
  extent: number;
};

export function AgentRunTileLayout({
  layout,
  panels,
  viewMode = "tiles",
  focusedPanelId = panels[0]?.panelId ?? "",
  onFocusPanel = () => undefined,
  onOpenAdjacent = () => undefined,
  onClosePanel = () => undefined,
  onMessagePeer = () => undefined,
  onResizeSplit,
}: AgentRunTileLayoutProps) {
  const frames = useMemo(() => calculateTileLayoutFrames(layout), [layout]);
  const containerRef = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<DragState | null>(null);

  useEffect(() => {
    if (!drag) {
      return;
    }
    const handleMove = (event: PointerEvent) => {
      const coordinate = drag.orientation === "horizontal" ? event.clientX : event.clientY;
      onResizeSplit(
        drag.splitId,
        drag.startRatio + (coordinate - drag.startCoordinate) / Math.max(1, drag.extent),
      );
    };
    const handleUp = () => setDrag(null);
    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp, { once: true });
    return () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
    };
  }, [drag, onResizeSplit]);

  return (
    <div ref={containerRef} className="relative h-full min-h-0 overflow-hidden">
      {panels.map(({ panelId, slot, content }) => {
        const frame = frames.leaves[panelId];
        const visible = viewMode === "tiles" ? Boolean(frame) : panelId === focusedPanelId;
        const style =
          viewMode === "tiles" && frame
            ? {
                left: `${frame.left * 100}%`,
                top: `${frame.top * 100}%`,
                width: `${frame.width * 100}%`,
                height: `${frame.height * 100}%`,
                padding: "3px",
              }
            : { inset: 0 };
        return (
          <div
            key={panelId}
            className="absolute min-h-0 min-w-0"
            style={{ ...style, display: visible ? "block" : "none" }}
          >
            <AgentRunTile
              panelId={panelId}
              title={slot?.title ?? panelId}
              isFocused={panelId === focusedPanelId}
              isRunning={slot?.isRunning ?? false}
              canClose={slot?.kind === "extra"}
              pendingExchangeCount={slot?.pendingExchangeCount ?? 0}
              showHeader={viewMode === "tiles"}
              onFocus={onFocusPanel}
              onOpenAdjacent={onOpenAdjacent}
              onClose={onClosePanel}
              onMessagePeer={onMessagePeer}
            >
              {content}
            </AgentRunTile>
          </div>
        );
      })}

      {viewMode === "tiles" &&
        frames.splits.map((split) => {
          const horizontal = split.orientation === "horizontal";
          const position = horizontal
            ? {
                left: `${(split.left + split.width * split.ratio) * 100}%`,
                top: `${split.top * 100}%`,
                height: `${split.height * 100}%`,
              }
            : {
                left: `${split.left * 100}%`,
                top: `${(split.top + split.height * split.ratio) * 100}%`,
                width: `${split.width * 100}%`,
              };
          return (
            <button
              key={split.id}
              type="button"
              role="separator"
              aria-label={horizontal ? "타일 너비 조절" : "타일 높이 조절"}
              aria-orientation={horizontal ? "vertical" : "horizontal"}
              aria-valuemin={15}
              aria-valuemax={85}
              aria-valuenow={Math.round(split.ratio * 100)}
              className={
                horizontal
                  ? "absolute z-30 w-2 -translate-x-1/2 cursor-ew-resize bg-transparent hover:bg-primary/20 focus-visible:bg-primary/20"
                  : "absolute z-30 h-2 -translate-y-1/2 cursor-ns-resize bg-transparent hover:bg-primary/20 focus-visible:bg-primary/20"
              }
              style={position}
              onPointerDown={(event) => {
                const rect = containerRef.current?.getBoundingClientRect();
                if (!rect) {
                  return;
                }
                setDrag({
                  splitId: split.id,
                  orientation: split.orientation,
                  startCoordinate:
                    split.orientation === "horizontal" ? event.clientX : event.clientY,
                  startRatio: split.ratio,
                  extent:
                    split.orientation === "horizontal"
                      ? rect.width * split.width
                      : rect.height * split.height,
                });
              }}
              onKeyDown={(event) => {
                const delta =
                  event.key === "ArrowLeft" || event.key === "ArrowUp"
                    ? -0.05
                    : event.key === "ArrowRight" || event.key === "ArrowDown"
                      ? 0.05
                      : 0;
                if (delta !== 0) {
                  event.preventDefault();
                  onResizeSplit(split.id, split.ratio + delta);
                }
              }}
            />
          );
        })}
    </div>
  );
}
