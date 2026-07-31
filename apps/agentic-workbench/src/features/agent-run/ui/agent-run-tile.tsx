import type { ReactNode } from "react";
import {
  CircleIcon,
  MessageSquareIcon,
  PanelBottomOpenIcon,
  PanelRightOpenIcon,
  XIcon,
} from "lucide-react";

import type { TilePlacement } from "@/entities/agent-run/model/tile-layout";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type AgentRunTileProps = {
  panelId: string;
  title: string;
  isFocused: boolean;
  isRunning: boolean;
  canClose: boolean;
  pendingExchangeCount: number;
  showHeader?: boolean;
  children: ReactNode;
  onFocus: (panelId: string) => void;
  onOpenAdjacent: (panelId: string, placement: TilePlacement) => void;
  onClose: (panelId: string) => void;
  onMessagePeer: (panelId: string) => void;
};

export function AgentRunTile({
  panelId,
  title,
  isFocused,
  isRunning,
  canClose,
  pendingExchangeCount,
  showHeader = true,
  children,
  onFocus,
  onOpenAdjacent,
  onClose,
  onMessagePeer,
}: AgentRunTileProps) {
  return (
    <section
      tabIndex={showHeader ? 0 : -1}
      className={cn(
        "flex h-full min-h-0 min-w-0 flex-col overflow-hidden bg-background",
        showHeader && "rounded-md border",
        isFocused && showHeader && "ring-1 ring-primary/70",
      )}
      aria-label={`${title} agent run 타일`}
      aria-current={isFocused ? "true" : undefined}
      onPointerDown={() => onFocus(panelId)}
      onFocusCapture={() => onFocus(panelId)}
    >
      {showHeader && (
        <header className="flex h-9 shrink-0 items-center gap-1 border-b bg-muted/30 px-2">
          {isRunning && (
            <CircleIcon
              className="size-2.5 fill-emerald-500 text-emerald-500"
              aria-label="실행 중"
            />
          )}
          <span className="min-w-0 flex-1 truncate text-xs font-medium">{title}</span>
          {pendingExchangeCount > 0 && (
            <span
              className="rounded-full bg-primary/10 px-1.5 text-micro text-primary"
              aria-label={`대기 중인 교환 ${pendingExchangeCount}개`}
            >
              {pendingExchangeCount}
            </span>
          )}
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="size-7"
            aria-label="다른 에이전트에게 메시지"
            onClick={() => onMessagePeer(panelId)}
          >
            <MessageSquareIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="size-7"
            aria-label="오른쪽에 새 타일 열기"
            onClick={() => onOpenAdjacent(panelId, "right")}
          >
            <PanelRightOpenIcon className="size-3.5" />
          </Button>
          <Button
            type="button"
            size="icon"
            variant="ghost"
            className="size-7"
            aria-label="아래에 새 타일 열기"
            onClick={() => onOpenAdjacent(panelId, "below")}
          >
            <PanelBottomOpenIcon className="size-3.5" />
          </Button>
          {canClose && (
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="size-7"
              aria-label={`${title} 닫기`}
              onClick={() => onClose(panelId)}
            >
              <XIcon className="size-3.5" />
            </Button>
          )}
        </header>
      )}
      <div className="min-h-0 flex-1">{children}</div>
    </section>
  );
}
