import { CircleIcon, PlusIcon, XIcon } from "lucide-react";

import type { AgentRunPanelSlot } from "@/features/agent-run/model/agent-run-panel-slots";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type AgentRunPanelTabsProps = {
  slots: AgentRunPanelSlot[];
  activePanelId: string;
  onSelectPanel: (panelId: string) => void;
  onAddExtraPanel: () => void;
  onClosePanel: (panelId: string) => void;
};

export function AgentRunPanelTabs({
  slots,
  activePanelId,
  onSelectPanel,
  onAddExtraPanel,
  onClosePanel,
}: AgentRunPanelTabsProps) {
  return (
    <div className="flex h-11 min-w-0 items-center gap-1 overflow-x-auto border-b bg-background px-2">
      <div className="flex min-w-0 flex-1 items-center gap-1" role="tablist" aria-label="Agent panels">
        {slots.map((slot) => {
          const isActive = slot.id === activePanelId;
          return (
            <div
              key={slot.id}
              className={cn(
                "group flex h-8 shrink-0 items-center rounded-md border text-sm",
                isActive
                  ? "border-border bg-muted text-foreground"
                  : "border-transparent text-muted-foreground hover:bg-muted/60 hover:text-foreground",
                slot.closeState === "closing" && "opacity-60",
              )}
            >
              <button
                type="button"
                role="tab"
                aria-selected={isActive}
                disabled={slot.closeState === "closing"}
                className="flex h-full min-w-0 items-center gap-1.5 px-2.5"
                onClick={() => onSelectPanel(slot.id)}
              >
                {slot.isRunning && (
                  <CircleIcon className="size-2.5 fill-emerald-500 text-emerald-500" />
                )}
                <span className="max-w-24 truncate">{slot.title}</span>
              </button>
              {slot.kind === "extra" && (
                <Button
                  type="button"
                  size="icon"
                  variant="ghost"
                  aria-label={`${slot.title} 닫기`}
                  disabled={slot.closeState === "closing"}
                  className="mr-0.5 size-6 opacity-80 hover:opacity-100"
                  onClick={(event) => {
                    event.stopPropagation();
                    onClosePanel(slot.id);
                  }}
                >
                  <XIcon className="size-3.5" />
                </Button>
              )}
            </div>
          );
        })}
      </div>
      <Button
        type="button"
        size="icon"
        variant="outline"
        aria-label="Extra agent panel 추가"
        className="size-8 shrink-0"
        onClick={onAddExtraPanel}
      >
        <PlusIcon className="size-4" />
      </Button>
    </div>
  );
}
