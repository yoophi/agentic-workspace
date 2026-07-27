import { Columns2Icon, LayoutGridIcon, PlusIcon } from "lucide-react";

import type { AgentRunViewMode } from "@/entities/agent-run/model/agent-run-workspace";
import { Button } from "@/components/ui/button";

type AgentRunWorkspaceToolbarProps = {
  viewMode: AgentRunViewMode;
  onViewModeChange: (mode: AgentRunViewMode) => void;
  onAddPanel: () => void;
};

export function AgentRunWorkspaceToolbar({
  viewMode,
  onViewModeChange,
  onAddPanel,
}: AgentRunWorkspaceToolbarProps) {
  return (
    <div
      className="flex h-10 shrink-0 items-center gap-1 border-b bg-muted/20 px-2"
      aria-label="Agent run 워크스페이스 도구"
    >
      {viewMode === "tiles" && (
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={onAddPanel}
          aria-label="현재 타일 오른쪽에 새 에이전트 패널 열기"
        >
          <PlusIcon className="size-4" />
          새 에이전트 패널
        </Button>
      )}
      <div
        className="ml-auto flex items-center gap-1"
        role="group"
        aria-label="Agent run 보기 방식"
      >
        <Button
          type="button"
          size="sm"
          variant={viewMode === "tabs" ? "secondary" : "ghost"}
          aria-pressed={viewMode === "tabs"}
          onClick={() => onViewModeChange("tabs")}
        >
          <Columns2Icon className="size-4" />
          탭
        </Button>
        <Button
          type="button"
          size="sm"
          variant={viewMode === "tiles" ? "secondary" : "ghost"}
          aria-pressed={viewMode === "tiles"}
          onClick={() => onViewModeChange("tiles")}
        >
          <LayoutGridIcon className="size-4" />
          타일
        </Button>
      </div>
    </div>
  );
}
