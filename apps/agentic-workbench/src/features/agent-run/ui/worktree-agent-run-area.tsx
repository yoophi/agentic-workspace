import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import { AlertTriangleIcon, FolderGit2Icon } from "lucide-react";

import type { GitWorktree } from "@/entities/project/model/git-worktree";
import { cancelAgentRun } from "@/entities/agent-run/api/agent-run-repository";
import {
  bindMainCoordinatorRun,
  delegateOrchestrationGoal,
  dispatchOrchestrationPrompt,
  adoptManualOrchestrationChild,
  bootstrapOrchestrationWorkspace,
  getOrchestrationWorkspace,
  listRecoverableOrchestrationWorkspaces,
  listenOrchestrationWorkspaceUpdated,
  MAIN_AGENT_NODE_ID,
  setOrchestrationPresentation,
  respondOrchestrationInput,
  cancelOrchestrationTask,
  retryOrchestrationTask,
  reassignOrchestrationTask,
  recoverOrchestrationWorkspace,
  sendOrchestrationChildCommand,
  type AgentNode,
  initialPromptDispatchState,
  handoffOrchestrationCoordinator,
  promptDispatchReducer,
  describeOrchestrationFailure,
  parseOrchestrationError,
  type OrchestrationSession,
} from "@/entities/agent-orchestration";
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
  detachOrchestrationPanel,
  promoteOrchestrationNode,
  selectPanel,
  setAgentRunViewMode,
  updatePanelRunState,
  type AgentPanelRunState,
} from "@/features/agent-run/model/agent-run-panel-slots";
import { shouldBindMainRun } from "@/features/agent-run/model/orchestration-workspace";
import { AgentRunControllerRegistry } from "@/features/agent-run/model/agent-run-controller";
import { TaskActivityRail } from "@/features/agent-run/ui/task-activity-rail";
import { WorkspacePromptComposer } from "@/features/agent-run/ui/workspace-prompt-composer";
import { PromptDispatchStatus } from "@/features/agent-run/ui/prompt-dispatch-status";
import { AgentRunRuntimeHost } from "@/features/agent-run/ui/agent-run-runtime-host";
import { CoordinatorHandoffDialog } from "@/features/agent-run/ui/coordinator-handoff-dialog";
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
import { Button } from "@/components/ui/button";
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
  const [orchestrationSession, setOrchestrationSession] =
    useState<OrchestrationSession | null>(null);
  const [recoverableSessions, setRecoverableSessions] = useState<
    OrchestrationSession[]
  >([]);
  const [isStartingOrchestration, setIsStartingOrchestration] = useState(true);
  const [dispatchState, dispatchPrompt] = useReducer(
    promptDispatchReducer,
    initialPromptDispatchState,
  );
  const [pendingMainHandoffRunId, setPendingMainHandoffRunId] =
    useState<string | null>(null);
  const pendingMainStartRef = useRef<{
    runId: string;
    resolve: () => void;
    reject: (error: Error) => void;
  } | null>(null);
  const orchestrationSessionRef = useRef<OrchestrationSession | null>(null);
  const boundMainRunIdRef = useRef<string | null>(null);
  const adoptedPanelIdsRef = useRef(new Set<string>());
  const runtimeControllerRegistryRef = useRef(new AgentRunControllerRegistry());
  const lastDirectPromptRef = useRef<string | null>(null);
  const [, setRuntimeControllerRevision] = useState(0);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  useEffect(() => {
    orchestrationSessionRef.current = orchestrationSession;
  }, [orchestrationSession]);

  const startOrchestrationWorkspace = useCallback(
    async (resumeWorkspaceId?: string) => {
      setIsStartingOrchestration(true);
      try {
        const session = await bootstrapOrchestrationWorkspace({
          worktreePath: worktree.path,
          resumeWorkspaceId,
        });
        setOrchestrationSession(session);
        const recovered = await recoverOrchestrationWorkspace();
        setOrchestrationSession(recovered);
        setRecoverableSessions([]);
      } catch (error) {
        setTargetMessage(`오케스트레이션 workspace 시작 실패: ${String(error)}`);
      } finally {
        setIsStartingOrchestration(false);
      }
    },
    [worktree.path],
  );

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    setOrchestrationSession(null);
    setRecoverableSessions([]);
    setIsStartingOrchestration(true);
    void listRecoverableOrchestrationWorkspaces(worktree.path)
      .then((sessions) => {
        if (disposed) return;
        if (sessions.length > 0) {
          setRecoverableSessions(sessions);
          setIsStartingOrchestration(false);
          return;
        }
        void startOrchestrationWorkspace();
      })
      .catch((error) => {
        if (!disposed) {
          setIsStartingOrchestration(false);
          setTargetMessage(`복구 가능한 workspace 조회 실패: ${String(error)}`);
        }
      });
    void listenOrchestrationWorkspaceUpdated(async (event) => {
      const current = orchestrationSessionRef.current;
      if (
        disposed ||
        (current && event.workspaceId !== current.id) ||
        (current && event.revision <= current.revision)
      ) {
        return;
      }
      const snapshot = await getOrchestrationWorkspace();
      if (!disposed && snapshot) {
        setOrchestrationSession(snapshot);
      }
    }).then((dispose) => {
      if (disposed) {
        dispose();
      } else {
        unlisten = dispose;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [startOrchestrationWorkspace, worktree.path]);

  const mainRunId =
    state.slots.find((slot) => slot.id === MAIN_AGENT_NODE_ID)?.activeRunId ??
    null;
  const prepareMainCoordinatorRun = useCallback(async (runId: string) => {
    const session = orchestrationSessionRef.current;
    if (!session) {
      throw new Error("Main Coordinator workspace가 준비되지 않았습니다.");
    }
    const activeGeneration = session.generations.find(
      (generation) => generation.id === session.activeCoordinatorGenerationId,
    );
    if (activeGeneration && activeGeneration.runId !== runId) {
      if (pendingMainStartRef.current) {
        throw new Error("다른 Main Coordinator 인계를 처리하고 있습니다.");
      }
      await new Promise<void>((resolve, reject) => {
        pendingMainStartRef.current = { runId, resolve, reject };
        setPendingMainHandoffRunId(runId);
      });
      return;
    }
    const next = await bindMainCoordinatorRun({
      requestId: crypto.randomUUID(),
      panelId: MAIN_AGENT_NODE_ID,
      runId,
      state: "active",
      expectedRevision: session.revision,
    });
    boundMainRunIdRef.current = runId;
    orchestrationSessionRef.current = next;
    setOrchestrationSession(next);
  }, []);

  useEffect(() => {
    if (
      !orchestrationSession ||
      !shouldBindMainRun(boundMainRunIdRef.current, mainRunId)
    ) {
      return;
    }
    const runId = mainRunId;
    const activeGeneration = orchestrationSession.generations.find(
      (generation) =>
        generation.id === orchestrationSession.activeCoordinatorGenerationId,
    );
    if (activeGeneration && activeGeneration.runId !== runId) {
      setPendingMainHandoffRunId(runId);
      return;
    }
    boundMainRunIdRef.current = runId;
    void bindMainCoordinatorRun({
      requestId: crypto.randomUUID(),
      panelId: MAIN_AGENT_NODE_ID,
      runId,
      state: "active",
      expectedRevision: orchestrationSession.revision,
    })
      .then(setOrchestrationSession)
      .catch((error) => {
        boundMainRunIdRef.current = null;
        setTargetMessage(`Main Coordinator 결합 실패: ${String(error)}`);
      });
  }, [mainRunId, orchestrationSession]);

  useEffect(() => {
    if (!orchestrationSession) {
      return;
    }
    for (const slot of state.slots) {
      if (
        slot.kind !== "extra" ||
        orchestrationSession.nodes.some((node) => node.id === slot.id) ||
        adoptedPanelIdsRef.current.has(slot.id)
      ) {
        continue;
      }
      adoptedPanelIdsRef.current.add(slot.id);
      void adoptManualOrchestrationChild({
        panelId: slot.id,
        title: slot.title,
      })
        .then(setOrchestrationSession)
        .catch((error) => {
          adoptedPanelIdsRef.current.delete(slot.id);
          setTargetMessage(`하위 에이전트 등록 실패: ${String(error)}`);
        });
    }
  }, [orchestrationSession, state.slots]);

  useEffect(() => {
    workspaceRevisionRef.current += 1;
    void syncAgentWorkspace({
      worktreePath: worktree.path,
      revision: workspaceRevisionRef.current,
      focusedPanelId: state.focusedPanelId,
      panels: state.slots.map((slot) => ({
        panelId: slot.id,
        title: slot.title,
        runId: slot.isRunning ? slot.activeRunId : null,
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
    const node = orchestrationSessionRef.current?.nodes.find(
      (candidate) => candidate.id === panelId && candidate.kind === "child",
    );
    if (!node || !orchestrationSessionRef.current) {
      setState((current) => requestClosePanel(current, panelId));
      return;
    }
    const session = orchestrationSessionRef.current;
    setState((current) => detachOrchestrationPanel(current, panelId));
    void setOrchestrationPresentation({
      requestId: crypto.randomUUID(),
      nodeId: panelId,
      presentationStatus: "detached",
      expectedRevision: session.revision,
    })
      .then(setOrchestrationSession)
      .catch((error) => setCloseError(String(error)));
  }, []);

  const handlePromoteNode = useCallback((node: AgentNode) => {
    const session = orchestrationSessionRef.current;
    if (!session) return;
    setState((current) =>
      promoteOrchestrationNode(current, {
        id: node.id,
        title: node.role.name,
        runId:
          node.executionStatus === "active" ? node.currentRunId : null,
        isRunning: node.executionStatus === "active",
      }),
    );
    void setOrchestrationPresentation({
      requestId: crypto.randomUUID(),
      nodeId: node.id,
      presentationStatus: "panel",
      expectedRevision: session.revision,
    })
      .then(setOrchestrationSession)
      .catch((error) => setTargetMessage(`패널 승격 실패: ${String(error)}`));
  }, []);

  const handleRuntimeControllerChange = useCallback(() => {
    setRuntimeControllerRevision((revision) => revision + 1);
  }, []);

  const handleRuntimeReplayGap = useCallback(() => {
    void getOrchestrationWorkspace().then((snapshot) => {
      if (snapshot) setOrchestrationSession(snapshot);
    });
  }, []);

  const runTaskAction = useCallback(
    (
      action: (input: {
        requestId: string;
        taskId: string;
        message?: string;
        expectedRevision: number;
      }) => Promise<unknown>,
      taskId: string,
      message?: string,
    ) => {
      const session = orchestrationSessionRef.current;
      if (!session) return;
      void action({
        requestId: crypto.randomUUID(),
        taskId,
        message,
        expectedRevision: session.revision,
      })
        .then(() => getOrchestrationWorkspace())
        .then((snapshot) => {
          if (snapshot) setOrchestrationSession(snapshot);
        })
        .catch((error) => setTargetMessage(String(error)));
    },
    [],
  );

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
      {orchestrationSession && (
        <AgentRunRuntimeHost
          nodes={orchestrationSession.nodes}
          registry={runtimeControllerRegistryRef.current}
          onControllerStateChange={handleRuntimeControllerChange}
          onReplayGap={handleRuntimeReplayGap}
        />
      )}
      <AgentRunWorkspaceToolbar
        viewMode={state.viewMode}
        onViewModeChange={(viewMode) =>
          setState((current) => setAgentRunViewMode(current, viewMode))
        }
        onAddPanel={() => handleOpenAdjacent(state.focusedPanelId, "right")}
      />
      {recoverableSessions.length > 0 && !orchestrationSession && (
        <div
          className="border-b border-amber-500/30 bg-amber-500/5 px-3 py-3"
          role="alert"
          aria-label="복구 가능한 에이전트 작업"
        >
          <div className="flex flex-wrap items-center gap-3">
            <AlertTriangleIcon className="size-4 shrink-0 text-amber-500" />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium">
                이전 에이전트 작업을 복구할 수 있습니다.
              </p>
              <p className="text-xs text-muted-foreground">
                작업 {recoverableSessions[0].tasks.length}개 · 마지막 변경{" "}
                {new Date(recoverableSessions[0].updatedAt).toLocaleString()}
              </p>
            </div>
            <Button
              size="sm"
              disabled={isStartingOrchestration}
              onClick={() =>
                void startOrchestrationWorkspace(recoverableSessions[0].id)
              }
            >
              이전 작업 복구
            </Button>
            <Button
              size="sm"
              variant="outline"
              disabled={isStartingOrchestration}
              onClick={() => void startOrchestrationWorkspace()}
            >
              새로 시작
            </Button>
          </div>
        </div>
      )}
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

      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
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
            const orchestrationNode = orchestrationSession?.nodes.find(
              (node) => node.id === slot.id,
            );
            const runtimeController = runtimeControllerRegistryRef.current.get(
              orchestrationNode?.currentRunId,
            );
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
                  onBeforeRunStart={
                    slot.id === MAIN_AGENT_NODE_ID
                      ? prepareMainCoordinatorRun
                      : undefined
                  }
                  scrollHeader={scrollHeader}
                  showPromptComposer={false}
                  existingRunId={
                    orchestrationNode?.kind === "child"
                      ? orchestrationNode.currentRunId
                      : undefined
                  }
                  existingIsRunning={
                    orchestrationNode?.executionStatus === "active"
                  }
                  runtimeHydrated={
                    !runtimeController ||
                    !["idle", "loading"].includes(
                      runtimeController.snapshot.hydrationStatus,
                    )
                  }
                  initialPermissionMode={
                    orchestrationNode?.kind === "child" ? "readOnly" : undefined
                  }
                  replayedEvents={runtimeController?.snapshot.events}
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
        {orchestrationSession && orchestrationSession.tasks.length > 0 && (
          <TaskActivityRail
            session={orchestrationSession}
            runtimeStates={Object.fromEntries(
              orchestrationSession.nodes.flatMap((node) => {
                if (!node.currentRunId) return [];
                const controller = runtimeControllerRegistryRef.current.get(
                  node.currentRunId,
                );
                return controller
                  ? [
                      [
                        node.currentRunId,
                        controller.snapshot.hydrationStatus,
                      ] as const,
                    ]
                  : [];
              }),
            )}
            onPromote={handlePromoteNode}
            onDetach={(node) => handleClosePanel(node.id)}
            onRespond={(taskId, response) =>
              runTaskAction(respondOrchestrationInput, taskId, response)
            }
            onCancel={(taskId) =>
              runTaskAction(cancelOrchestrationTask, taskId)
            }
            onRetry={(taskId) =>
              runTaskAction(retryOrchestrationTask, taskId)
            }
            onReassign={(taskId, targetNodeId) => {
              const session = orchestrationSessionRef.current;
              if (!session) return;
              void reassignOrchestrationTask({
                requestId: crypto.randomUUID(),
                taskId,
                targetNodeId,
                expectedRevision: session.revision,
              })
                .then(setOrchestrationSession)
                .catch((error) => setTargetMessage(String(error)));
            }}
          />
        )}
      </div>

      <WorkspacePromptComposer
        slots={state.slots}
        focusedPanelId={state.focusedPanelId}
        disabled={!orchestrationSession}
        onSubmit={async ({ requestId, message, mode, panelIds, delegate }) => {
          lastDirectPromptRef.current = message;
          dispatchPrompt({ type: "queued", dispatchId: requestId, panelIds });
          if (delegate) {
            const session = orchestrationSessionRef.current;
            if (!session) return;
            dispatchPrompt({ type: "sending", panelId: MAIN_AGENT_NODE_ID });
            try {
              await delegateOrchestrationGoal({
                requestId,
                goal: message,
                expectedRevision: session.revision,
              });
              dispatchPrompt({ type: "succeeded", panelId: MAIN_AGENT_NODE_ID });
            } catch (error) {
              // Show the parsed reason instead of the raw payload, and rethrow so the
              // Composer can restore the text and render the next action (FR-022, FR-048).
              const guidance = describeOrchestrationFailure(parseOrchestrationError(error));
              dispatchPrompt({
                type: "failed",
                panelId: MAIN_AGENT_NODE_ID,
                error: guidance.nextAction
                  ? `${guidance.reason} ${guidance.nextAction}`
                  : guidance.reason,
              });
              throw error;
            }
            return;
          }
          const session = orchestrationSessionRef.current;
          if (!session) return;
          try {
            const result = await dispatchOrchestrationPrompt({
              requestId,
              intent: "direct",
              targetMode: mode,
              message,
              delivery: "send",
              panelIds,
              expectedRevision: session.revision,
            });
            let next = stateRef.current;
            for (const panelId of panelIds) {
              const node = session.nodes.find((candidate) => candidate.id === panelId);
              const target = result.targets.find((candidate) => candidate.panelId === panelId);
              if (node?.kind === "child") {
                if (target?.status === "delivered") {
                  dispatchPrompt({ type: "succeeded", panelId });
                } else {
                  dispatchPrompt({
                    type: "failed",
                    panelId,
                    error:
                      target?.failureReason ??
                      "Child runtime이 명령을 수락하지 않았습니다.",
                  });
                }
                continue;
              }
              dispatchPrompt({ type: "sending", panelId });
              const routed = routePromptToPanel(next, panelId, {
                id: `${requestId}:${panelId}`,
                text: message,
                delivery: "send",
              });
              if (routed.routed) {
                next = routed.state;
                dispatchPrompt({ type: "succeeded", panelId });
              } else {
                dispatchPrompt({
                  type: "failed",
                  panelId,
                  error: routed.reason,
                });
              }
            }
            stateRef.current = next;
            setState(next);
          } catch (error) {
            for (const panelId of panelIds) {
              dispatchPrompt({ type: "failed", panelId, error: String(error) });
            }
            return;
          }
        }}
      />
      <PromptDispatchStatus
        state={dispatchState}
        onRetry={(panelId) => {
          const session = orchestrationSessionRef.current;
          const message = lastDirectPromptRef.current;
          const node = session?.nodes.find((candidate) => candidate.id === panelId);
          const task = node?.assignedTaskId
            ? session?.tasks.find((candidate) => candidate.id === node.assignedTaskId)
            : null;
          if (!session || !message || node?.kind !== "child" || !task) {
            dispatchPrompt({
              type: "failed",
              panelId,
              error: "재시도할 Child command 정보를 찾을 수 없습니다.",
            });
            return;
          }
          dispatchPrompt({ type: "sending", panelId });
          void sendOrchestrationChildCommand({
            requestId: crypto.randomUUID(),
            taskId: task.id,
            kind: "message",
            message,
            delivery: "send",
            expectedTaskRevision: task.revision,
          })
            .then((command) => {
              if (command.status === "accepted") {
                dispatchPrompt({ type: "succeeded", panelId });
              } else {
                dispatchPrompt({
                  type: "failed",
                  panelId,
                  error:
                    command.failure?.message ??
                    "Child runtime이 재시도 명령을 수락하지 않았습니다.",
                });
              }
            })
            .catch((error) =>
              dispatchPrompt({
                type: "failed",
                panelId,
                error: String(error),
              }),
            );
        }}
      />

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

      <CoordinatorHandoffDialog
        open={Boolean(pendingMainHandoffRunId)}
        previousRunId={
          orchestrationSession?.generations.find(
            (generation) =>
              generation.id === orchestrationSession.activeCoordinatorGenerationId,
          )?.runId ?? "unknown"
        }
        successorRunId={pendingMainHandoffRunId ?? ""}
        onOpenChange={(open) => {
          if (!open) {
            const pending = pendingMainStartRef.current;
            pendingMainStartRef.current = null;
            setPendingMainHandoffRunId(null);
            pending?.reject(new Error("Main Coordinator 인계가 취소되었습니다."));
          }
        }}
        onConfirm={async (summary) => {
          const session = orchestrationSessionRef.current;
          const successorRunId = pendingMainHandoffRunId;
          const pending = pendingMainStartRef.current;
          if (!session || !successorRunId) return;
          try {
            const next = await handoffOrchestrationCoordinator({
              requestId: crypto.randomUUID(),
              successorRunId,
              summary,
              confirmed: true,
              expectedRevision: session.revision,
            });
            boundMainRunIdRef.current = successorRunId;
            orchestrationSessionRef.current = next;
            setOrchestrationSession(next);
            pendingMainStartRef.current = null;
            setPendingMainHandoffRunId(null);
            if (pending?.runId === successorRunId) {
              pending.resolve();
            }
          } catch (error) {
            setTargetMessage(`Main Coordinator 인계 실패: ${String(error)}`);
          }
        }}
      />
    </div>
  );
}
