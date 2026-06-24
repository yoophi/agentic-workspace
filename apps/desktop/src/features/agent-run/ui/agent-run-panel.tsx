import type { PointerEventHandler, ReactNode } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  BotIcon,
  CheckCircleIcon,
  ClockIcon,
  Loader2Icon,
  PencilIcon,
  PlayIcon,
  SettingsIcon,
  SquareIcon,
  XCircleIcon,
  XIcon,
} from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import {
  cancelAgentRun,
  listenRunEvents,
  listAgents,
  listProviderSessions,
  respondAgentPermission,
  sendPromptToRun,
  startAgentRun,
} from "@/entities/agent-run/api/agent-run-repository";
import { agentRunQueryKeys } from "@/entities/agent-run/api/query-keys";
import {
  appendOneTimelineItem,
  eventGroups,
  toTimelineItem,
} from "@/entities/agent-run/model";
import type { TimelineRunEvent } from "@/entities/agent-run/model";
import type {
  ContextSizePreset,
  EventGroup,
  PermissionMode,
  ProviderSession,
  RunEvent,
  TimelineItem,
} from "@/entities/agent-run/model";
import {
  addUserMessage,
  buildSteerPrompt,
  moveQueuedPrompt as reorderQueuedPrompt,
  removeUserMessage,
  updateQueuedPrompt,
} from "@/features/agent-run/model/run-panel-state";
import type { QueuedPrompt, UsageContext } from "@/features/agent-run/model/run-panel-state";
import { formatSessionLabel } from "@/features/agent-run/model/session-label";
import { SavedPromptToolbar } from "@/features/saved-prompt/ui/saved-prompt-toolbar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { CodeBlock, CodeBlockCode } from "@/components/ui/code-block";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Message, MessageAvatar, MessageContent } from "@/components/ui/message";
import {
  PromptInput,
  PromptInputAction,
  PromptInputActions,
  PromptInputTextarea,
} from "@/components/ui/prompt-input";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { Steps, StepsContent, StepsItem, StepsTrigger } from "@/components/ui/steps";
import { SystemMessage } from "@/components/ui/system-message";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";
import { ExternalLink } from "@/shared/ui/external-link";

type AgentRunPanelProps = {
  workingDirectory: string;
  scrollHeader?: ReactNode;
};

const defaultPrompt = "";
// 백엔드(MAX_RALPH_ITERATIONS)와 맞춘 자동 반복 상한. 입력은 이 값으로 제한된다.
const RALPH_MAX_ITERATIONS = 100;
const RALPH_DEFAULT_PROMPT =
  "이전 결과를 바탕으로 목표를 계속 진행하세요. 목표를 모두 달성했다면 더 진행하지 말고 완료를 알려주세요.";
const PROMPT_PANEL_DEFAULT_HEIGHT = 300;
const PROMPT_PANEL_MIN_HEIGHT = 180;
const PROMPT_PANEL_MAX_HEIGHT = 560;
const TIMELINE_ESTIMATED_ITEM_HEIGHT = 96;
const TIMELINE_ITEM_GAP = 12;
const TIMELINE_OVERSCAN = 6;

const permissionModeOptions: Array<{
  value: PermissionMode;
  label: string;
  description: string;
}> = [
  {
    value: "default",
    label: "Default",
    description: "Use the agent's normal approval behavior.",
  },
  {
    value: "auto",
    label: "Auto",
    description: "Use automatic approval mode when the agent supports it.",
  },
  {
    value: "readOnly",
    label: "Read-only",
    description: "Prefer analysis without unapproved edits.",
  },
  {
    value: "plan",
    label: "Plan",
    description: "Prefer planning or read-only behavior before edits.",
  },
  {
    value: "acceptEdits",
    label: "Accept edits",
    description: "Allow supported agents to edit files without each edit prompt.",
  },
  {
    value: "dangerouslySkipAllPermissions",
    label: "Danger full access",
    description: "Use only in isolated workspaces.",
  },
];

const modelOptions = [
  {
    value: "providerDefault",
    label: "Provider default",
    description: "Use the agent/provider default model.",
  },
  {
    value: "gpt-5",
    label: "GPT-5",
    description: "Use GPT-5 when the selected provider advertises it.",
  },
  {
    value: "gpt-5-codex",
    label: "GPT-5 Codex",
    description: "Use GPT-5 Codex when the selected provider advertises it.",
  },
  {
    value: "claude-sonnet-4-5",
    label: "Claude Sonnet 4.5",
    description: "Use Claude Sonnet 4.5 when the selected provider advertises it.",
  },
] as const;

const contextSizeOptions: Array<{
  value: ContextSizePreset;
  label: string;
  description: string;
}> = [
  {
    value: "default",
    label: "Default context",
    description: "Use the agent/provider default context size.",
  },
  {
    value: "medium",
    label: "Medium",
    description: "Prefer a balanced context window when supported.",
  },
  {
    value: "large",
    label: "Large",
    description: "Prefer a larger context window when supported.",
  },
  {
    value: "xLarge",
    label: "XL",
    description: "Prefer the largest common context window when supported.",
  },
];

export function AgentRunPanel({ workingDirectory, scrollHeader }: AgentRunPanelProps) {
  const [selectedAgentId, setSelectedAgentId] = useState<string>("");
  const [sessionMode, setSessionMode] = useState<"new" | "reuse">("new");
  const [selectedSessionId, setSelectedSessionId] = useState<string>("");
  const [permissionMode, setPermissionMode] = useState<PermissionMode>("default");
  const [modelId, setModelId] = useState<(typeof modelOptions)[number]["value"]>(
    "providerDefault",
  );
  const [contextSize, setContextSize] = useState<ContextSizePreset>("default");
  const [ralphLoopEnabled, setRalphLoopEnabled] = useState(false);
  const [ralphMaxIterations, setRalphMaxIterations] = useState(5);
  const [ralphDelaySeconds, setRalphDelaySeconds] = useState(0);
  const [ralphStopOnError, setRalphStopOnError] = useState(true);
  const [ralphPromptTemplate, setRalphPromptTemplate] = useState(RALPH_DEFAULT_PROMPT);
  const [prompt, setPrompt] = useState(defaultPrompt);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [isAwaitingPromptResponse, setIsAwaitingPromptResponse] = useState(false);
  const [directPrompt, setDirectPrompt] = useState<string | null>(null);
  const [queuedPrompts, setQueuedPrompts] = useState<QueuedPrompt[]>([]);
  const [filter, setFilter] = useState<EventGroup | "all">("all");
  const [items, setItems] = useState<TimelineItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [editingPrompt, setEditingPrompt] = useState<QueuedPrompt | null>(null);
  const [editingPromptText, setEditingPromptText] = useState("");
  const [usageContext, setUsageContext] = useState<UsageContext | null>(null);
  const [promptPanelHeight, setPromptPanelHeight] = useState(PROMPT_PANEL_DEFAULT_HEIGHT);
  const activeRunIdRef = useRef<string | null>(null);
  const promptResizeRef = useRef<{
    pointerId: number;
    startY: number;
    startHeight: number;
  } | null>(null);

  const agentsQuery = useQuery({
    queryKey: agentRunQueryKeys.agents,
    queryFn: listAgents,
  });
  const agents = agentsQuery.data ?? [];

  const sessionsQuery = useQuery({
    queryKey: agentRunQueryKeys.sessions(selectedAgentId, workingDirectory),
    queryFn: () => listProviderSessions(selectedAgentId, workingDirectory),
    enabled: sessionMode === "reuse" && Boolean(selectedAgentId),
  });
  const sessions = sessionsQuery.data ?? [];

  useEffect(() => {
    if (!selectedAgentId && agents[0]) {
      setSelectedAgentId(agents[0].id);
    }
  }, [agents, selectedAgentId]);

  // agent를 바꾸면 이전 provider의 세션 선택은 더 이상 유효하지 않다.
  useEffect(() => {
    setSelectedSessionId("");
  }, [selectedAgentId]);

  useEffect(() => {
    activeRunIdRef.current = activeRunId;
  }, [activeRunId]);

  useEffect(() => {
    return () => {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, []);

  useEffect(() => {
    const unlisten = listenRunEvents((envelope) => {
      if (envelope.runId !== activeRunIdRef.current) {
        return;
      }

      if (envelope.event.type === "usage") {
        setUsageContext({ used: envelope.event.used, size: envelope.event.size });
        return;
      }

      const timelineEvent: TimelineRunEvent = envelope.event;
      setItems((currentItems) =>
        addRunEventItem(currentItems, envelope.runId, timelineEvent),
      );

      if (timelineEvent.type === "error") {
        setIsAwaitingPromptResponse(false);
        setQueuedPrompts([]);
        setDirectPrompt(null);
        setIsRunning(false);
        activeRunIdRef.current = null;
        setActiveRunId(null);
        return;
      }

      if (timelineEvent.type === "lifecycle") {
        if (timelineEvent.status === "promptSent") {
          setIsAwaitingPromptResponse(true);
        }
        if (timelineEvent.status === "promptCompleted") {
          setIsAwaitingPromptResponse(false);
        }
        if (["completed", "cancelled"].includes(timelineEvent.status)) {
          setIsAwaitingPromptResponse(false);
          setQueuedPrompts([]);
          setDirectPrompt(null);
          setIsRunning(false);
          activeRunIdRef.current = null;
          setActiveRunId(null);
        }
      }
    });

    return () => {
      unlisten();
    };
  }, []);

  useEffect(() => {
    if (!activeRunId || !isRunning || isAwaitingPromptResponse || queuedPrompts.length === 0) {
      return;
    }

    const nextPrompt = queuedPrompts[0];
    setIsAwaitingPromptResponse(true);
    setQueuedPrompts((current) => current.slice(1));
    setItems((currentItems) =>
      addUserMessage(currentItems, activeRunId, nextPrompt.text),
    );
    void sendPromptToRun(activeRunId, nextPrompt.text).catch((caughtError) => {
      setQueuedPrompts((current) => [nextPrompt, ...current]);
      setItems((currentItems) =>
        removeUserMessage(currentItems, activeRunId, nextPrompt.text),
      );
      setIsAwaitingPromptResponse(false);
      setError(String(caughtError));
    });
  }, [activeRunId, isAwaitingPromptResponse, isRunning, queuedPrompts]);

  const selectedAgent = agents.find((agent) => agent.id === selectedAgentId);
  const visibleItems = useMemo(
    () => (filter === "all" ? items : items.filter((item) => item.group === filter)),
    [filter, items],
  );
  const usagePercent =
    usageContext && usageContext.size > 0
      ? Math.min(100, Math.round((usageContext.used / usageContext.size) * 100))
      : null;
  const pendingPermission = useMemo(() => findPendingPermission(items), [items]);
  const canStartRun = Boolean(
    selectedAgentId &&
      prompt.trim() &&
      !isRunning &&
      (sessionMode === "new" || selectedSessionId),
  );
  const canQueuePrompt = Boolean(activeRunId && isRunning && prompt.trim());
  const canSteerPrompt = Boolean(
    activeRunId && isRunning && directPrompt?.trim() && prompt.trim(),
  );
  const canCancel = Boolean(activeRunId && isRunning);

  async function run() {
    const goal = prompt.trim();
    const started = await startRun(goal);
    if (started) {
      setPrompt(defaultPrompt);
    }
  }

  async function startRun(goal: string) {
    if (!selectedAgentId || !goal) {
      return false;
    }

    const runId = crypto.randomUUID();
    setError(null);
    setItems([]);
    setQueuedPrompts([]);
    setUsageContext(null);
    setDirectPrompt(goal);
    activeRunIdRef.current = runId;
    setActiveRunId(runId);
    setIsRunning(true);
    setIsAwaitingPromptResponse(true);
    setItems((currentItems) => addUserMessage(currentItems, runId, goal));

    const reuseSession = sessionMode === "reuse" && Boolean(selectedSessionId);

    try {
      await startAgentRun({
        runId,
        goal,
        agentId: selectedAgentId,
        cwd: workingDirectory,
        stdioBufferLimitMb: 50,
        permissionMode,
        ...(modelId !== "providerDefault" ? { modelId } : {}),
        ...(contextSize !== "default" ? { contextSize } : {}),
        ...(reuseSession
          ? { resumeSessionId: selectedSessionId, resumePolicy: "resumeIfAvailable" }
          : {}),
        ...(ralphLoopEnabled
          ? {
              ralphLoop: {
                enabled: true,
                maxIterations: ralphMaxIterations,
                promptTemplate: ralphPromptTemplate,
                stopOnError: ralphStopOnError,
                stopOnPermission: false,
                delayMs: Math.max(0, Math.round(ralphDelaySeconds * 1000)),
              },
            }
          : {}),
      });
      return true;
    } catch (caughtError) {
      setError(String(caughtError));
      setPrompt(goal);
      setItems((currentItems) => removeUserMessage(currentItems, runId, goal));
      setDirectPrompt(null);
      setIsAwaitingPromptResponse(false);
      setIsRunning(false);
      activeRunIdRef.current = null;
      setActiveRunId(null);
      return false;
    }
  }

  function enqueuePrompt(promptText = prompt) {
    const nextPrompt = promptText.trim();
    if (!nextPrompt) {
      return;
    }

    setQueuedPrompts((current) => [...current, { id: crypto.randomUUID(), text: nextPrompt }]);
    if (promptText === prompt) {
      setPrompt(defaultPrompt);
    }
  }

  async function sendSavedPrompt(savedPrompt: string) {
    const nextPrompt = savedPrompt.trim();
    if (!nextPrompt) {
      return;
    }

    if (isRunning) {
      enqueuePrompt(nextPrompt);
      return;
    }

    await startRun(nextPrompt);
  }

  function moveQueuedPrompt(fromIndex: number, toIndex: number) {
    setQueuedPrompts((current) => reorderQueuedPrompt(current, fromIndex, toIndex));
  }

  function openQueuedPromptEditor(queuedPrompt: QueuedPrompt) {
    setEditingPrompt(queuedPrompt);
    setEditingPromptText(queuedPrompt.text);
  }

  function closeQueuedPromptEditor() {
    setEditingPrompt(null);
    setEditingPromptText("");
  }

  function saveQueuedPromptEdit() {
    const nextText = editingPromptText.trim();
    if (!editingPrompt || !nextText) {
      return;
    }

    setQueuedPrompts((current) => {
      const result = updateQueuedPrompt(current, editingPrompt.id, nextText);
      if (!result.updated) {
        setError("편집하려던 prompt가 이미 전송되었거나 queue에서 제거되었습니다.");
      }
      return result.queue;
    });
    closeQueuedPromptEditor();
  }

  async function cancel() {
    if (!activeRunId) {
      return;
    }

    try {
      await cancelAgentRun(activeRunId);
    } catch (caughtError) {
      setError(String(caughtError));
    } finally {
      setQueuedPrompts([]);
      setDirectPrompt(null);
      setIsAwaitingPromptResponse(false);
      setIsRunning(false);
      activeRunIdRef.current = null;
      setActiveRunId(null);
    }
  }

  async function steer() {
    const steerPrompt = prompt.trim();
    const originalPrompt = directPrompt?.trim();
    const runIdToCancel = activeRunId;

    if (!runIdToCancel || !originalPrompt || !steerPrompt) {
      return;
    }

    const nextGoal = buildSteerPrompt(originalPrompt, steerPrompt);
    setError(null);
    setPrompt(defaultPrompt);
    setQueuedPrompts([]);

    try {
      await cancelAgentRun(runIdToCancel);
      const started = await startRun(nextGoal);
      if (!started) {
        setPrompt(steerPrompt);
      }
    } catch (caughtError) {
      setError(String(caughtError));
      setPrompt(steerPrompt);
    }
  }

  async function respondToPermission(permissionId: string, optionId: string) {
    if (!activeRunId) {
      setError("응답할 active run이 없습니다.");
      throw new Error("응답할 active run이 없습니다.");
    }

    try {
      await respondAgentPermission(activeRunId, permissionId, optionId);
    } catch (caughtError) {
      setError(String(caughtError));
      throw caughtError;
    }
  }

  const startPromptResize: PointerEventHandler<HTMLDivElement> = (event) => {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();
    promptResizeRef.current = {
      pointerId: event.pointerId,
      startY: event.clientY,
      startHeight: promptPanelHeight,
    };
    event.currentTarget.setPointerCapture(event.pointerId);
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
  };

  const resizePromptPanel: PointerEventHandler<HTMLDivElement> = (event) => {
    const resizeState = promptResizeRef.current;
    if (!resizeState || resizeState.pointerId !== event.pointerId) {
      return;
    }

    const nextHeight = resizeState.startHeight + resizeState.startY - event.clientY;
    setPromptPanelHeight(clampPromptPanelHeight(nextHeight));
  };

  const stopPromptResize: PointerEventHandler<HTMLDivElement> = (event) => {
    const resizeState = promptResizeRef.current;
    if (!resizeState || resizeState.pointerId !== event.pointerId) {
      return;
    }

    promptResizeRef.current = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-4">
      <div className="min-h-0 flex-1 overflow-auto pr-1">
        <div className="flex flex-col gap-4">
          {scrollHeader}

          <Card>
            <CardHeader>
              <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                <div className="flex flex-col gap-1.5">
                  <CardTitle className="flex items-center gap-2">
                    <BotIcon />
                    Agentic coding
                  </CardTitle>
                  <CardDescription>
                    선택한 worktree를 작업 디렉토리로 사용해 ACP agent를 실행합니다.
                  </CardDescription>
                </div>
                <div className="flex flex-col gap-3">
                  <div className="flex flex-col gap-1.5">
                    <span className="text-sm font-medium">Agent</span>
                    <Select
                      value={selectedAgentId}
                      onValueChange={setSelectedAgentId}
                      disabled={isRunning || agentsQuery.isLoading}
                    >
                      <SelectTrigger className="w-full sm:w-56">
                        <SelectValue placeholder="Agent 선택" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectGroup>
                          {agents.map((agent) => (
                            <SelectItem key={agent.id} value={agent.id}>
                              {agent.label}
                            </SelectItem>
                          ))}
                        </SelectGroup>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <span className="text-sm font-medium">세션</span>
                    <div className="flex gap-1.5">
                      <Button
                        type="button"
                        size="sm"
                        variant={sessionMode === "new" ? "default" : "outline"}
                        disabled={isRunning}
                        onClick={() => setSessionMode("new")}
                      >
                        새 세션
                      </Button>
                      <Button
                        type="button"
                        size="sm"
                        variant={sessionMode === "reuse" ? "default" : "outline"}
                        disabled={isRunning}
                        onClick={() => setSessionMode("reuse")}
                      >
                        기존 세션 재사용
                      </Button>
                    </div>
                    {sessionMode === "reuse" && (
                      <Select
                        value={selectedSessionId}
                        onValueChange={setSelectedSessionId}
                        disabled={
                          isRunning ||
                          sessionsQuery.isLoading ||
                          sessionsQuery.isError ||
                          sessions.length === 0
                        }
                      >
                        <SelectTrigger className="w-full sm:w-56">
                          <SelectValue
                            placeholder={
                              sessionsQuery.isLoading
                                ? "세션 불러오는 중…"
                                : sessionsQuery.isError
                                  ? "세션을 불러오지 못함"
                                  : sessions.length === 0
                                    ? "재사용 가능한 세션 없음"
                                    : "재개할 세션 선택"
                            }
                          />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectGroup>
                            {sessions.map((session) => (
                              <SelectItem key={session.id} value={session.id}>
                                {formatSessionLabel(session)}
                              </SelectItem>
                            ))}
                          </SelectGroup>
                        </SelectContent>
                      </Select>
                    )}
                    {sessionMode === "reuse" &&
                      !sessionsQuery.isLoading &&
                      (sessionsQuery.isError ? (
                        <span className="text-xs text-destructive">
                          세션 목록을 불러오지 못했습니다: {String(sessionsQuery.error)}
                        </span>
                      ) : sessions.length === 0 ? (
                        <span className="text-xs text-muted-foreground">
                          이 worktree에서 해당 agent의 기존 세션을 찾지 못했습니다.
                        </span>
                      ) : null)}
                  </div>
                </div>
              </div>
            </CardHeader>
            <CardContent className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge variant={isRunning ? "default" : "secondary"}>
                    {isRunning ? "Running" : "Idle"}
                  </Badge>
                  <EllipsisPopoverText
                    value={`cwd=${workingDirectory}`}
                    className="max-w-full font-mono text-xs text-muted-foreground"
                    contentClassName="font-mono text-xs"
                  />
                </div>
                {selectedAgent && (
                  <EllipsisPopoverText
                    value={`command=${selectedAgent.command}`}
                    className="font-mono text-xs text-muted-foreground"
                    contentClassName="font-mono text-xs"
                  />
                )}
                {error && (
                  <SystemMessage variant="error" fill>
                    {error}
                  </SystemMessage>
                )}
              </div>

              <div className="flex flex-col rounded-lg border bg-background">
                <div className="flex flex-wrap gap-1.5 border-b p-3" role="tablist" aria-label="ACP event filter">
                  {eventGroups.map((group) => (
                    <Button
                      key={group.id}
                      type="button"
                      size="sm"
                      variant={filter === group.id ? "default" : "outline"}
                      onClick={() => setFilter(group.id)}
                    >
                      {group.label}
                    </Button>
                  ))}
                </div>
                <VirtualizedRunTimeline items={visibleItems} />
              </div>
            </CardContent>
          </Card>
        </div>
      </div>

      {usageContext && (
        <div className="shrink-0 rounded-lg border bg-background px-3 py-2">
          <div className="flex items-center justify-between gap-3 text-xs">
            <span className="font-medium text-muted-foreground">Context</span>
            <span className="font-mono text-muted-foreground">
              {usageContext.used}/{usageContext.size}
              {usagePercent !== null ? ` (${usagePercent}%)` : ""}
            </span>
          </div>
          <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-[width]"
              style={{ width: `${usagePercent ?? 0}%` }}
            />
          </div>
        </div>
      )}

      <div className="relative shrink-0" style={{ height: promptPanelHeight }}>
        <div
          role="separator"
          aria-label="프롬프트 영역 크기 조정"
          aria-orientation="horizontal"
          className="group absolute -top-2 left-0 right-0 z-10 flex h-4 cursor-ns-resize items-center justify-center touch-none"
          onPointerDown={startPromptResize}
          onPointerMove={resizePromptPanel}
          onPointerUp={stopPromptResize}
          onPointerCancel={stopPromptResize}
        >
          <div className="h-1 w-12 rounded-full bg-border transition-colors group-hover:bg-muted-foreground/60 group-active:bg-primary" />
        </div>
        <PromptInput
          value={prompt}
          onValueChange={setPrompt}
          onSubmit={() => {
            if (isRunning) {
              enqueuePrompt();
              return;
            }
            if (canStartRun) {
              void run();
            }
          }}
          isLoading={isRunning}
          className="flex h-full min-h-0 flex-col overflow-hidden rounded-lg"
        >
          <div className="flex shrink-0 flex-wrap items-center gap-2 border-b px-2 py-2">
            <span className="text-xs font-medium text-muted-foreground">Permission mode</span>
            <Select
              value={permissionMode}
              onValueChange={(value) => setPermissionMode(value as PermissionMode)}
              disabled={isRunning}
            >
              <SelectTrigger className="h-8 w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {permissionModeOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <span className="min-w-0 flex-1 text-xs text-muted-foreground">
              {permissionModeOptions.find((option) => option.value === permissionMode)?.description}
            </span>
          </div>
          <div className="flex shrink-0 flex-wrap items-center gap-2 border-b px-2 py-2">
            <span className="text-xs font-medium text-muted-foreground">Model</span>
            <Select
              value={modelId}
              onValueChange={(value) =>
                setModelId(value as (typeof modelOptions)[number]["value"])
              }
              disabled={isRunning}
            >
              <SelectTrigger className="h-8 w-52">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {modelOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <span className="text-xs font-medium text-muted-foreground">Context</span>
            <Select
              value={contextSize}
              onValueChange={(value) => setContextSize(value as ContextSizePreset)}
              disabled={isRunning}
            >
              <SelectTrigger className="h-8 w-44">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {contextSizeOptions.map((option) => (
                    <SelectItem key={option.value} value={option.value}>
                      {option.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <span className="min-w-0 flex-1 text-xs text-muted-foreground">
              {modelOptions.find((option) => option.value === modelId)?.description}{" "}
              {contextSizeOptions.find((option) => option.value === contextSize)?.description}
            </span>
          </div>
          <div className="flex shrink-0 flex-col gap-2 border-b px-2 py-2">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-xs font-medium text-muted-foreground">Ralph loop</span>
              <Button
                type="button"
                size="sm"
                variant={ralphLoopEnabled ? "default" : "outline"}
                disabled={isRunning}
                onClick={() => setRalphLoopEnabled((value) => !value)}
              >
                {ralphLoopEnabled ? "On" : "Off"}
              </Button>
              <span className="min-w-0 flex-1 text-xs text-muted-foreground">
                목표가 끝날 때까지 loop prompt를 자동 반복 실행합니다. (최대 {RALPH_MAX_ITERATIONS}회)
              </span>
            </div>
            {ralphLoopEnabled && (
              <div className="flex flex-col gap-2">
                <div className="flex flex-wrap items-center gap-3">
                  <label className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    반복 횟수
                    <Input
                      type="number"
                      min={1}
                      max={RALPH_MAX_ITERATIONS}
                      value={ralphMaxIterations}
                      disabled={isRunning}
                      className="h-8 w-20"
                      onChange={(event) => {
                        const next = Number(event.target.value);
                        if (Number.isFinite(next)) {
                          setRalphMaxIterations(
                            Math.min(RALPH_MAX_ITERATIONS, Math.max(1, Math.round(next))),
                          );
                        }
                      }}
                    />
                  </label>
                  <label className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    반복 간 지연(초)
                    <Input
                      type="number"
                      min={0}
                      value={ralphDelaySeconds}
                      disabled={isRunning}
                      className="h-8 w-20"
                      onChange={(event) => {
                        const next = Number(event.target.value);
                        if (Number.isFinite(next)) {
                          setRalphDelaySeconds(Math.max(0, next));
                        }
                      }}
                    />
                  </label>
                  <Button
                    type="button"
                    size="sm"
                    variant={ralphStopOnError ? "default" : "outline"}
                    disabled={isRunning}
                    onClick={() => setRalphStopOnError((value) => !value)}
                  >
                    오류 시 중단: {ralphStopOnError ? "On" : "Off"}
                  </Button>
                </div>
                <Textarea
                  value={ralphPromptTemplate}
                  disabled={isRunning}
                  placeholder="반복마다 agent에게 보낼 loop prompt"
                  className="min-h-16 text-sm"
                  onChange={(event) => setRalphPromptTemplate(event.target.value)}
                />
              </div>
            )}
          </div>
          <SavedPromptToolbar
            disabled={!selectedAgentId}
            onSendPrompt={(savedPrompt) => void sendSavedPrompt(savedPrompt)}
          />
          <div className="min-h-0 flex-1">
            <PromptInputTextarea
              disableAutosize
              placeholder="선택한 worktree에서 실행할 작업을 입력하세요."
              className="h-full min-h-0 resize-none overflow-auto"
            />
          </div>
          {queuedPrompts.length > 0 && (
            <div className="flex max-h-36 shrink-0 flex-col gap-2 px-2 pb-2">
              <div className="text-xs text-muted-foreground">대기 중인 prompt {queuedPrompts.length}개</div>
              <div className="flex min-h-0 flex-col gap-2 overflow-auto pr-1">
                {queuedPrompts.map((queuedPrompt, index) => (
                  <div
                    key={queuedPrompt.id}
                    className="flex items-start justify-between gap-3 rounded-md border bg-muted/40 px-3 py-2"
                  >
                    <div className="min-w-0 flex-1 text-sm">
                      <span className="mr-2 text-xs text-muted-foreground">#{index + 1}</span>
                      <span className="whitespace-pre-wrap break-words">{queuedPrompt.text}</span>
                    </div>
                    <div className="flex shrink-0 items-center gap-1">
                      <PromptInputAction tooltip="Edit prompt" side="left">
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="size-7"
                          aria-label={`${index + 1}번 prompt 편집`}
                          onClick={() => openQueuedPromptEditor(queuedPrompt)}
                        >
                          <PencilIcon className="size-4" />
                        </Button>
                      </PromptInputAction>
                      <PromptInputAction tooltip="Move up" side="left">
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="size-7"
                          disabled={index === 0}
                          aria-label={`${index + 1}번 prompt 위로 이동`}
                          onClick={() => moveQueuedPrompt(index, index - 1)}
                        >
                          <ArrowUpIcon className="size-4" />
                        </Button>
                      </PromptInputAction>
                      <PromptInputAction tooltip="Move down" side="left">
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="size-7"
                          disabled={index === queuedPrompts.length - 1}
                          aria-label={`${index + 1}번 prompt 아래로 이동`}
                          onClick={() => moveQueuedPrompt(index, index + 1)}
                        >
                          <ArrowDownIcon className="size-4" />
                        </Button>
                      </PromptInputAction>
                      <PromptInputAction tooltip="Remove prompt" side="left">
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="size-7"
                          aria-label={`${index + 1}번 prompt 제거`}
                          onClick={() => {
                            setQueuedPrompts((current) =>
                              current.filter((item) => item.id !== queuedPrompt.id),
                            );
                          }}
                        >
                          <XIcon className="size-4" />
                        </Button>
                      </PromptInputAction>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
          <div className="flex shrink-0 flex-col gap-3 px-2 pb-1 sm:flex-row sm:items-center sm:justify-between">
            <span className="min-w-0 text-xs text-muted-foreground">
              {isRunning
                ? isAwaitingPromptResponse
                  ? "Enter는 queue에 추가합니다. Steer는 현재 실행을 중단하고 새 지시로 재실행합니다."
                  : "Enter는 queue에 추가합니다. Steer는 queue와 별도로 현재 실행을 재지시합니다."
                : "Enter로 실행, Shift+Enter로 줄바꿈"}
            </span>
            <PromptInputActions className="justify-end">
              {isRunning ? (
                <>
                  <PromptInputAction tooltip="Queue prompt">
                    <Button
                      type="button"
                      size="sm"
                      disabled={!canQueuePrompt}
                      onClick={() => enqueuePrompt()}
                    >
                      <PlayIcon data-icon="inline-start" />
                      Queue
                    </Button>
                  </PromptInputAction>
                  <PromptInputAction tooltip="Cancel and steer">
                    <Button
                      type="button"
                      size="sm"
                      variant="secondary"
                      disabled={!canSteerPrompt}
                      onClick={() => void steer()}
                    >
                      <PencilIcon data-icon="inline-start" />
                      Steer
                    </Button>
                  </PromptInputAction>
                  <PromptInputAction tooltip="Cancel run">
                    <Button
                      type="button"
                      variant="destructive"
                      size="sm"
                      disabled={!canCancel}
                      onClick={() => void cancel()}
                    >
                      <SquareIcon data-icon="inline-start" />
                      Cancel
                    </Button>
                  </PromptInputAction>
                </>
              ) : (
                <PromptInputAction tooltip="Start run">
                  <Button type="button" size="sm" disabled={!canStartRun} onClick={() => void run()}>
                    <PlayIcon data-icon="inline-start" />
                    Run
                  </Button>
                </PromptInputAction>
              )}
            </PromptInputActions>
          </div>
        </PromptInput>
      </div>

      <Dialog
        open={Boolean(editingPrompt)}
        onOpenChange={(open) => {
          if (!open) {
            closeQueuedPromptEditor();
          }
        }}
      >
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>Prompt 편집</DialogTitle>
            <DialogDescription>
              Queue에 대기 중인 prompt 내용을 수정합니다.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-2">
            <Textarea
              value={editingPromptText}
              onChange={(event) => setEditingPromptText(event.target.value)}
              className="max-h-[50svh] min-h-48 resize-y font-mono text-sm"
              placeholder="Queue에 저장할 prompt를 입력하세요."
              autoFocus
            />
            <span className="text-xs text-muted-foreground">
              저장하면 현재 queue 항목만 갱신됩니다.
            </span>
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                취소
              </Button>
            </DialogClose>
            <Button
              type="button"
              disabled={!editingPromptText.trim()}
              onClick={saveQueuedPromptEdit}
            >
              저장
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <PermissionRequestDialog
        permission={pendingPermission}
        onSelect={respondToPermission}
      />
    </div>
  );
}

function findPendingPermission(items: TimelineItem[]) {
  const pending = new Map<string, Extract<RunEvent, { type: "permission" }>>();
  for (const item of items) {
    if (item.event.type !== "permission" || !item.event.permissionId) {
      continue;
    }
    if (item.event.requiresResponse) {
      pending.set(item.event.permissionId, item.event);
    } else {
      pending.delete(item.event.permissionId);
    }
  }
  const pendingPermissions = Array.from(pending.values());
  return pendingPermissions[pendingPermissions.length - 1] ?? null;
}

function clampPromptPanelHeight(height: number) {
  return Math.min(PROMPT_PANEL_MAX_HEIGHT, Math.max(PROMPT_PANEL_MIN_HEIGHT, height));
}

function VirtualizedRunTimeline({ items }: { items: TimelineItem[] }) {
  const scrollRef = useRef<HTMLDivElement | null>(null);
  const stickToBottomRef = useRef(true);
  const [viewportHeight, setViewportHeight] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [measuredHeights, setMeasuredHeights] = useState<Record<string, number>>({});

  useEffect(() => {
    const scrollElement = scrollRef.current;
    if (!scrollElement) {
      return;
    }

    const resizeObserver = new ResizeObserver(([entry]) => {
      setViewportHeight(entry.contentRect.height);
    });
    resizeObserver.observe(scrollElement);
    setViewportHeight(scrollElement.clientHeight);

    return () => {
      resizeObserver.disconnect();
    };
  }, []);

  const itemLayouts = useMemo(() => {
    let offset = 0;
    return items.map((item) => {
      const height = measuredHeights[item.id] ?? TIMELINE_ESTIMATED_ITEM_HEIGHT;
      const layout = { item, height, start: offset, end: offset + height };
      offset += height + TIMELINE_ITEM_GAP;
      return layout;
    });
  }, [items, measuredHeights]);

  const totalHeight = itemLayouts.length
    ? itemLayouts[itemLayouts.length - 1].end
    : 0;

  const virtualItems = useMemo(() => {
    if (!itemLayouts.length) {
      return [];
    }

    const viewportBottom = scrollTop + viewportHeight;
    let startIndex = itemLayouts.findIndex((layout) => layout.end >= scrollTop);
    if (startIndex === -1) {
      startIndex = itemLayouts.length - 1;
    }
    startIndex = Math.max(0, startIndex - TIMELINE_OVERSCAN);

    let endIndex = startIndex;
    while (
      endIndex < itemLayouts.length - 1 &&
      itemLayouts[endIndex].start <= viewportBottom
    ) {
      endIndex += 1;
    }
    endIndex = Math.min(itemLayouts.length - 1, endIndex + TIMELINE_OVERSCAN);

    return itemLayouts.slice(startIndex, endIndex + 1);
  }, [itemLayouts, scrollTop, viewportHeight]);

  useEffect(() => {
    const scrollElement = scrollRef.current;
    if (!scrollElement || !stickToBottomRef.current) {
      return;
    }

    scrollElement.scrollTop = scrollElement.scrollHeight;
  }, [items.length, totalHeight]);

  const updateItemHeight = useCallback((itemId: string, height: number) => {
    setMeasuredHeights((current) => {
      if (current[itemId] === height) {
        return current;
      }
      return { ...current, [itemId]: height };
    });
  }, []);

  return (
    <div
      ref={scrollRef}
      className="max-h-[min(70svh,720px)] min-h-[320px] overflow-auto p-4"
      role="log"
      aria-live="polite"
      onScroll={(event) => {
        const element = event.currentTarget;
        const distanceFromBottom =
          element.scrollHeight - element.scrollTop - element.clientHeight;
        stickToBottomRef.current = distanceFromBottom < 48;
        setScrollTop(element.scrollTop);
      }}
    >
      {items.length === 0 ? (
        <div className="grid min-h-[320px] place-items-center rounded-lg border border-dashed bg-muted/30 text-sm text-muted-foreground">
          ACP 응답이 아직 없습니다.
        </div>
      ) : (
        <div className="relative" style={{ height: totalHeight }}>
          {virtualItems.map(({ item, start }) => (
            <MeasuredRunEventItem
              key={item.id}
              item={item}
              top={start}
              onHeightChange={updateItemHeight}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function MeasuredRunEventItem({
  item,
  top,
  onHeightChange,
}: {
  item: TimelineItem;
  top: number;
  onHeightChange: (itemId: string, height: number) => void;
}) {
  const itemRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const element = itemRef.current;
    if (!element) {
      return;
    }

    const measure = () => {
      onHeightChange(item.id, element.getBoundingClientRect().height);
    };
    measure();

    const resizeObserver = new ResizeObserver(measure);
    resizeObserver.observe(element);

    return () => {
      resizeObserver.disconnect();
    };
  }, [item.id, onHeightChange]);

  return (
    <div
      ref={itemRef}
      className="absolute left-0 right-0"
      style={{ transform: `translateY(${top}px)` }}
    >
      <RunEventItem item={item} />
    </div>
  );
}

function PermissionRequestDialog({
  permission,
  onSelect,
}: {
  permission: Extract<RunEvent, { type: "permission" }> | null;
  onSelect: (permissionId: string, optionId: string) => Promise<void>;
}) {
  const permissionId = permission?.permissionId;
  const [submittingOptionId, setSubmittingOptionId] = useState<string | null>(null);
  const submittingPermissionIdRef = useRef<string | null>(null);
  const isSubmitting =
    submittingPermissionIdRef.current === permissionId && submittingOptionId !== null;

  useEffect(() => {
    submittingPermissionIdRef.current = null;
    setSubmittingOptionId(null);
  }, [permissionId]);

  async function submitPermission(optionId: string) {
    if (!permissionId || submittingPermissionIdRef.current === permissionId) {
      return;
    }

    submittingPermissionIdRef.current = permissionId;
    setSubmittingOptionId(optionId);

    try {
      await onSelect(permissionId, optionId);
    } catch {
      submittingPermissionIdRef.current = null;
      setSubmittingOptionId(null);
    }
  }

  return (
    <Dialog open={Boolean(permissionId)}>
      <DialogContent showCloseButton={false} className="sm:max-w-xl">
        <DialogHeader>
          <DialogTitle>Permission required</DialogTitle>
          <DialogDescription>
            Agent가 작업을 계속하려면 아래 요청에 대한 결정을 선택해야 합니다.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3">
          <div className="rounded-md border bg-muted/40 p-3">
            <div className="text-sm font-medium">{permission?.title || "Tool request"}</div>
            {permission?.input !== undefined && (
              <pre className="mt-2 max-h-72 overflow-auto whitespace-pre-wrap break-words rounded border bg-background p-3 font-mono text-xs">
                {JSON.stringify(permission.input, null, 2)}
              </pre>
            )}
          </div>
        </div>
        <DialogFooter className="flex-wrap">
          {permission?.options.map((option) => (
            <Button
              key={option.optionId}
              type="button"
              variant={option.kind.startsWith("reject") ? "outline" : "default"}
              disabled={isSubmitting}
              onClick={() => void submitPermission(option.optionId)}
            >
              {submittingOptionId === option.optionId
                ? "Submitting..."
                : option.name || option.kind || option.optionId}
            </Button>
          ))}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function RunEventItem({ item }: { item: TimelineItem }) {
  if (item.group === "tool_call/tool_result") {
    return <ToolStep item={item} />;
  }

  if (item.event.type === "lifecycle") {
    return <LifecycleStep item={item} />;
  }

  if (item.group === "lifecycle" || item.group === "error" || item.group === "permission") {
    return (
      <SystemMessage
        variant={item.group === "error" ? "error" : item.tone === "warning" ? "warning" : "action"}
        fill={item.group === "error"}
      >
        <span className="font-medium">{item.title}</span>
        <span className="ml-2 break-words">{item.body}</span>
      </SystemMessage>
    );
  }

  if (item.group === "raw") {
    return (
      <CodeBlock className="rounded-lg">
        <CodeBlockCode code={item.body} language="json" />
      </CodeBlock>
    );
  }

  if (item.group === "user/message") {
    return (
      <Message className="justify-end">
        <MessageContent className="min-w-0 max-w-[80%] whitespace-pre-wrap break-words bg-primary text-primary-foreground">
          {item.body}
        </MessageContent>
      </Message>
    );
  }

  return (
    <Message className={cn(item.group === "thought" && "opacity-80")}>
      <MessageAvatar src="" alt={item.group} fallback={item.group === "assistant/message" ? "AI" : "•"} />
      <div className="min-w-0 flex-1 rounded-lg bg-secondary p-2 text-foreground">
        {item.group === "assistant/message" || item.group === "thought" ? (
          <StreamingMarkdown content={item.body} />
        ) : (
          <pre className="m-0 whitespace-pre-wrap break-words font-mono text-sm leading-6">
            {item.body}
          </pre>
        )}
      </div>
    </Message>
  );
}

function addRunEventItem(items: TimelineItem[], runId: string, event: TimelineRunEvent) {
  return appendOneTimelineItem(items, toTimelineItem(runId, event));
}

function LifecycleStep({ item }: { item: TimelineItem }) {
  const status = item.event.type === "lifecycle" ? item.event.status : "started";
  const lines = item.body.split("\n").filter(Boolean);

  return (
    <Steps className="rounded-lg border bg-background px-3 py-2" defaultOpen={false}>
      <StepsTrigger className="items-start">
        <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <span className="font-medium text-foreground">Agent run</span>
          <span className={cn("rounded-full px-2 py-0.5 text-xs font-medium", lifecycleStatusClassName(status))}>
            {lifecycleStatusLabel(status)}
          </span>
        </span>
      </StepsTrigger>
      <StepsContent>
        {lines.map((line, index) => {
          const [lineStatus, ...messageParts] = line.split(": ");
          const message = messageParts.join(": ");
          return (
            <StepsItem
              key={`${line}-${index}`}
              className={cn(index === lines.length - 1 && isLifecycleTerminal(lineStatus) && "text-foreground")}
            >
              <span className="font-medium text-foreground">{lifecycleStatusLabel(lineStatus)}</span>
              {message && <span className="ml-2 break-words">{message}</span>}
            </StepsItem>
          );
        })}
      </StepsContent>
    </Steps>
  );
}

function ToolStep({ item }: { item: TimelineItem }) {
  const tool = item.event.type === "tool" ? item.event : null;
  const status = tool?.status || (item.tone === "success" ? "completed" : item.tone === "danger" ? "failed" : "running");
  const locations = tool?.locations ?? [];
  const toolCallId = tool?.toolCallId;

  return (
    <div className="space-y-4">
    <Steps className="" defaultOpen={false}>
      <StepsTrigger>
        <span className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
          <span className="">{item.title || "tool"}</span>
          <ToolStatusIcon status={status} />
        </span>
      </StepsTrigger>
      <StepsContent>
        {locations.map((path) => (
          <StepsItem key={path}>
            <span className="font-medium text-foreground">path</span>{" "}
            <code className="inline-block max-w-full rounded bg-background px-1.5 py-0.5 align-bottom font-mono text-xs">
              <EllipsisPopoverText
                value={path}
                className="font-mono text-xs"
                contentClassName="font-mono text-xs"
              />
            </code>
          </StepsItem>
        ))}
        {!toolCallId && locations.length === 0 && item.body && (
          <StepsItem className="whitespace-pre-wrap break-words font-mono text-xs">{item.body}</StepsItem>
        )}
      </StepsContent>
    </Steps>
    </div>
  );
}

function ToolStatusIcon({ status }: { status: string }) {
  const label = toolStatusLabel(status);
  const className = "size-4 shrink-0";

  if (status === "completed") {
    return (
      <span className="inline-flex size-5 items-center justify-center text-emerald-600 dark:text-emerald-400" role="img" aria-label={label} title={label}>
        <CheckCircleIcon className={className} aria-hidden />
      </span>
    );
  }

  if (status === "failed") {
    return (
      <span className="inline-flex size-5 items-center justify-center text-destructive" role="img" aria-label={label} title={label}>
        <XCircleIcon className={className} aria-hidden />
      </span>
    );
  }

  if (status === "in_progress" || status === "running") {
    return (
      <span className="inline-flex size-5 items-center justify-center text-primary" role="img" aria-label={label} title={label}>
        <Loader2Icon className={cn(className, "animate-spin")} aria-hidden />
      </span>
    );
  }

  if (status === "pending") {
    return (
      <span className="inline-flex size-5 items-center justify-center text-primary" role="img" aria-label={label} title={label}>
        <ClockIcon className={className} aria-hidden />
      </span>
    );
  }

  return (
    <span className="inline-flex size-5 items-center justify-center text-muted-foreground" role="img" aria-label={label} title={label}>
      <SettingsIcon className={className} aria-hidden />
    </span>
  );
}

function lifecycleStatusLabel(status: string) {
  if (status === "started") return "Started";
  if (status === "initialized") return "Initialized";
  if (status === "sessionCreated") return "Session created";
  if (status === "promptSent") return "Prompt sent";
  if (status === "promptCompleted") return "Prompt completed";
  if (status === "completed") return "Completed";
  if (status === "cancelled") return "Cancelled";
  return status || "Lifecycle";
}

function lifecycleStatusClassName(status: string) {
  if (status === "completed" || status === "promptCompleted") {
    return "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300";
  }
  if (status === "cancelled") {
    return "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300";
  }
  return "bg-primary/10 text-primary";
}

function isLifecycleTerminal(status: string) {
  return status === "completed" || status === "promptCompleted" || status === "cancelled";
}

function toolStatusLabel(status: string) {
  if (status === "completed") return "Completed";
  if (status === "failed") return "Failed";
  if (status === "in_progress") return "In progress";
  if (status === "pending") return "In progress";
  if (status === "running") return "Running";
  return status || "Tool";
}

function StreamingMarkdown({ content }: { content: string }) {
  return (
    <div className="min-w-0 break-words text-sm leading-6">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => (
            <h1 className="mb-2 mt-4 text-2xl font-semibold tracking-tight first:mt-0">
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="mb-2 mt-4 text-xl font-semibold tracking-tight first:mt-0">
              {children}
            </h2>
          ),
          h3: ({ children }) => (
            <h3 className="mb-2 mt-3 text-lg font-semibold tracking-tight first:mt-0">
              {children}
            </h3>
          ),
          h4: ({ children }) => (
            <h4 className="mb-2 mt-3 text-base font-semibold tracking-tight first:mt-0">
              {children}
            </h4>
          ),
          p: ({ children }) => <p className="my-2 first:mt-0 last:mb-0">{children}</p>,
          ul: ({ children }) => <ul className="my-2 list-disc pl-5">{children}</ul>,
          ol: ({ children }) => <ol className="my-2 list-decimal pl-5">{children}</ol>,
          li: ({ children }) => <li className="mt-1">{children}</li>,
          blockquote: ({ children }) => (
            <blockquote className="my-3 border-l-4 pl-4 text-muted-foreground">
              {children}
            </blockquote>
          ),
          code: ({ children, className }) => {
            const isBlockCode = className?.includes("language-");
            return (
              <code
                className={cn(
                  "font-mono",
                  isBlockCode
                    ? "text-sm"
                    : "rounded bg-muted px-1.5 py-0.5 text-[0.92em]",
                  className,
                )}
              >
                {children}
              </code>
            );
          },
          pre: ({ children }) => (
            <pre className="my-3 overflow-x-auto rounded-md border bg-muted p-3">
              {children}
            </pre>
          ),
          a: ({ children, href }) => (
            <ExternalLink href={href}>
              {children}
            </ExternalLink>
          ),
          hr: () => <hr className="my-4 border-border" />,
          table: ({ children }) => (
            <div className="my-3 overflow-x-auto">
              <table className="w-full border-collapse text-sm">{children}</table>
            </div>
          ),
          th: ({ children }) => (
            <th className="border bg-muted px-2 py-1 text-left font-semibold align-top">
              {children}
            </th>
          ),
          td: ({ children }) => <td className="border px-2 py-1 align-top">{children}</td>,
          img: ({ alt, src }) => <img className="h-auto max-w-full rounded-md" alt={alt ?? ""} src={src} />,
        }}
      >
        {normalizeStreamingMarkdown(content)}
      </ReactMarkdown>
    </div>
  );
}

function normalizeStreamingMarkdown(content: string) {
  const fenceMatches = content.match(/```/g);
  if (fenceMatches && fenceMatches.length % 2 === 1) {
    const suffix = content.endsWith("\n") ? "```" : "\n```";
    return `${content}${suffix}`;
  }
  return content;
}
