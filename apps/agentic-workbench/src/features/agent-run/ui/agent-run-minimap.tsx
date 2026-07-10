import { useEffect, useMemo, useRef } from "react";

import type { MinimapEntry } from "@/entities/agent-run/model";
import {
  createViewportIndicator,
  keyboardSeekRatio,
  pointerSeekRatio,
  type MinimapSeekInput,
  type TimelineLayoutSnapshot,
} from "@/features/agent-run/model/agent-run-minimap";
import { cn } from "@/lib/utils";

type AgentRunMinimapProps = {
  entries: MinimapEntry[];
  layoutSnapshot: TimelineLayoutSnapshot;
  onSeek: (targetRatio: number, input: MinimapSeekInput) => void;
  className?: string;
};

type ActivePointer = {
  id: number;
  grabOffset: number;
};

export function AgentRunMinimap({
  entries,
  layoutSnapshot,
  onSeek,
  className,
}: AgentRunMinimapProps) {
  const trackRef = useRef<HTMLDivElement | null>(null);
  const indicatorRef = useRef<HTMLDivElement | null>(null);
  const activePointerRef = useRef<ActivePointer | null>(null);
  const frameRef = useRef<number | null>(null);
  const pendingPointerYRef = useRef<number | null>(null);
  const indicator = createViewportIndicator(layoutSnapshot);
  const isDisabled = indicator.disabled || entries.length === 0;
  const layoutById = useMemo(
    () => new Map(layoutSnapshot.itemLayouts.map((layout) => [layout.id, layout])),
    [layoutSnapshot.itemLayouts],
  );
  const lastSourceOrder = entries[entries.length - 1]?.sourceOrder ?? 0;

  useEffect(
    () => () => {
      if (frameRef.current !== null) {
        cancelAnimationFrame(frameRef.current);
      }
    },
    [],
  );

  const seekFromPointer = (clientY: number, grabOffset: number) => {
    const track = trackRef.current;
    const viewport = indicatorRef.current;
    if (!track || !viewport || isDisabled) {
      return;
    }
    const trackRect = track.getBoundingClientRect();
    const viewportRect = viewport.getBoundingClientRect();
    onSeek(
      pointerSeekRatio({
        pointerY: clientY,
        trackTop: trackRect.top,
        trackHeight: trackRect.height,
        indicatorHeight: viewportRect.height,
        grabOffset,
      }),
      "pointer",
    );
  };

  const schedulePointerSeek = (clientY: number, grabOffset: number) => {
    pendingPointerYRef.current = clientY;
    if (frameRef.current !== null) {
      return;
    }
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = null;
      if (pendingPointerYRef.current !== null) {
        seekFromPointer(pendingPointerYRef.current, grabOffset);
      }
    });
  };

  const indicatorTop = indicator.startRatio * (1 - indicator.visualSizeRatio);

  return (
    <aside
      aria-label="Agent run 대화 미니맵"
      className={cn(
        "flex h-full min-h-0 w-28 shrink-0 flex-col border-l bg-muted/30",
        className,
      )}
      data-agent-run-minimap
    >
      <div className="h-8 shrink-0 border-b px-2 py-2 text-[10px] font-medium text-muted-foreground">
        대화 미니맵
      </div>
      <div ref={trackRef} className="relative min-h-0 flex-1 overflow-hidden" data-minimap-track>
        {entries.length === 0 ? (
          <div className="grid h-full place-items-center px-2 text-center text-[10px] text-muted-foreground">
            대화 없음
          </div>
        ) : (
          <div aria-hidden className="absolute inset-1 overflow-hidden">
            {entries.map((entry) => {
              const layout = layoutById.get(entry.id);
              const fallbackRatio = lastSourceOrder > 0 ? entry.sourceOrder / lastSourceOrder : 0;
              const topRatio = layout
                ? layout.start / Math.max(1, layoutSnapshot.totalHeight)
                : fallbackRatio;
              const heightRatio = layout
                ? layout.height / Math.max(1, layoutSnapshot.totalHeight)
                : entry.contentWeight / Math.max(1, entries.length * 3);
              const showSummary = heightRatio >= 0.025 || entries.length <= 12;
              return (
                <div
                  key={entry.id}
                  className={cn(
                    "absolute left-0.5 right-0.5 overflow-hidden border-l-2 px-1 text-[9px] leading-3",
                    entry.role === "user"
                      ? "border-amber-500 bg-amber-400/25 text-amber-950 dark:text-amber-100"
                      : "border-sky-500 bg-sky-500/20 text-foreground",
                  )}
                  data-minimap-entry-role={entry.role}
                  style={{
                    top: `${Math.min(1, Math.max(0, topRatio)) * 100}%`,
                    height: `${Math.min(0.12, Math.max(0.004, heightRatio)) * 100}%`,
                  }}
                  title={entry.summary || (entry.role === "user" ? "사용자 프롬프트" : "Agent 출력")}
                >
                  {showSummary ? entry.summary : null}
                </div>
              );
            })}
          </div>
        )}

        <div
          ref={indicatorRef}
          role="slider"
          tabIndex={isDisabled ? -1 : 0}
          aria-label="현재 대화 위치"
          aria-orientation="vertical"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(indicator.startRatio * 100)}
          aria-disabled={isDisabled}
          className={cn(
            "absolute inset-x-0 z-10 border-y border-foreground/50 bg-background/35 outline-none backdrop-blur-[1px]",
            isDisabled
              ? "cursor-default"
              : "cursor-grab touch-none focus-visible:ring-2 focus-visible:ring-ring active:cursor-grabbing",
          )}
          style={{
            top: `${indicatorTop * 100}%`,
            height: `${indicator.visualSizeRatio * 100}%`,
          }}
          data-minimap-viewport
          onPointerDown={(event) => {
            if (isDisabled || event.button !== 0) {
              return;
            }
            const rect = event.currentTarget.getBoundingClientRect();
            const grabOffset = event.clientY - rect.top;
            activePointerRef.current = { id: event.pointerId, grabOffset };
            event.currentTarget.setPointerCapture(event.pointerId);
            schedulePointerSeek(event.clientY, grabOffset);
          }}
          onPointerMove={(event) => {
            const activePointer = activePointerRef.current;
            if (!activePointer || activePointer.id !== event.pointerId) {
              return;
            }
            schedulePointerSeek(event.clientY, activePointer.grabOffset);
          }}
          onPointerUp={(event) => {
            if (activePointerRef.current?.id !== event.pointerId) {
              return;
            }
            schedulePointerSeek(event.clientY, activePointerRef.current.grabOffset);
            activePointerRef.current = null;
            if (event.currentTarget.hasPointerCapture(event.pointerId)) {
              event.currentTarget.releasePointerCapture(event.pointerId);
            }
          }}
          onPointerCancel={(event) => {
            if (activePointerRef.current?.id === event.pointerId) {
              activePointerRef.current = null;
            }
          }}
          onKeyDown={(event) => {
            const targetRatio = keyboardSeekRatio({
              key: event.key,
              currentRatio: indicator.startRatio,
              snapshot: layoutSnapshot,
            });
            if (targetRatio === null) {
              return;
            }
            event.preventDefault();
            onSeek(targetRatio, "keyboard");
          }}
        />
      </div>
    </aside>
  );
}
