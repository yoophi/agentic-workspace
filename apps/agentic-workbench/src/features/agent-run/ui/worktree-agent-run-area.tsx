import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AlertTriangleIcon, FolderGit2Icon } from "lucide-react";

import type { GitWorktree } from "@/entities/project/model/git-worktree";
import { cancelAgentRun } from "@/entities/agent-run/api/agent-run-repository";
import {
  acknowledgeAgentExchange,
  listenAgentExchangeRequests,
  listenAgentExchangeStatus,
  sendAgentExchange,
  syncAgentWorkspace,
} from "@/entities/agent-run/api/agent-exchange-repository";
import type {
  AgentExchange,
  AgentPanelEndpoint,
} from "@/entities/agent-run/model/agent-exchange";
import {
  AgentRunPanel,
  type AgentPromptRequest,
} from "@/features/agent-run/ui/agent-run-panel";
import { AgentRunPanelTabs } from "@/features/agent-run/ui/agent-run-panel-tabs";
import { AgentRunTileLayout } from "@/features/agent-run/ui/agent-run-tile-layout";
import { AgentRunWorkspaceToolbar } from "@/features/agent-run/ui/agent-run-workspace-toolbar";
import { AgentPeerMessageDialog } from "@/features/agent-run/ui/agent-peer-message-dialog";
import {
  addExtraPanel,
  cancelClosePanel,
  confirmClosePanel,
  createInitialAgentRunAreaState,
  getRunningPanelCount,
  removeClosedPanel,
  requestClosePanel,
  routePromptToActivePanel,
  routePromptToPanel,
  openAdjacentPanel,
  resizeAgentRunSplit,
  selectPanel,
  setAgentRunViewMode,
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
  const stateRef = useRef(state);
  const workspaceRevisionRef = useRef(0);
  const [targetMessage, setTargetMessage] = useState<string | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [messageSourcePanelId, setMessageSourcePanelId] = useState<string | null>(null);
  const [exchanges, setExchanges] = useState<AgentExchange[]>([]);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  useEffect(() => {
    workspaceRevisionRef.current += 1;
    void syncAgentWorkspace({
      worktreePath: worktree.path,
      revision: workspaceRevisionRef.current,
      focusedPanelId: state.focusedPanelId,
      panels: state.slots.map((slot) => ({
        panelId: slot.id,
        title: slot.title,
        runId: slot.activeRunId,
        status:
          slot.closeState !== "open"
            ? "closing"
            : slot.isRunning
              ? "running"
              : "idle",
      })),
    }).catch((error) => {
      setTargetMessage(`에이전트 workspace 동기화 실패: ${String(error)}`);
    });
  }, [state.focusedPanelId, state.slots, worktree.path]);

  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];

    void listenAgentExchangeRequests(async (request) => {
      if (disposed) {
        return;
      }
      const result = routePromptToPanel(stateRef.current, request.target.panelId, {
        id: request.requestId,
        text: request.message,
        delivery: request.delivery,
      });
      if (result.routed) {
        stateRef.current = result.state;
        setState(result.state);
      }
      try {
        await acknowledgeAgentExchange({
          requestId: request.requestId,
          targetPanelId: request.target.panelId,
          outcome: result.routed ? "delivered" : "rejected",
          reason: result.routed ? null : result.reason,
        });
      } catch (error) {
        setTargetMessage(`에이전트 메시지 상태 반영 실패: ${String(error)}`);
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

    void listenAgentExchangeStatus((exchange) => {
      if (disposed) {
        return;
      }
      setExchanges((current) => [
        ...current.filter((item) => item.requestId !== exchange.requestId),
        exchange,
      ].slice(-100));
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlisteners.push(unlisten);
      }
    });

    return () => {
      disposed = true;
      for (const unlisten of unlisteners) {
        unlisten();
      }
    };
  }, []);

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
      externalPromptRequest.delivery,
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

  const handleOpenAdjacent = useCallback(
    (panelId: string, placement: "right" | "below") => {
      setState((current) => {
        const result = openAdjacentPanel(current, panelId, placement);
        if (!result.opened) {
          setTargetMessage(
            result.reason === "panel-limit"
              ? "에이전트 타일은 최대 8개까지 열 수 있습니다."
              : result.reason === "depth-limit"
                ? "현재 방향으로 타일을 더 분할할 수 없습니다."
                : "선택한 타일을 찾을 수 없습니다.",
          );
        }
        return result.state;
      });
    },
    [],
  );

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
      <AgentRunWorkspaceToolbar
        viewMode={state.viewMode}
        onViewModeChange={(viewMode) =>
          setState((current) => setAgentRunViewMode(current, viewMode))
        }
        onAddPanel={() => handleOpenAdjacent(state.focusedPanelId, "right")}
      />
      {state.viewMode === "tabs" && (
        <AgentRunPanelTabs
          slots={state.slots}
          activePanelId={state.activePanelId}
          onSelectPanel={handleSelectPanel}
          onAddExtraPanel={handleAddExtraPanel}
          onClosePanel={handleClosePanel}
        />
      )}

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
        <AgentRunTileLayout
          layout={state.layout}
          viewMode={state.viewMode}
          focusedPanelId={state.focusedPanelId}
          panels={state.slots.map((slot) => {
            const pendingExchangeCount = exchanges.filter(
              (exchange) =>
                exchange.target.panelId === slot.id &&
                exchange.status === "accepted",
            ).length;
            return {
              panelId: slot.id,
              slot: { ...slot, pendingExchangeCount },
              content: (
                <AgentRunPanel
                  key={slot.id}
                  panelId={slot.id}
                  workingDirectory={worktree.path}
                  externalPromptRequest={slot.externalPromptRequest}
                  onOpenSettings={onOpenSettings}
                  variant={slot.kind}
                  onRunStateChange={handleRunStateChange}
                  scrollHeader={scrollHeader}
                />
              ),
            };
          })}
          onFocusPanel={handleSelectPanel}
          onOpenAdjacent={handleOpenAdjacent}
          onClosePanel={handleClosePanel}
          onMessagePeer={(panelId) => {
            setMessageSourcePanelId(panelId);
          }}
          onResizeSplit={(splitId, ratio) =>
            setState((current) => resizeAgentRunSplit(current, splitId, ratio))
          }
        />
      </div>

      <AgentPeerMessageDialog
        open={Boolean(messageSourcePanelId)}
        sourcePanelId={messageSourcePanelId}
        peers={state.slots
          .filter(
            (slot) =>
              slot.id !== messageSourcePanelId && slot.closeState === "open",
          )
          .map<AgentPanelEndpoint>((slot) => ({
            panelId: slot.id,
            title: slot.title,
            runId: slot.activeRunId,
            status: slot.isRunning ? "running" : "idle",
          }))}
        onOpenChange={(open) => {
          if (!open) {
            setMessageSourcePanelId(null);
          }
        }}
        onSubmit={async ({ sourcePanelId, targetPanelId, message, delivery }) => {
          const source = stateRef.current.slots.find(
            (slot) => slot.id === sourcePanelId,
          );
          const target = stateRef.current.slots.find(
            (slot) => slot.id === targetPanelId,
          );
          if (!source || !target) {
            throw new Error("메시지 대상 패널을 찾을 수 없습니다.");
          }
          const exchange = await sendAgentExchange({
            requestId: crypto.randomUUID(),
            sourcePanelId,
            sourceRunId: source.activeRunId,
            targetPanelId,
            targetRunId: target.activeRunId,
            message,
            delivery,
          });
          setExchanges((current) => [...current, exchange].slice(-100));
          setTargetMessage(`${target.title} 패널에 메시지를 전달했습니다.`);
        }}
      />

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
