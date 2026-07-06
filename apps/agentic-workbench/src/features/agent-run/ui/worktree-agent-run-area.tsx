import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangleIcon, FolderGit2Icon } from "lucide-react";

import type { GitWorktree } from "@/entities/project/model/git-worktree";
import { cancelAgentRun } from "@/entities/agent-run/api/agent-run-repository";
import {
  AgentRunPanel,
  type AgentPromptRequest,
} from "@/features/agent-run/ui/agent-run-panel";
import { AgentRunPanelTabs } from "@/features/agent-run/ui/agent-run-panel-tabs";
import {
  addExtraPanel,
  cancelClosePanel,
  confirmClosePanel,
  createInitialAgentRunAreaState,
  getRunningPanelCount,
  removeClosedPanel,
  requestClosePanel,
  routePromptToActivePanel,
  selectPanel,
  updatePanelRunState,
  type AgentPanelRunState,
} from "@/features/agent-run/model/agent-run-panel-slots";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { SystemMessage } from "@/components/ui/system-message";
import { WorktreeStatusBadge } from "@/entities/project/ui/worktree-status-badge";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type WorktreeAgentRunAreaProps = {
  worktree: GitWorktree;
  externalPromptRequest?: AgentPromptRequest | null;
  onOpenSettings?: () => void;
};

export function WorktreeAgentRunArea({
  worktree,
  externalPromptRequest = null,
  onOpenSettings,
}: WorktreeAgentRunAreaProps) {
  const [state, setState] = useState(createInitialAgentRunAreaState);
  const handledExternalPromptRequestIdRef = useRef<string | null>(null);
  const [targetMessage, setTargetMessage] = useState<string | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);

  useEffect(() => {
    if (
      !externalPromptRequest ||
      handledExternalPromptRequestIdRef.current === externalPromptRequest.id
    ) {
      return;
    }

    handledExternalPromptRequestIdRef.current = externalPromptRequest.id;
    const result = routePromptToActivePanel(
      state,
      externalPromptRequest.text,
      externalPromptRequest.id,
    );

    if (result.routed) {
      setState(result.state);
      setTargetMessage(`${result.target.title} 패널로 prompt를 보냈습니다.`);
      return;
    }

    if (result.reason === "closing-target") {
      setTargetMessage("닫히는 중인 agent 패널에는 prompt를 보낼 수 없습니다.");
    }
  }, [externalPromptRequest, state]);

  const handleRunStateChange = useCallback((report: AgentPanelRunState) => {
    setState((current) => updatePanelRunState(current, report));
  }, []);

  const handleSelectPanel = useCallback((panelId: string) => {
    setState((current) => selectPanel(current, panelId));
  }, []);

  const handleAddExtraPanel = useCallback(() => {
    setState(addExtraPanel);
  }, []);

  const handleClosePanel = useCallback((panelId: string) => {
    setCloseError(null);
    setState((current) => requestClosePanel(current, panelId));
  }, []);

  const scrollHeader = useMemo(
    () => (
      <div className="sticky top-0 z-20 flex min-w-0 items-center gap-2 border-b bg-background/95 px-3 py-2 backdrop-blur">
        <FolderGit2Icon className="size-4 shrink-0 text-muted-foreground" />
        <EllipsisPopoverText
          value={worktree.path}
          className="min-w-0 flex-1 font-mono text-xs text-muted-foreground"
          contentClassName="font-mono text-xs"
        />
        <Badge variant="outline" className="max-w-44 shrink-0 truncate font-mono">
          {worktree.branch || (worktree.status === "unknown" ? "…" : "-")}
        </Badge>
        <WorktreeStatusBadge status={worktree.status} />
      </div>
    ),
    [worktree.branch, worktree.path, worktree.status],
  );

  const confirmingCloseSlot = state.slots.find(
    (slot) => slot.closeState === "confirmingClose",
  );
  const runningPanelCount = getRunningPanelCount(state);

  async function confirmRunningClose(panelId: string) {
    setCloseError(null);
    const closeResult = confirmClosePanel(state, panelId);
    setState(closeResult.state);

    if (closeResult.activeRunId) {
      try {
        await cancelAgentRun(closeResult.activeRunId);
      } catch (caughtError) {
        setCloseError(String(caughtError));
      }
    }

    setState((current) => removeClosedPanel(current, panelId));
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <AgentRunPanelTabs
        slots={state.slots}
        activePanelId={state.activePanelId}
        onSelectPanel={handleSelectPanel}
        onAddExtraPanel={handleAddExtraPanel}
        onClosePanel={handleClosePanel}
      />

      {runningPanelCount > 1 && (
        <div className="border-b px-3 py-2">
          <SystemMessage fill>
            <span className="flex items-center gap-2">
              <AlertTriangleIcon className="size-4 shrink-0 text-amber-500" />
              같은 worktree에서 {runningPanelCount}개 agent가 실행 중입니다. 파일 변경이
              섞일 수 있습니다.
            </span>
          </SystemMessage>
        </div>
      )}

      {(targetMessage || closeError) && (
        <div className="border-b px-3 py-2" role="status" aria-live="polite">
          {targetMessage && (
            <SystemMessage fill>
              {targetMessage}
            </SystemMessage>
          )}
          {closeError && (
            <SystemMessage variant="error" fill>
              {closeError}
            </SystemMessage>
          )}
        </div>
      )}

      <div className="min-h-0 flex-1">
        {state.slots.map((slot) => (
          <div
            key={slot.id}
            className={slot.id === state.activePanelId ? "h-full min-h-0" : "hidden"}
          >
            <AgentRunPanel
              panelId={slot.id}
              workingDirectory={worktree.path}
              externalPromptRequest={slot.externalPromptRequest}
              onOpenSettings={onOpenSettings}
              variant={slot.kind}
              onRunStateChange={handleRunStateChange}
              scrollHeader={scrollHeader}
            />
          </div>
        ))}
      </div>

      <AlertDialog
        open={Boolean(confirmingCloseSlot)}
        onOpenChange={(open) => {
          if (!open && confirmingCloseSlot) {
            setState((current) => cancelClosePanel(current, confirmingCloseSlot.id));
          }
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>실행 중인 extra 패널 닫기</AlertDialogTitle>
            <AlertDialogDescription>
              {confirmingCloseSlot?.title ?? "Extra"} 패널에서 agent가 실행 중입니다. 실행을
              취소하고 패널을 닫거나, 닫기를 취소할 수 있습니다.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>닫기 취소</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => {
                if (confirmingCloseSlot) {
                  void confirmRunningClose(confirmingCloseSlot.id);
                }
              }}
            >
              실행 취소 후 닫기
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
