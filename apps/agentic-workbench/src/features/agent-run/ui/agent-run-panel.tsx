import type { KeyboardEvent, ReactNode, RefObject } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { DiffViewer } from "@yoophi/git-ui";
import {
  Group as ResizablePanelGroup,
  Panel as ResizablePanel,
  Separator as ResizableHandle,
} from "react-resizable-panels";
import {
  ArrowDownIcon,
  ArrowUpIcon,
  BotIcon,
  CheckCircleIcon,
  ChevronDownIcon,
  ClockIcon,
  InfoIcon,
  Loader2Icon,
  PencilIcon,
  PlayIcon,
  SettingsIcon,
  SquareIcon,
  XCircleIcon,
  XIcon,
} from "lucide-react";

import {
  cancelAgentRun,
  getAgentRunSettings,
  listenRunEvents,
  listAgents,
  listAgentToolCommandCandidates,
  listProviderSessions,
  respondAgentPermission,
  saveAgentRunSettings,
  sendPromptToRun,
  setRunPermissionMode,
  startAgentRun,
} from "@/entities/agent-run/api/agent-run-repository";
import {
  clearGoal,
  createGoal,
  getGoal,
  recordGoalProgress,
  updateGoal,
} from "@/entities/agent-run/api/goal-repository";
import { agentRunQueryKeys } from "@/entities/agent-run/api/query-keys";
import {
  agentCatalogQueryOptions,
  agentRunSettingsQueryOptions,
  agentToolCommandCandidateQueryOptions,
  goalQueryOptions,
} from "@/entities/agent-run/api/query-options";
import {
  appendOneTimelineItem,
  availableCommandCandidatesFromSessionUpdate,
  clampHighlightedIndex,
  eventGroups,
  filterToolCommandCandidates,
  findPromptAutocompleteTrigger,
  isAvailableCommandsSessionUpdate,
  replacePromptAutocompleteTrigger,
  toTimelineItem,
} from "@/entities/agent-run/model";
import type { TimelineRunEvent } from "@/entities/agent-run/model";
import type {
  ContextSizePreset,
  EventGroup,
  AgentRunSessionMode,
  AgentRunSettings,
  AgentToolCommandCandidate,
  GoalStatus,
  PermissionMode,
  ProviderSession,
  RunEvent,
  ThreadGoal,
  TimelineItem,
  ToolFileChange,
} from "@/entities/agent-run/model";
import {
  buildGoalContinuationPrompt,
  shouldStartGoalContinuation,
} from "@/features/agent-run/model/goal-continuation";
import {
  activateRunStartQueuedPrompt,
  addUserMessage,
  appendQueuedPrompt,
  appendPromptHistory,
  buildSteerPrompt,
  createQueuedPrompt,
  createRunStartQueuedPrompt,
  initialPromptHistoryState,
  insertQueuedPrompt,
  isOverrideCommandFailure,
  isPromptHistoryNavigationBoundary,
  moveQueuedPrompt as reorderQueuedPrompt,
  navigatePromptHistory,
  removeQueuedPrompt,
  removeUserMessage,
  resolveRequestAgentLaunch,
  resolveSelectedProfileId,
  resetPromptHistoryCursor,
  shouldAutoDispatchQueuedPrompt,
  updateQueuedPrompt,
} from "@/features/agent-run/model/run-panel-state";
import type {
  PromptHistoryDirection,
  QueuedPrompt,
  QueuedPromptSource,
  UsageContext,
} from "@/features/agent-run/model/run-panel-state";
import {
  APP_COMMAND_OVERRIDE_SETTINGS_KEY,
  builtInProfileDefaultName,
  effectiveProfiles,
} from "@/features/agent-command-override/model/command-overrides";
import { formatSessionLabel } from "@/features/agent-run/model/session-label";
import { StreamingMarkdown } from "@/features/agent-run/ui/agent-run-markdown";
import { PromptCommandAutocomplete } from "@/features/agent-run/ui/prompt-command-autocomplete";
import { SavedPromptToolbar } from "@/features/saved-prompt/ui/saved-prompt-toolbar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
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
import { CircularLoader } from "@/components/ui/loader";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Message, MessageAvatar, MessageContent } from "@/components/ui/message";
import {
  PromptInput,
  PromptInputAction,
  PromptInputActions,
  PromptInputTextarea,
} from "@/components/ui/prompt-input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Steps, StepsContent, StepsItem, StepsTrigger } from "@/components/ui/steps";
import { SystemMessage } from "@/components/ui/system-message";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type AgentRunPanelProps = {
  workingDirectory: string;
  scrollHeader?: ReactNode;
  onRunSettled?: () => void;
  initialInputMode?: AgentInputMode;
  externalPromptRequest?: AgentPromptRequest | null;
  onOpenSettings?: () => void;
};

type AgentInputMode = "prompt" | "ralphLoop";

export type AgentPromptRequest = {
  id: string;
  text: string;
};

const defaultPrompt = "";
// 백엔드(MAX_RALPH_ITERATIONS)와 맞춘 자동 반복 상한. 입력은 이 값으로 제한된다.
const RALPH_MAX_ITERATIONS = 100;
const RALPH_DEFAULT_PROMPT =
  "이전 결과를 바탕으로 목표를 계속 진행하세요. 목표를 모두 달성했다면 더 진행하지 말고 완료를 알려주세요.";
const GOAL_CONTINUATION_DELAY_MS = 800;
const TIMELINE_ESTIMATED_ITEM_HEIGHT = 96;
const TIMELINE_ITEM_GAP = 12;
const TIMELINE_OVERSCAN = 6;

type TimelineRenderItem =
  | {
      id: string;
      kind: "item";
      item: TimelineItem;
    }
  | {
      id: string;
      kind: "tool-group";
      items: TimelineItem[];
    };

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

type SelectOption<Value extends string = string> = {
  value: Value;
  label: string;
  description: string;
};

const providerDefaultModelOption: SelectOption = {
  value: "providerDefault",
  label: "Provider default",
  description: "Use the selected agent/provider default model.",
};

const defaultContextSizeOption: SelectOption<ContextSizePreset> = {
  value: "default",
  label: "Default context",
  description: "Use the selected agent/provider default context size.",
};

const contextSizeDescriptions: Record<ContextSizePreset, string> = {
  default: defaultContextSizeOption.description,
  medium: "Prefer a balanced context window.",
  large: "Prefer a larger context window.",
  xLarge: "Prefer the largest context window advertised by the selected agent.",
};

const fallbackContextSizeLabels: Record<ContextSizePreset, string> = {
  default: defaultContextSizeOption.label,
  medium: "Medium",
  large: "Large",
  xLarge: "XL",
};

const fallbackModelDescriptions: Record<string, string> = {
  "gpt-5.5": "Use OpenAI's current flagship model for coding and reasoning.",
  "gpt-5.4": "Use OpenAI's more affordable current-generation model.",
  "gpt-5.4-mini": "Use OpenAI's lower-latency mini model.",
  "gpt-5.4-nano": "Use OpenAI's lowest-latency nano model.",
  "gpt-5.3-codex": "Use OpenAI's newer Codex model for coding tasks.",
  "gpt-5.3-codex-spark": "Use OpenAI's faster Codex Spark model when available.",
  "gpt-5.2-codex": "Use GPT-5.2 Codex when the selected provider advertises it.",
  "gpt-5.1-codex": "Use GPT-5.1 Codex when the selected provider advertises it.",
  "gpt-5-codex": "Use GPT-5 Codex when the selected provider advertises it.",
  opus: "Use Claude Code's latest Opus alias.",
  sonnet: "Use Claude Code's latest Sonnet alias.",
  fable: "Use Claude Code's Fable alias.",
  "claude-opus-4-8": "Use Claude's most capable Opus-tier model.",
  "claude-sonnet-4-6": "Use Claude's current Sonnet model.",
  "claude-haiku-4-5": "Use Claude's fast Haiku model.",
};

export function AgentRunPanel({
  workingDirectory,
  scrollHeader,
  onRunSettled,
  initialInputMode = "prompt",
  externalPromptRequest = null,
  onOpenSettings,
}: AgentRunPanelProps) {
  const queryClient = useQueryClient();
  const [selectedAgentId, setSelectedAgentId] = useState<string>("");
  const [sessionMode, setSessionMode] = useState<AgentRunSessionMode>("new");
  const [selectedSessionId, setSelectedSessionId] = useState<string>("");
  const [permissionMode, setPermissionMode] = useState<PermissionMode>("default");
  const [isChangingPermissionMode, setIsChangingPermissionMode] = useState(false);
  const [modelId, setModelId] = useState("providerDefault");
  const [contextSize, setContextSize] = useState<ContextSizePreset>("default");
  const [ralphLoopEnabled, setRalphLoopEnabled] = useState(
    initialInputMode === "ralphLoop",
  );
  const [ralphMaxIterations, setRalphMaxIterations] = useState(5);
  const [ralphDelaySeconds, setRalphDelaySeconds] = useState(0);
  const [ralphStopOnError, setRalphStopOnError] = useState(true);
  const [ralphStopOnPermission, setRalphStopOnPermission] = useState(false);
  const [ralphPromptTemplate, setRalphPromptTemplate] = useState(RALPH_DEFAULT_PROMPT);
  const [prompt, setPrompt] = useState(defaultPrompt);
  const [promptSelection, setPromptSelection] = useState({ start: 0, end: 0 });
  const [autocompleteHighlightedIndex, setAutocompleteHighlightedIndex] = useState(0);
  const [availableCommandCandidates, setAvailableCommandCandidates] = useState<
    AgentToolCommandCandidate[]
  >([]);
  const [autocompleteSuppression, setAutocompleteSuppression] = useState<{
    text: string;
    cursorStart: number;
    cursorEnd: number;
  } | null>(null);
  const [promptHistory, setPromptHistory] = useState(initialPromptHistoryState);
  const [activeRunId, setActiveRunId] = useState<string | null>(null);
  const [isRunning, setIsRunning] = useState(false);
  const [isAwaitingPromptResponse, setIsAwaitingPromptResponse] = useState(false);
  const [directPrompt, setDirectPrompt] = useState<string | null>(null);
  const [queuedPrompts, setQueuedPrompts] = useState<QueuedPrompt[]>([]);
  const [filter, setFilter] = useState<EventGroup | "all">("all");
  const [items, setItems] = useState<TimelineItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isRunSettingsDialogOpen, setIsRunSettingsDialogOpen] = useState(false);
  const [isGoalDialogOpen, setIsGoalDialogOpen] = useState(false);
  const [isRalphSettingsDialogOpen, setIsRalphSettingsDialogOpen] = useState(false);
  const [goalDraft, setGoalDraft] = useState("");
  const [goalTokenBudget, setGoalTokenBudget] = useState("");
  const [editingPrompt, setEditingPrompt] = useState<QueuedPrompt | null>(null);
  const [editingPromptText, setEditingPromptText] = useState("");
  const [usageContext, setUsageContext] = useState<UsageContext | null>(null);
  const [inputMode, setInputMode] = useState<AgentInputMode>(initialInputMode);
  const activeRunIdRef = useRef<string | null>(null);
  const activeGoalRef = useRef<ThreadGoal | null>(null);
  const activePromptSentRef = useRef(false);
  const queuedPromptsRef = useRef<QueuedPrompt[]>([]);
  const runStartedAtRef = useRef<number | null>(null);
  const usageContextRef = useRef<UsageContext | null>(null);
  const goalContinuationPendingRef = useRef(false);
  const settingsHydratedRef = useRef(false);
  const timelineScrollRef = useRef<HTMLDivElement | null>(null);
  const promptTextareaElementRef = useRef<HTMLTextAreaElement | null>(null);
  const handledExternalPromptRequestIdRef = useRef<string | null>(null);

  // 세션 재진입 시 불필요한 refetch를 막는 신선도 정책(specs/007 research R7)은
  // entities/agent-run/api/query-options에서 key 단위로 정의된다.
  const agentsQuery = useQuery({
    queryKey: agentRunQueryKeys.agents,
    queryFn: listAgents,
    ...agentCatalogQueryOptions,
  });
  const agents = agentsQuery.data ?? [];

  const settingsQueryKey = agentRunQueryKeys.settings(workingDirectory);
  const settingsQuery = useQuery({
    queryKey: settingsQueryKey,
    queryFn: () => getAgentRunSettings(workingDirectory),
    ...agentRunSettingsQueryOptions,
  });
  const appCommandSettingsQuery = useQuery({
    queryKey: agentRunQueryKeys.settings(APP_COMMAND_OVERRIDE_SETTINGS_KEY),
    queryFn: () => getAgentRunSettings(APP_COMMAND_OVERRIDE_SETTINGS_KEY),
    ...agentRunSettingsQueryOptions,
  });
  // 세션 시작 선택지 = enabled 프로필(specs/008 FR-011). selectedAgentId에는
  // profile id를 저장하고, provider 흐름(세션 조회 등)에는 agentType을 쓴다.
  const enabledProfiles = useMemo(
    () =>
      effectiveProfiles(appCommandSettingsQuery.data?.commandOverrides).filter(
        (profile) => profile.enabled,
      ),
    [appCommandSettingsQuery.data?.commandOverrides],
  );
  const selectedProfile = enabledProfiles.find((profile) => profile.id === selectedAgentId);
  const providerAgentId = selectedProfile?.agentType ?? selectedAgentId;
  const saveSettingsMutation = useMutation({
    mutationFn: saveAgentRunSettings,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: settingsQueryKey });
    },
  });
  const saveRunSettings = saveSettingsMutation.mutate;

  const goalQueryKey = agentRunQueryKeys.goal(workingDirectory);
  const goalQuery = useQuery({
    queryKey: goalQueryKey,
    queryFn: () => getGoal(workingDirectory),
    ...goalQueryOptions,
  });
  const activeGoal = goalQuery.data ?? null;

  const createGoalMutation = useMutation({
    mutationFn: createGoal,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: goalQueryKey });
    },
  });

  const updateGoalMutation = useMutation({
    mutationFn: (input: Parameters<typeof updateGoal>[1]) =>
      updateGoal(workingDirectory, input),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: goalQueryKey });
    },
  });

  const clearGoalMutation = useMutation({
    mutationFn: () => clearGoal(workingDirectory),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: goalQueryKey });
    },
  });

  const recordRunGoalProgress = useCallback(async () => {
    const goal = activeGoalRef.current;
    if (!goal || !["active", "budgetLimited"].includes(goal.status)) {
      return;
    }

    const elapsedSeconds = runStartedAtRef.current
      ? Math.max(0, Math.round((Date.now() - runStartedAtRef.current) / 1000))
      : 0;
    const tokensUsed = usageContextRef.current?.used ?? goal.tokensUsed;

    try {
      await recordGoalProgress(workingDirectory, {
        tokensUsed,
        timeUsedSeconds: elapsedSeconds,
      });
      await queryClient.invalidateQueries({ queryKey: goalQueryKey });
    } catch (caughtError) {
      setError(String(caughtError));
    } finally {
      runStartedAtRef.current = null;
    }
  }, [goalQueryKey, queryClient, workingDirectory]);

  const sessionsQuery = useQuery({
    queryKey: agentRunQueryKeys.sessions(providerAgentId, workingDirectory),
    queryFn: () => listProviderSessions(providerAgentId, workingDirectory),
    enabled: sessionMode === "reuse" && Boolean(providerAgentId),
  });
  const sessions = sessionsQuery.data ?? [];

  const toolCommandCandidatesQuery = useQuery({
    queryKey: agentRunQueryKeys.toolCommandCandidates(
      activeRunId,
      providerAgentId,
      workingDirectory,
      sessionMode,
    ),
    queryFn: () =>
      listAgentToolCommandCandidates({
        runId: activeRunId,
        agentId: providerAgentId,
        workingDirectory,
        sessionMode,
      }),
    enabled: Boolean(providerAgentId && workingDirectory.trim()),
    ...agentToolCommandCandidateQueryOptions,
  });

  useEffect(() => {
    // 저장값이 없거나(빈 문자열) disabled/삭제된 프로필이면 첫 enabled 프로필로
    // 폴백한다. 설정 로드(hydration) 이후에도 동일 규칙이 적용된다.
    if (enabledProfiles.length === 0) {
      return;
    }
    const resolved = resolveSelectedProfileId(enabledProfiles, selectedAgentId);
    if (resolved !== selectedAgentId) {
      setSelectedAgentId(resolved);
    }
  }, [enabledProfiles, selectedAgentId]);

  // agent를 바꾸면 이전 provider의 세션 선택은 더 이상 유효하지 않다.
  useEffect(() => {
    setSelectedSessionId("");
  }, [selectedAgentId]);

  useEffect(() => {
    if (settingsHydratedRef.current || settingsQuery.isLoading) {
      return;
    }
    if (settingsQuery.isError) {
      setError(String(settingsQuery.error));
      return;
    }

    const savedSettings = settingsQuery.data;
    if (savedSettings) {
      setSelectedAgentId(savedSettings.agentId);
      setPermissionMode(savedSettings.permissionMode);
      setModelId(
        isModelOptionValue(savedSettings.modelId)
          ? savedSettings.modelId
          : "providerDefault",
      );
      setContextSize(savedSettings.contextSize);
      setSessionMode(savedSettings.sessionMode);
      setRalphLoopEnabled(savedSettings.ralphLoop.enabled);
      setInputMode(savedSettings.ralphLoop.enabled ? "ralphLoop" : "prompt");
      setRalphMaxIterations(
        Math.min(
          RALPH_MAX_ITERATIONS,
          Math.max(1, Math.round(savedSettings.ralphLoop.maxIterations)),
        ),
      );
      setRalphDelaySeconds(Math.max(0, savedSettings.ralphLoop.delayMs / 1000));
      setRalphStopOnError(savedSettings.ralphLoop.stopOnError);
      setRalphStopOnPermission(savedSettings.ralphLoop.stopOnPermission);
      setRalphPromptTemplate(
        savedSettings.ralphLoop.promptTemplate.trim() || RALPH_DEFAULT_PROMPT,
      );
    }

    settingsHydratedRef.current = true;
  }, [
    settingsQuery.data,
    settingsQuery.error,
    settingsQuery.isError,
    settingsQuery.isLoading,
  ]);

  function changeInputMode(nextMode: AgentInputMode) {
    if (isRunning) {
      return;
    }
    setInputMode(nextMode);
    setRalphLoopEnabled(nextMode === "ralphLoop");
    if (nextMode === "ralphLoop") {
      setIsRalphSettingsDialogOpen(true);
    } else {
      setIsRalphSettingsDialogOpen(false);
    }
  }

  useEffect(() => {
    if (!settingsHydratedRef.current || !selectedAgentId || !workingDirectory.trim()) {
      return;
    }

    const settings: AgentRunSettings = {
      workingDirectory,
      agentId: selectedAgentId,
      permissionMode,
      modelId,
      contextSize,
      sessionMode,
      ralphLoop: {
        enabled: ralphLoopEnabled,
        maxIterations: ralphMaxIterations,
        delayMs: Math.max(0, Math.round(ralphDelaySeconds * 1000)),
        stopOnError: ralphStopOnError,
        stopOnPermission: ralphStopOnPermission,
        promptTemplate: ralphPromptTemplate,
      },
    };

    const timeoutId = window.setTimeout(() => {
      saveRunSettings(settings, {
        onError: (caughtError) => setError(String(caughtError)),
      });
    }, 400);

    return () => window.clearTimeout(timeoutId);
  }, [
    contextSize,
    modelId,
    permissionMode,
    ralphDelaySeconds,
    ralphLoopEnabled,
    ralphMaxIterations,
    ralphPromptTemplate,
    ralphStopOnPermission,
    ralphStopOnError,
    saveRunSettings,
    selectedAgentId,
    sessionMode,
    workingDirectory,
  ]);

  useEffect(() => {
    activeRunIdRef.current = activeRunId;
    setAvailableCommandCandidates([]);
  }, [activeRunId]);

  useEffect(() => {
    queuedPromptsRef.current = queuedPrompts;
  }, [queuedPrompts]);

  useEffect(() => {
    activeGoalRef.current = activeGoal;
  }, [activeGoal]);

  useEffect(() => {
    usageContextRef.current = usageContext;
  }, [usageContext]);

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
        usageContextRef.current = { used: envelope.event.used, size: envelope.event.size };
        return;
      }

      const timelineEvent: TimelineRunEvent = envelope.event;
      if (timelineEvent.type === "raw") {
        const nextCandidates = availableCommandCandidatesFromSessionUpdate(
          timelineEvent.payload,
          {
            runId: envelope.runId,
            agentId: providerAgentId,
            workingDirectory,
          },
        );
        if (isAvailableCommandsSessionUpdate(timelineEvent.payload)) {
          setAvailableCommandCandidates(nextCandidates);
        }
      }

      setItems((currentItems) =>
        addRunEventItem(currentItems, envelope.runId, timelineEvent),
      );

      if (timelineEvent.type === "error") {
        setIsAwaitingPromptResponse(false);
        setQueuedPrompts([]);
        setDirectPrompt(null);
        setIsRunning(false);
        activePromptSentRef.current = false;
        activeRunIdRef.current = null;
        setActiveRunId(null);
        onRunSettled?.();
        void recordRunGoalProgress();
        return;
      }

      if (timelineEvent.type === "lifecycle") {
        if (timelineEvent.status === "promptSent") {
          activePromptSentRef.current = true;
          const activated = activateRunStartQueuedPrompt({
            queue: queuedPromptsRef.current,
            items,
            runId: envelope.runId,
          });
          if (activated.queuedPrompt) {
            queuedPromptsRef.current = activated.queue;
            setQueuedPrompts(activated.queue);
            setDirectPrompt(activated.queuedPrompt.text);
            setItems((currentItems) =>
              activateRunStartQueuedPrompt({
                queue: [activated.queuedPrompt!],
                items: currentItems,
                runId: envelope.runId,
              }).items,
            );
          }
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
          activePromptSentRef.current = false;
          activeRunIdRef.current = null;
          setActiveRunId(null);
          onRunSettled?.();
          void recordRunGoalProgress();
        }
      }
      if (isIdleThreadStatusEvent(timelineEvent)) {
        setIsAwaitingPromptResponse(false);
      }
    });

    return () => {
      unlisten();
    };
  }, [onRunSettled, providerAgentId, recordRunGoalProgress, workingDirectory]);

  useEffect(() => {
    if (
      !activeRunId ||
      !isRunning ||
      isAwaitingPromptResponse ||
      !shouldAutoDispatchQueuedPrompt(queuedPrompts)
    ) {
      return;
    }

    const nextPrompt = queuedPrompts[0];
    const previousDirectPrompt = directPrompt;
    setIsAwaitingPromptResponse(true);
    setQueuedPrompts((current) => current.slice(1));
    setDirectPrompt(nextPrompt.text);
    setItems((currentItems) =>
      addUserMessage(currentItems, activeRunId, nextPrompt.text),
    );
    void sendPromptToRun(activeRunId, nextPrompt.text)
      .then(() => {
        recordPromptHistory(nextPrompt.text);
      })
      .catch((caughtError) => {
        setQueuedPrompts((current) => [nextPrompt, ...current]);
        setItems((currentItems) =>
          removeUserMessage(currentItems, activeRunId, nextPrompt.text),
        );
        setDirectPrompt(previousDirectPrompt);
        setIsAwaitingPromptResponse(false);
        setError(String(caughtError));
      });
  }, [activeRunId, directPrompt, isAwaitingPromptResponse, isRunning, queuedPrompts]);

  useEffect(() => {
    const sessionReady = sessionMode === "new" || Boolean(selectedSessionId);
    if (
      !shouldStartGoalContinuation({
        goal: activeGoal,
        selectedAgentId,
        isRunning,
        hasQueuedPrompt: queuedPrompts.length > 0,
        promptText: prompt,
        sessionReady,
      })
    ) {
      goalContinuationPendingRef.current = false;
      return;
    }
    if (!activeGoal || goalContinuationPendingRef.current) {
      return;
    }

    goalContinuationPendingRef.current = true;
    const timeoutId = window.setTimeout(() => {
      goalContinuationPendingRef.current = false;
      const goal = activeGoalRef.current;
      if (!goal) {
        return;
      }

      const stillReady = sessionMode === "new" || Boolean(selectedSessionId);
      if (
        !shouldStartGoalContinuation({
          goal,
          selectedAgentId,
          isRunning: activeRunIdRef.current !== null,
          hasQueuedPrompt: queuedPrompts.length > 0,
          promptText: prompt,
          sessionReady: stillReady,
        })
      ) {
        return;
      }

      void startRun(buildGoalContinuationPrompt(goal));
    }, GOAL_CONTINUATION_DELAY_MS);

    return () => {
      window.clearTimeout(timeoutId);
      goalContinuationPendingRef.current = false;
    };
  }, [
    activeGoal,
    isRunning,
    prompt,
    queuedPrompts.length,
    selectedAgentId,
    selectedSessionId,
    sessionMode,
  ]);

  const selectedAgent = agents.find((agent) => agent.id === providerAgentId);
  const modelOptions = useMemo<SelectOption[]>(() => {
    const advertisedModels = selectedAgent?.models ?? [];
    if (advertisedModels.length === 0) {
      return [providerDefaultModelOption];
    }

    return [
      providerDefaultModelOption,
      ...advertisedModels.map((model) => ({
        value: model.id,
        label: model.label,
        description:
          fallbackModelDescriptions[model.id] ??
          `Use ${model.label} with ${selectedAgent?.label ?? "the selected agent"}.`,
      })),
    ];
  }, [selectedAgent]);
  const contextSizeOptions = useMemo<SelectOption<ContextSizePreset>[]>(() => {
    const advertisedContextSizes = selectedAgent?.contextSizes ?? [];
    if (advertisedContextSizes.length === 0) {
      return [defaultContextSizeOption];
    }

    return [
      defaultContextSizeOption,
      ...advertisedContextSizes
        .filter((contextSize): contextSize is { id: ContextSizePreset; label: string } =>
          isContextSizePreset(contextSize.id),
        )
        .map((contextSize) => ({
          value: contextSize.id,
          label: contextSize.label || fallbackContextSizeLabels[contextSize.id],
          description: contextSizeDescriptions[contextSize.id],
        })),
    ];
  }, [selectedAgent]);
  useEffect(() => {
    if (!modelOptions.some((option) => option.value === modelId)) {
      setModelId(providerDefaultModelOption.value);
    }
    if (!contextSizeOptions.some((option) => option.value === contextSize)) {
      setContextSize(defaultContextSizeOption.value);
    }
  }, [contextSize, contextSizeOptions, modelId, modelOptions]);
  const selectedPermissionModeOption = permissionModeOptions.find(
    (option) => option.value === permissionMode,
  );
  const selectedModelOption = modelOptions.find((option) => option.value === modelId);
  const selectedContextSizeOption = contextSizeOptions.find(
    (option) => option.value === contextSize,
  );
  const autocompleteTrigger = useMemo(
    () => {
      const trigger =
        inputMode === "prompt"
          ? findPromptAutocompleteTrigger(prompt, promptSelection.start, promptSelection.end)
          : null;
      if (
        trigger &&
        autocompleteSuppression &&
        autocompleteSuppression.text === prompt &&
        autocompleteSuppression.cursorStart === promptSelection.start &&
        autocompleteSuppression.cursorEnd === promptSelection.end
      ) {
        return null;
      }
      return trigger;
    },
    [
      autocompleteSuppression,
      inputMode,
      prompt,
      promptSelection.end,
      promptSelection.start,
    ],
  );
  const autocompleteSourceCandidates = useMemo(
    () => [
      ...(toolCommandCandidatesQuery.data?.candidates ?? []),
      ...availableCommandCandidates,
    ],
    [availableCommandCandidates, toolCommandCandidatesQuery.data?.candidates],
  );
  const autocompleteCandidates = useMemo(
    () =>
      autocompleteTrigger
        ? filterToolCommandCandidates(
            autocompleteSourceCandidates,
            autocompleteTrigger.query,
          )
        : [],
    [autocompleteSourceCandidates, autocompleteTrigger],
  );
  const autocompleteStatus = useMemo(() => {
    if (!autocompleteTrigger) {
      return "empty" as const;
    }
    if (autocompleteCandidates.length > 0) {
      return "ready" as const;
    }
    if (toolCommandCandidatesQuery.isLoading && autocompleteSourceCandidates.length === 0) {
      return "loading" as const;
    }
    if (toolCommandCandidatesQuery.isError && autocompleteSourceCandidates.length === 0) {
      return "error" as const;
    }
    if (autocompleteSourceCandidates.length === 0) {
      return "empty" as const;
    }
    return "noMatch" as const;
  }, [
    autocompleteCandidates.length,
    autocompleteSourceCandidates.length,
    autocompleteTrigger,
    toolCommandCandidatesQuery.isError,
    toolCommandCandidatesQuery.isLoading,
  ]);
  const isAutocompleteOpen = Boolean(autocompleteTrigger);
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
  const shouldQueueSendPrompt = Boolean(
    activeRunId &&
      isRunning &&
      !isAwaitingPromptResponse &&
      queuedPrompts.length > 0,
  );
  const shouldSendDirectPrompt = Boolean(
    activeRunId &&
      isRunning &&
      !isAwaitingPromptResponse &&
      queuedPrompts.length === 0,
  );
  const canSendPrompt = shouldQueueSendPrompt || shouldSendDirectPrompt
    ? canQueuePrompt
    : canSteerPrompt;
  const canCancel = Boolean(activeRunId && isRunning);

  useEffect(() => {
    setAutocompleteHighlightedIndex((current) =>
      clampHighlightedIndex(current, autocompleteCandidates.length),
    );
  }, [autocompleteCandidates.length]);

  useEffect(() => {
    if (
      !externalPromptRequest ||
      handledExternalPromptRequestIdRef.current === externalPromptRequest.id
    ) {
      return;
    }

    const nextPrompt = externalPromptRequest.text.trim();
    if (!nextPrompt) {
      handledExternalPromptRequestIdRef.current = externalPromptRequest.id;
      return;
    }

    if (activeRunIdRef.current && isRunning) {
      handledExternalPromptRequestIdRef.current = externalPromptRequest.id;
      enqueuePrompt(nextPrompt, "external-request");
      return;
    }

    setInputMode("prompt");
    setRalphLoopEnabled(false);
    setIsRalphSettingsDialogOpen(false);
    if (
      !selectedAgentId ||
      (sessionMode === "reuse" && !selectedSessionId)
    ) {
      setPrompt(nextPrompt);
      return;
    }

    handledExternalPromptRequestIdRef.current = externalPromptRequest.id;
    void startRun(nextPrompt, {
      ralphLoopEnabled: false,
      queuedPromptSource: "external-request",
    }).then((started) => {
      if (!started) {
        setPrompt(nextPrompt);
      }
    });
  }, [externalPromptRequest, isRunning, selectedAgentId, selectedSessionId, sessionMode]);

  async function run() {
    const goal = prompt.trim();
    if (!goal) {
      return;
    }
    setPrompt(defaultPrompt);
    await startRun(goal);
  }

  async function startRun(
    goal: string,
    options: {
      queuedPrompts?: QueuedPrompt[];
      displayPrompt?: string;
      ralphLoopEnabled?: boolean;
      queuedPromptSource?: QueuedPromptSource;
    } = {},
  ) {
    if (!selectedAgentId || !goal) {
      return false;
    }

    const runId = crypto.randomUUID();
    const displayPrompt = options.displayPrompt ?? goal;
    const runStartQueuedPrompt = createRunStartQueuedPrompt({
      id: `${runId}:initial-prompt`,
      text: displayPrompt,
      source: options.queuedPromptSource ?? "first-run",
    });
    const nextQueuedPrompts = runStartQueuedPrompt
      ? [runStartQueuedPrompt, ...(options.queuedPrompts ?? [])]
      : (options.queuedPrompts ?? []);
    setError(null);
    setItems([]);
    queuedPromptsRef.current = nextQueuedPrompts;
    setQueuedPrompts(nextQueuedPrompts);
    setUsageContext(null);
    usageContextRef.current = null;
    runStartedAtRef.current = Date.now();
    setDirectPrompt(null);
    activePromptSentRef.current = false;
    activeRunIdRef.current = runId;
    setActiveRunId(runId);
    setIsRunning(true);
    setIsAwaitingPromptResponse(true);

    const reuseSession = sessionMode === "reuse" && Boolean(selectedSessionId);
    // 선택된 프로필의 command/env를 해석한다(specs/008). agentId는 프로필의
    // agentType(= provider id)으로, 세션 재사용 등 기존 흐름과 호환된다.
    const launch = resolveRequestAgentLaunch({
      profileId: selectedAgentId,
      agents,
      overrides: appCommandSettingsQuery.data?.commandOverrides,
    });

    try {
      await startAgentRun({
        runId,
        goal,
        agentId: launch?.agentId ?? selectedAgentId,
        cwd: workingDirectory,
        ...(launch?.agentCommand ? { agentCommand: launch.agentCommand } : {}),
        ...(launch?.agentEnv ? { agentEnv: launch.agentEnv } : {}),
        stdioBufferLimitMb: 50,
        permissionMode,
        ...(modelId !== "providerDefault" ? { modelId } : {}),
        ...(contextSize !== "default" ? { contextSize } : {}),
        ...(reuseSession
          ? { resumeSessionId: selectedSessionId, resumePolicy: "resumeIfAvailable" }
          : {}),
        ...((options.ralphLoopEnabled ?? ralphLoopEnabled)
          ? {
              ralphLoop: {
                enabled: true,
                maxIterations: ralphMaxIterations,
                promptTemplate: ralphPromptTemplate,
                stopOnError: ralphStopOnError,
                stopOnPermission: ralphStopOnPermission,
                delayMs: Math.max(0, Math.round(ralphDelaySeconds * 1000)),
              },
            }
          : {}),
      });
      recordPromptHistory(displayPrompt);
      return true;
    } catch (caughtError) {
      setError(String(caughtError));
      setPrompt(displayPrompt);
      setItems((currentItems) => removeUserMessage(currentItems, runId, displayPrompt));
      queuedPromptsRef.current = [];
      setQueuedPrompts([]);
      setDirectPrompt(null);
      setIsAwaitingPromptResponse(false);
      setIsRunning(false);
      activePromptSentRef.current = false;
      activeRunIdRef.current = null;
      runStartedAtRef.current = null;
      setActiveRunId(null);
      return false;
    }
  }

  function enqueuePrompt(
    promptText = prompt,
    source: QueuedPromptSource = "manual-queue",
  ) {
    const nextPrompt = promptText.trim();
    if (!nextPrompt) {
      return;
    }

    setQueuedPrompts((current) => {
      const next = appendQueuedPrompt(
        current,
        createQueuedPrompt({ id: crypto.randomUUID(), text: nextPrompt, source }),
      );
      queuedPromptsRef.current = next;
      return next;
    });
    if (promptText === prompt) {
      setPrompt(defaultPrompt);
    }
  }

  async function sendSavedPrompt(savedPrompt: string) {
    const nextPrompt = savedPrompt.trim();
    if (!nextPrompt) {
      return;
    }

    if (isRunning || activeRunIdRef.current) {
      enqueuePrompt(nextPrompt, "saved-prompt");
      return;
    }

    await startRun(nextPrompt, { queuedPromptSource: "saved-prompt" });
  }

  function recordPromptHistory(promptText: string) {
    setPromptHistory((current) => appendPromptHistory(current, promptText));
  }

  function updatePromptDraft(nextPrompt: string) {
    setAutocompleteSuppression(null);
    setPrompt(nextPrompt);
    setPromptHistory((current) => resetPromptHistoryCursor(current));
  }

  function updatePromptSelection(target: HTMLTextAreaElement) {
    promptTextareaElementRef.current = target;
    setPromptSelection({
      start: target.selectionStart,
      end: target.selectionEnd,
    });
  }

  function selectAutocompleteCandidate(candidate: AgentToolCommandCandidate) {
    if (!autocompleteTrigger) {
      return;
    }
    const nextDraft = replacePromptAutocompleteTrigger(
      {
        text: prompt,
        cursorStart: promptSelection.start,
        cursorEnd: promptSelection.end,
      },
      autocompleteTrigger,
      candidate,
    );
    setPrompt(nextDraft.text);
    setPromptSelection({
      start: nextDraft.cursorStart,
      end: nextDraft.cursorEnd,
    });
    setAutocompleteSuppression({
      text: nextDraft.text,
      cursorStart: nextDraft.cursorStart,
      cursorEnd: nextDraft.cursorEnd,
    });
    setPromptHistory((current) => resetPromptHistoryCursor(current));
    window.requestAnimationFrame(() => {
      const textarea = promptTextareaElementRef.current;
      if (!textarea) {
        return;
      }
      textarea.focus();
      textarea.setSelectionRange(nextDraft.cursorStart, nextDraft.cursorEnd);
    });
  }

  function handleAutocompleteKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (!isAutocompleteOpen) {
      return false;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      setAutocompleteSuppression({
        text: prompt,
        cursorStart: promptSelection.start,
        cursorEnd: promptSelection.end,
      });
      return true;
    }
    if (autocompleteCandidates.length === 0) {
      return false;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setAutocompleteHighlightedIndex((current) =>
        clampHighlightedIndex(current + 1, autocompleteCandidates.length),
      );
      return true;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setAutocompleteHighlightedIndex((current) =>
        clampHighlightedIndex(current - 1, autocompleteCandidates.length),
      );
      return true;
    }
    if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      const index = clampHighlightedIndex(
        autocompleteHighlightedIndex,
        autocompleteCandidates.length,
      );
      if (index >= 0) {
        selectAutocompleteCandidate(autocompleteCandidates[index]);
      }
      return true;
    }
    return false;
  }

  function handlePromptHistoryNavigation(
    event: KeyboardEvent<HTMLTextAreaElement>,
    direction: PromptHistoryDirection,
  ) {
    if (inputMode !== "prompt") {
      return false;
    }

    const target = event.currentTarget;
    const result = navigatePromptHistory({
      state: promptHistory,
      direction,
      currentInput: prompt,
      isEditableBoundary: isPromptHistoryNavigationBoundary({
        value: target.value,
        selectionStart: target.selectionStart,
        selectionEnd: target.selectionEnd,
        direction,
      }),
      hasModifierKey:
        event.shiftKey || event.metaKey || event.ctrlKey || event.altKey,
    });

    if (!result.handled) {
      return false;
    }

    event.preventDefault();
    setPrompt(result.nextInput);
    setPromptHistory(result.nextState);
    return true;
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
      await recordRunGoalProgress();
      setQueuedPrompts([]);
      setDirectPrompt(null);
      setIsAwaitingPromptResponse(false);
      setIsRunning(false);
      activeRunIdRef.current = null;
      setActiveRunId(null);
    }
  }

  async function changePermissionMode(nextMode: PermissionMode) {
    if (nextMode === permissionMode) {
      return;
    }

    const previousMode = permissionMode;
    setPermissionMode(nextMode);

    if (!activeRunId || !isRunning) {
      return;
    }

    setError(null);
    setIsChangingPermissionMode(true);
    try {
      await setRunPermissionMode(activeRunId, nextMode);
    } catch (caughtError) {
      setPermissionMode(previousMode);
      setError(String(caughtError));
    } finally {
      setIsChangingPermissionMode(false);
    }
  }

  async function steer() {
    const steerPrompt = prompt.trim();
    const originalPrompt = directPrompt?.trim();
    const runIdToCancel = activeRunId;
    const wasAwaitingPromptResponse = isAwaitingPromptResponse;

    if (!runIdToCancel || !originalPrompt || !steerPrompt) {
      return;
    }

    const nextGoal = buildSteerPrompt(originalPrompt, steerPrompt);
    const queuedPromptsToKeep = queuedPrompts;
    setError(null);
    setPrompt(defaultPrompt);
    setIsAwaitingPromptResponse(true);

    try {
      await cancelAgentRun(runIdToCancel);
      const started = await startRun(nextGoal, {
        queuedPrompts: queuedPromptsToKeep,
        displayPrompt: steerPrompt,
      });
      if (!started) {
        setPrompt(steerPrompt);
        setQueuedPrompts(queuedPromptsToKeep);
      }
    } catch (caughtError) {
      setError(String(caughtError));
      setPrompt(steerPrompt);
      setQueuedPrompts(queuedPromptsToKeep);
      setIsAwaitingPromptResponse(wasAwaitingPromptResponse);
    }
  }

  function sendPrompt() {
    if (activeRunIdRef.current && !activePromptSentRef.current) {
      enqueuePrompt();
      return;
    }
    if (shouldQueueSendPrompt) {
      enqueuePrompt();
      return;
    }
    if (shouldSendDirectPrompt) {
      void sendDirectPrompt();
      return;
    }

    void steer();
  }

  async function sendDirectPrompt() {
    const nextPrompt = prompt.trim();
    const runId = activeRunId;
    const previousDirectPrompt = directPrompt;
    if (!runId || !nextPrompt) {
      return;
    }

    setError(null);
    setPrompt(defaultPrompt);
    setIsAwaitingPromptResponse(true);
    setDirectPrompt(nextPrompt);
    setItems((currentItems) => addUserMessage(currentItems, runId, nextPrompt));

    try {
      await sendPromptToRun(runId, nextPrompt);
      recordPromptHistory(nextPrompt);
    } catch (caughtError) {
      setPrompt(nextPrompt);
      setItems((currentItems) => removeUserMessage(currentItems, runId, nextPrompt));
      setDirectPrompt(previousDirectPrompt);
      setIsAwaitingPromptResponse(false);
      setError(String(caughtError));
    }
  }

  async function steerQueuedPrompt(queuedPrompt: QueuedPrompt) {
    const originalPrompt = directPrompt?.trim();
    const runIdToCancel = activeRunId;
    const wasAwaitingPromptResponse = isAwaitingPromptResponse;

    if (!runIdToCancel || !originalPrompt) {
      return;
    }

    const result = removeQueuedPrompt(queuedPrompts, queuedPrompt.id);
    if (!result.queuedPrompt) {
      setError("전송하려던 prompt가 이미 queue에서 제거되었습니다.");
      return;
    }

    const nextGoal = buildSteerPrompt(originalPrompt, result.queuedPrompt.text);
    setError(null);
    setQueuedPrompts(result.queue);
    setIsAwaitingPromptResponse(true);
    if (editingPrompt?.id === result.queuedPrompt.id) {
      closeQueuedPromptEditor();
    }

    const restoreQueue = () => {
      setQueuedPrompts((current) =>
        current.some((item) => item.id === result.queuedPrompt!.id)
          ? current
          : insertQueuedPrompt(current, result.queuedPrompt!, result.index),
      );
    };

    try {
      await cancelAgentRun(runIdToCancel);
      const started = await startRun(nextGoal, {
        queuedPrompts: result.queue,
        displayPrompt: result.queuedPrompt.text,
      });
      if (!started) {
        restoreQueue();
        setPrompt(defaultPrompt);
      }
    } catch (caughtError) {
      restoreQueue();
      setError(String(caughtError));
      setPrompt(defaultPrompt);
      setIsAwaitingPromptResponse(wasAwaitingPromptResponse);
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

  function openGoalDialog(goal: ThreadGoal | null) {
    setGoalDraft(goal?.objective ?? "");
    setGoalTokenBudget(goal?.tokenBudget ? String(goal.tokenBudget) : "");
    setIsGoalDialogOpen(true);
  }

  async function saveGoal() {
    const objective = goalDraft.trim();
    if (!objective) {
      return;
    }

    const tokenBudget = parseOptionalPositiveInteger(goalTokenBudget);

    try {
      if (activeGoal) {
        await updateGoalMutation.mutateAsync({
          objective,
          tokenBudget,
          ...(activeGoal.status === "complete" ? { status: "active" as GoalStatus } : {}),
        });
      } else {
        await createGoalMutation.mutateAsync({
          workingDirectory,
          objective,
          tokenBudget,
        });
      }
      setIsGoalDialogOpen(false);
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  async function setGoalStatus(status: GoalStatus) {
    try {
      await updateGoalMutation.mutateAsync({ status });
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  async function clearCurrentGoal() {
    try {
      await clearGoalMutation.mutateAsync();
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  return (
    <div className="h-full min-h-0">
      <ResizablePanelGroup
        orientation="vertical"
        className="flex h-full min-h-0 w-full flex-col"
      >
        <ResizablePanel id="agent-run-timeline" minSize="220px">
          <div ref={timelineScrollRef} className="h-full min-h-0 overflow-auto">
            <div className="flex flex-col">
          {scrollHeader}

          <div className="m-4 flex flex-col gap-4">
            {!isRunning && (
              <div>
                <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                  <div className="flex flex-col gap-1.5">
                    <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
                      <BotIcon />
                      Agentic coding
                    </h2>
                  </div>
                  <div className="flex flex-wrap items-center gap-3">
                    <div className="flex flex-col gap-1.5">
                      <Select
                        value={selectedAgentId}
                        onValueChange={setSelectedAgentId}
                        disabled={agentsQuery.isLoading || appCommandSettingsQuery.isLoading}
                      >
                        <SelectTrigger className="w-full sm:w-56">
                          <SelectValue placeholder="Agent 프로필 선택" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectGroup>
                            {enabledProfiles.map((profile) => (
                              <SelectItem key={profile.id} value={profile.id}>
                                {profile.name}
                                {profile.name !== builtInProfileDefaultName(profile.agentType)
                                  ? ` · ${builtInProfileDefaultName(profile.agentType)}`
                                  : ""}
                              </SelectItem>
                            ))}
                          </SelectGroup>
                        </SelectContent>
                      </Select>
                    </div>
                    <div className="flex flex-col gap-1.5">
                      <div className="flex flex-wrap items-center gap-1.5">
                        <Button
                          type="button"
                          size="sm"
                          variant={sessionMode === "new" ? "default" : "outline"}
                          onClick={() => setSessionMode("new")}
                        >
                          새 세션
                        </Button>
                        <Button
                          type="button"
                          size="sm"
                          variant={sessionMode === "reuse" ? "default" : "outline"}
                          onClick={() => setSessionMode("reuse")}
                        >
                          기존 세션 재사용
                        </Button>
                        {sessionMode === "reuse" && (
                          <Select
                            value={selectedSessionId}
                            onValueChange={setSelectedSessionId}
                            disabled={
                              sessionsQuery.isLoading ||
                              sessionsQuery.isError ||
                              sessions.length === 0
                            }
                          >
                            <SelectTrigger className="w-56">
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
                      </div>
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
              </div>
            )}
            <div className="flex flex-col gap-4">
              {error && (
                <SystemMessage variant="error" fill>
                  <span className="flex flex-wrap items-center gap-2">
                    <span>{error}</span>
                    {onOpenSettings && isOverrideCommandFailure(error) && (
                      <Button type="button" size="sm" variant="outline" onClick={onOpenSettings}>
                        <SettingsIcon data-icon="inline-start" />
                        설정 수정
                      </Button>
                    )}
                  </span>
                </SystemMessage>
              )}

              <GoalStatusPanel
                goal={activeGoal}
                isLoading={goalQuery.isLoading}
                isMutating={
                  createGoalMutation.isPending ||
                  updateGoalMutation.isPending ||
                  clearGoalMutation.isPending
                }
                onCreate={() => openGoalDialog(null)}
                onEdit={() => openGoalDialog(activeGoal)}
                onPause={() => void setGoalStatus("paused")}
                onResume={() => void setGoalStatus("active")}
                onComplete={() => void setGoalStatus("complete")}
                onClear={() => void clearCurrentGoal()}
              />

              <div className="flex flex-col">
                <div className="flex flex-wrap gap-1.5 border-b pb-3" role="tablist" aria-label="ACP event filter">
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
                <VirtualizedRunTimeline
                  items={visibleItems}
                  scrollParentRef={timelineScrollRef}
                />
                {inputMode === "prompt" && queuedPrompts.length > 0 && (
                  <QueuedPromptTimeline
                    queuedPrompts={queuedPrompts}
                    activeRunId={activeRunId}
                    directPrompt={directPrompt}
                    onSteerPrompt={(queuedPrompt) => void steerQueuedPrompt(queuedPrompt)}
                    onEditPrompt={openQueuedPromptEditor}
                    onMovePrompt={moveQueuedPrompt}
                    onRemovePrompt={(queuedPromptId) => {
                      setQueuedPrompts((current) =>
                        current.filter((item) => item.id !== queuedPromptId),
                      );
                    }}
                  />
                )}
              </div>

          </div>
            </div>
          </div>
          </div>
        </ResizablePanel>

        <ResizableHandle
          aria-label="프롬프트 영역 크기 조정"
          className="relative flex h-2 shrink-0 cursor-ns-resize items-center justify-center bg-transparent transition-colors after:absolute after:left-0 after:right-0 after:h-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <div className="relative z-10 h-1 w-12 rounded-full bg-border transition-colors" />
        </ResizableHandle>

        <ResizablePanel
          id="agent-run-prompt"
          defaultSize="300px"
          minSize="180px"
          maxSize="560px"
        >
        <PromptInput
          value={prompt}
          onValueChange={updatePromptDraft}
          onSubmit={() => {
            if (inputMode === "prompt" && (isRunning || activeRunIdRef.current)) {
              sendPrompt();
              return;
            }
            if (!isRunning && canStartRun) {
              void run();
            }
          }}
          isLoading={isRunning}
          className="flex h-full min-h-0 flex-col overflow-hidden rounded-none border-0 bg-transparent p-0 shadow-none"
        >
          <div className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b px-4 py-2">
            <div className="flex gap-1.5" role="tablist" aria-label="Agent input mode">
              <Button
                type="button"
                size="sm"
                variant={inputMode === "prompt" ? "default" : "outline"}
                role="tab"
                aria-selected={inputMode === "prompt"}
                disabled={isRunning}
                onClick={() => changeInputMode("prompt")}
              >
                Prompt
              </Button>
              <Button
                type="button"
                size="sm"
                variant={inputMode === "ralphLoop" ? "default" : "outline"}
                role="tab"
                aria-selected={inputMode === "ralphLoop"}
                disabled={isRunning}
                onClick={() => changeInputMode("ralphLoop")}
              >
                Ralph loop
              </Button>
            </div>
            {usageContext && (
              <div className="flex min-w-0 items-center gap-2 text-xs text-muted-foreground">
                <span className="font-medium">Context</span>
                <div className="h-1.5 w-20 overflow-hidden rounded-full bg-muted sm:w-28">
                  <div
                    className="h-full rounded-full bg-primary transition-[width]"
                    style={{ width: `${usagePercent ?? 0}%` }}
                  />
                </div>
                <span className="shrink-0 font-mono">
                  {usageContext.used}/{usageContext.size}
                  {usagePercent !== null ? ` (${usagePercent}%)` : ""}
                </span>
              </div>
            )}
          </div>
          {inputMode === "ralphLoop" && (
            <div
              className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b px-4 py-2"
              role="tabpanel"
              onClick={(event) => event.stopPropagation()}
            >
              <span className="min-w-0 text-xs text-muted-foreground">
                최대 {ralphMaxIterations}회 반복, {ralphDelaySeconds}초 지연, 오류 시{" "}
                {ralphStopOnError ? "중단" : "계속"}, 권한 요청 시{" "}
                {ralphStopOnPermission ? "중단" : "계속"}
              </span>
              <Button
                type="button"
                size="sm"
                variant="outline"
                disabled={isRunning}
                onClick={() => setIsRalphSettingsDialogOpen(true)}
              >
                <SettingsIcon data-icon="inline-start" />
                Settings
              </Button>
            </div>
          )}
          {inputMode === "prompt" && (
            <SavedPromptToolbar
              disabled={!selectedAgentId}
              onSendPrompt={(savedPrompt) => void sendSavedPrompt(savedPrompt)}
            />
          )}
          <div className="relative min-h-0 flex-1">
            <PromptCommandAutocomplete
              open={isAutocompleteOpen}
              status={autocompleteStatus}
              candidates={autocompleteCandidates}
              highlightedIndex={autocompleteHighlightedIndex}
              onHighlight={setAutocompleteHighlightedIndex}
              onSelect={selectAutocompleteCandidate}
            />
            <PromptInputTextarea
              disableAutosize
              placeholder={
                inputMode === "ralphLoop"
                  ? "Ralph loop로 반복 실행할 초기 작업을 입력하세요."
                  : "선택한 worktree에서 실행할 작업을 입력하세요."
              }
              className="h-full min-h-0 resize-none overflow-auto px-4"
              onFocus={(event) => updatePromptSelection(event.currentTarget)}
              onSelect={(event) => updatePromptSelection(event.currentTarget)}
              onKeyUp={(event) => updatePromptSelection(event.currentTarget)}
              onKeyDown={(event) => {
                promptTextareaElementRef.current = event.currentTarget;
                if (handleAutocompleteKeyDown(event)) {
                  return;
                }
                if (event.key === "ArrowUp") {
                  if (handlePromptHistoryNavigation(event, "previous")) {
                    return;
                  }
                }
                if (event.key === "ArrowDown") {
                  if (handlePromptHistoryNavigation(event, "next")) {
                    return;
                  }
                }
                if (
                  event.key === "Tab" &&
                  !event.shiftKey &&
                  !event.metaKey &&
                  !event.ctrlKey &&
                  !event.altKey &&
                  inputMode === "prompt" &&
                  (isRunning || activeRunIdRef.current)
                ) {
                  event.preventDefault();
                  enqueuePrompt();
                }
              }}
            />
          </div>
          <div className="flex shrink-0 flex-col gap-3 px-4 pb-1 sm:flex-row sm:items-center sm:justify-between">
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() => setIsRunSettingsDialogOpen(true)}
            >
              <SettingsIcon data-icon="inline-start" />
              Settings
            </Button>
            <PromptInputActions className="justify-end">
              <Popover>
                <PopoverTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="size-8"
                    aria-label="Run 설정 정보"
                  >
                    <InfoIcon className="size-4" />
                  </Button>
                </PopoverTrigger>
                <PopoverContent
                  align="end"
                  className="w-72"
                >
                  <div className="flex flex-col gap-3 text-sm">
                    {isChangingPermissionMode ? (
                      <p className="text-xs text-muted-foreground">
                        permission mode를 실행 중인 agent에 적용하는 중입니다...
                      </p>
                    ) : (
                      <>
                        <div className="grid gap-1">
                          <span className="text-xs font-medium text-muted-foreground">
                            Permission mode
                          </span>
                          <span>{selectedPermissionModeOption?.label ?? "Default"}</span>
                          <span className="text-xs text-muted-foreground">
                            {isRunning
                              ? "실행 중에 변경하면 이후 승인 요청부터 즉시 적용됩니다."
                              : selectedPermissionModeOption?.description}
                          </span>
                        </div>
                        <div className="grid gap-1">
                          <span className="text-xs font-medium text-muted-foreground">Model</span>
                          <span>{selectedModelOption?.label ?? providerDefaultModelOption.label}</span>
                          <span className="text-xs text-muted-foreground">
                            {selectedModelOption?.description}
                          </span>
                        </div>
                        <div className="grid gap-1">
                          <span className="text-xs font-medium text-muted-foreground">Context</span>
                          <span>
                            {selectedContextSizeOption?.label ?? defaultContextSizeOption.label}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {selectedContextSizeOption?.description}
                          </span>
                        </div>
                        <div className="grid gap-1">
                          <span className="text-xs font-medium text-muted-foreground">Status</span>
                          <span>{isRunning ? "Running" : "Idle"}</span>
                        </div>
                        <div className="grid gap-1">
                          <span className="text-xs font-medium text-muted-foreground">
                            Working directory
                          </span>
                          <span className="break-all font-mono text-xs text-muted-foreground">
                            {workingDirectory}
                          </span>
                        </div>
                        {selectedAgent && (
                          <div className="grid gap-1">
                            <span className="text-xs font-medium text-muted-foreground">
                              Agent command
                            </span>
                            <span className="break-all font-mono text-xs text-muted-foreground">
                              {selectedAgent.command}
                            </span>
                          </div>
                        )}
                      </>
                    )}
                  </div>
                </PopoverContent>
              </Popover>
              {inputMode === "ralphLoop" ? (
                isRunning ? (
                  <PromptInputAction tooltip="Cancel loop">
                    <Button
                      type="button"
                      variant="destructive"
                      size="sm"
                      disabled={!canCancel}
                      onClick={() => void cancel()}
                    >
                      <SquareIcon data-icon="inline-start" />
                      Cancel loop
                    </Button>
                  </PromptInputAction>
                ) : (
                  <PromptInputAction tooltip="Start Ralph loop">
                    <Button type="button" size="sm" disabled={!canStartRun} onClick={() => void run()}>
                      <PlayIcon data-icon="inline-start" />
                      Run loop
                    </Button>
                  </PromptInputAction>
                )
              ) : isRunning ? (
                <>
                  <div className="inline-flex shrink-0 items-center">
                    <PromptInputAction tooltip="Send">
                      <Button
                        type="button"
                        size="sm"
                        disabled={!canSendPrompt}
                        className="rounded-r-none border-r border-primary-foreground/20"
                        onClick={sendPrompt}
                      >
                        Send
                      </Button>
                    </PromptInputAction>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          type="button"
                          size="icon-sm"
                          disabled={!canSteerPrompt && !canQueuePrompt}
                          className="rounded-l-none"
                          aria-label="Send 옵션"
                          onClick={(event) => event.stopPropagation()}
                        >
                          <ChevronDownIcon className="size-3" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end" className="min-w-32">
                        <DropdownMenuItem
                          disabled={!canSendPrompt}
                          onSelect={sendPrompt}
                        >
                          Send
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          disabled={!canQueuePrompt}
                          onSelect={() => enqueuePrompt()}
                        >
                          Queue
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
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
        </ResizablePanel>
      </ResizablePanelGroup>

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

      <Dialog open={isRunSettingsDialogOpen} onOpenChange={setIsRunSettingsDialogOpen}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>Run 설정</DialogTitle>
            <DialogDescription>
              권한 모드, 모델, 컨텍스트 크기를 설정합니다.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-4">
            <label className="flex flex-col gap-2 text-sm font-medium">
              Permission mode
              <Select
                value={permissionMode}
                onValueChange={(value) => void changePermissionMode(value as PermissionMode)}
                disabled={isChangingPermissionMode}
              >
                <SelectTrigger>
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
              <span className="text-xs font-normal text-muted-foreground">
                {isChangingPermissionMode
                  ? "permission mode를 실행 중인 agent에 적용하는 중입니다..."
                  : isRunning
                    ? "실행 중에 변경하면 이후 승인 요청부터 즉시 적용됩니다."
                    : selectedPermissionModeOption?.description}
              </span>
            </label>
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="flex flex-col gap-2 text-sm font-medium">
                Model
                <Select
                  value={modelId}
                  onValueChange={setModelId}
                  disabled={isRunning}
                >
                  <SelectTrigger>
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
                <span className="text-xs font-normal text-muted-foreground">
                  {isRunning
                    ? "실행 중에는 모델을 변경할 수 없습니다."
                    : selectedModelOption?.description}
                </span>
              </label>
              <label className="flex flex-col gap-2 text-sm font-medium">
                Context
                <Select
                  value={contextSize}
                  onValueChange={(value) => setContextSize(value as ContextSizePreset)}
                  disabled={isRunning}
                >
                  <SelectTrigger>
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
                <span className="text-xs font-normal text-muted-foreground">
                  {isRunning
                    ? "실행 중에는 컨텍스트 크기를 변경할 수 없습니다."
                    : selectedContextSizeOption?.description}
                </span>
              </label>
            </div>
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button">완료</Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={isRalphSettingsDialogOpen}
        onOpenChange={(open) => {
          if (!isRunning) {
            setIsRalphSettingsDialogOpen(open);
          }
        }}
      >
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>Ralph loop 설정</DialogTitle>
            <DialogDescription>
              반복 실행 횟수, 지연, 중단 조건과 반복 prompt를 설정합니다.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-4">
            <div className="grid gap-3 sm:grid-cols-2">
              <label className="flex flex-col gap-2 text-sm font-medium">
                반복 횟수
                <Input
                  type="number"
                  min={1}
                  max={RALPH_MAX_ITERATIONS}
                  value={ralphMaxIterations}
                  disabled={isRunning}
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
              <label className="flex flex-col gap-2 text-sm font-medium">
                반복 간 지연(초)
                <Input
                  type="number"
                  min={0}
                  value={ralphDelaySeconds}
                  disabled={isRunning}
                  onChange={(event) => {
                    const next = Number(event.target.value);
                    if (Number.isFinite(next)) {
                      setRalphDelaySeconds(Math.max(0, next));
                    }
                  }}
                />
              </label>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                size="sm"
                variant={ralphStopOnError ? "default" : "outline"}
                disabled={isRunning}
                onClick={() => setRalphStopOnError((value) => !value)}
              >
                오류 시 중단: {ralphStopOnError ? "On" : "Off"}
              </Button>
              <Button
                type="button"
                size="sm"
                variant={ralphStopOnPermission ? "default" : "outline"}
                disabled={isRunning}
                onClick={() => setRalphStopOnPermission((value) => !value)}
              >
                권한 요청 시 중단: {ralphStopOnPermission ? "On" : "Off"}
              </Button>
            </div>
            <label className="flex flex-col gap-2 text-sm font-medium">
              Loop prompt
              <Textarea
                value={ralphPromptTemplate}
                disabled={isRunning}
                placeholder="반복마다 agent에게 보낼 loop prompt"
                className="min-h-32 text-sm"
                onChange={(event) => setRalphPromptTemplate(event.target.value)}
              />
            </label>
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button">완료</Button>
            </DialogClose>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={isGoalDialogOpen} onOpenChange={setIsGoalDialogOpen}>
        <DialogContent className="sm:max-w-xl">
          <DialogHeader>
            <DialogTitle>{activeGoal ? "Goal 편집" : "Goal 생성"}</DialogTitle>
            <DialogDescription>
              현재 worktree에 저장할 장기 목표를 설정합니다.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-2">
              <span className="text-sm font-medium">Objective</span>
              <Textarea
                value={goalDraft}
                onChange={(event) => setGoalDraft(event.target.value)}
                className="min-h-36 resize-y"
                placeholder="완료 또는 차단될 때까지 이어갈 목표를 입력하세요."
                autoFocus
              />
            </div>
            <div className="flex flex-col gap-2">
              <span className="text-sm font-medium">Token budget</span>
              <Input
                value={goalTokenBudget}
                onChange={(event) => setGoalTokenBudget(event.target.value)}
                inputMode="numeric"
                placeholder="Optional"
              />
            </div>
          </div>
          <DialogFooter>
            <DialogClose asChild>
              <Button type="button" variant="outline">
                취소
              </Button>
            </DialogClose>
            <Button
              type="button"
              disabled={
                !goalDraft.trim() ||
                createGoalMutation.isPending ||
                updateGoalMutation.isPending
              }
              onClick={() => void saveGoal()}
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

type GoalStatusPanelProps = {
  goal: ThreadGoal | null;
  isLoading: boolean;
  isMutating: boolean;
  onCreate: () => void;
  onEdit: () => void;
  onPause: () => void;
  onResume: () => void;
  onComplete: () => void;
  onClear: () => void;
};

function GoalStatusPanel({
  goal,
  isLoading,
  isMutating,
  onCreate,
  onEdit,
  onPause,
  onResume,
  onComplete,
  onClear,
}: GoalStatusPanelProps) {
  if (isLoading) {
    return (
      <div className="flex items-center gap-2 rounded-md border bg-muted/30 px-3 py-2 text-sm text-muted-foreground">
        <Loader2Icon className="size-4 animate-spin" />
        Goal 불러오는 중
      </div>
    );
  }

  if (!goal) {
    return (
      <div className="flex flex-col gap-2 rounded-md border bg-muted/30 px-3 py-2 sm:flex-row sm:items-center sm:justify-between">
        <span className="text-sm text-muted-foreground">
          이 worktree에 설정된 goal이 없습니다.
        </span>
        <Button type="button" size="sm" onClick={onCreate} disabled={isMutating}>
          <PlayIcon data-icon="inline-start" />
          Goal 생성
        </Button>
      </div>
    );
  }

  const canPause = goal.status === "active";
  const canResume = goal.status === "paused" || goal.status === "blocked";
  const canComplete = goal.status !== "complete";

  return (
    <div className="flex flex-col gap-3 rounded-md border bg-muted/30 px-3 py-2">
      <div className="flex flex-col gap-2 lg:flex-row lg:items-start lg:justify-between">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-sm font-medium">Goal</span>
            <Badge variant={goal.status === "active" ? "default" : "secondary"}>
              {goalStatusLabel(goal.status)}
            </Badge>
            {goal.tokenBudget ? (
              <span className="font-mono text-xs text-muted-foreground">
                {goal.tokensUsed}/{goal.tokenBudget} tokens
              </span>
            ) : null}
          </div>
          <p className="mt-1 whitespace-pre-wrap break-words text-sm text-muted-foreground">
            {goal.objective}
          </p>
        </div>
        <div className="flex shrink-0 flex-wrap gap-1.5">
          <Button type="button" size="sm" variant="outline" onClick={onEdit} disabled={isMutating}>
            <PencilIcon data-icon="inline-start" />
            Edit
          </Button>
          {canPause && (
            <Button type="button" size="sm" variant="outline" onClick={onPause} disabled={isMutating}>
              <SquareIcon data-icon="inline-start" />
              Pause
            </Button>
          )}
          {canResume && (
            <Button type="button" size="sm" variant="outline" onClick={onResume} disabled={isMutating}>
              <PlayIcon data-icon="inline-start" />
              Resume
            </Button>
          )}
          {canComplete && (
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={onComplete}
              disabled={isMutating}
            >
              <CheckCircleIcon data-icon="inline-start" />
              Complete
            </Button>
          )}
          <Button type="button" size="sm" variant="destructive" onClick={onClear} disabled={isMutating}>
            <XIcon data-icon="inline-start" />
            Clear
          </Button>
        </div>
      </div>
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

function isModelOptionValue(value: string) {
  return value.trim().length > 0;
}

function isContextSizePreset(value: string): value is ContextSizePreset {
  return value === "default" || value === "medium" || value === "large" || value === "xLarge";
}

function parseOptionalPositiveInteger(value: string) {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const parsed = Number.parseInt(trimmed, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return null;
  }

  return parsed;
}

function goalStatusLabel(status: GoalStatus) {
  const labels: Record<GoalStatus, string> = {
    active: "Active",
    paused: "Paused",
    blocked: "Blocked",
    usageLimited: "Usage limited",
    budgetLimited: "Budget limited",
    complete: "Complete",
  };

  return labels[status];
}

function VirtualizedRunTimeline({
  items,
  scrollParentRef,
}: {
  items: TimelineItem[];
  scrollParentRef: RefObject<HTMLDivElement | null>;
}) {
  const timelineRef = useRef<HTMLDivElement | null>(null);
  const stickToBottomRef = useRef(true);
  const [viewportHeight, setViewportHeight] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [measuredHeights, setMeasuredHeights] = useState<Record<string, number>>({});
  const renderItems = useMemo(() => groupTimelineRenderItems(items), [items]);

  useEffect(() => {
    const scrollElement = scrollParentRef.current;
    const timelineElement = timelineRef.current;
    if (!scrollElement || !timelineElement) {
      return;
    }

    const updateViewport = () => {
      const scrollRect = scrollElement.getBoundingClientRect();
      const timelineRect = timelineElement.getBoundingClientRect();
      const visibleTop = Math.max(0, scrollRect.top - timelineRect.top);
      const visibleBottom = Math.min(scrollRect.bottom, timelineRect.bottom);
      const visibleHeight = Math.max(
        0,
        visibleBottom - Math.max(scrollRect.top, timelineRect.top),
      );
      const distanceFromBottom =
        scrollElement.scrollHeight - scrollElement.scrollTop - scrollElement.clientHeight;

      stickToBottomRef.current = distanceFromBottom < 48;
      setScrollTop(visibleTop);
      setViewportHeight(visibleHeight);
    };

    const resizeObserver = new ResizeObserver(updateViewport);
    resizeObserver.observe(scrollElement);
    resizeObserver.observe(timelineElement);
    scrollElement.addEventListener("scroll", updateViewport, { passive: true });
    updateViewport();

    return () => {
      resizeObserver.disconnect();
      scrollElement.removeEventListener("scroll", updateViewport);
    };
  }, [scrollParentRef]);

  const itemLayouts = useMemo(() => {
    let offset = 0;
    return renderItems.map((item) => {
      const height = measuredHeights[item.id] ?? TIMELINE_ESTIMATED_ITEM_HEIGHT;
      const layout = { item, height, start: offset, end: offset + height };
      offset += height + TIMELINE_ITEM_GAP;
      return layout;
    });
  }, [renderItems, measuredHeights]);

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
    const scrollElement = scrollParentRef.current;
    if (!scrollElement || !stickToBottomRef.current) {
      return;
    }

    scrollElement.scrollTop = scrollElement.scrollHeight;
  }, [items.length, scrollParentRef, totalHeight]);

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
      ref={timelineRef}
      className="min-h-[320px] py-4"
      role="log"
      aria-live="polite"
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
  item: TimelineRenderItem;
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
      <RunEventRenderItem item={item} />
    </div>
  );
}

function groupTimelineRenderItems(items: TimelineItem[]): TimelineRenderItem[] {
  const result: TimelineRenderItem[] = [];
  let toolGroup: TimelineItem[] = [];

  const flushToolGroup = () => {
    if (!toolGroup.length) {
      return;
    }

    result.push({
      id: `tool-group:${toolGroup.map((item) => item.id).join(":")}`,
      kind: "tool-group",
      items: toolGroup,
    });
    toolGroup = [];
  };

  for (const item of items) {
    if (item.group === "tool_call/tool_result") {
      toolGroup.push(item);
      continue;
    }

    flushToolGroup();
    result.push({ id: item.id, kind: "item", item });
  }

  flushToolGroup();
  return result;
}

function RunEventRenderItem({ item }: { item: TimelineRenderItem }) {
  if (item.kind === "tool-group") {
    return <ToolStepGroup items={item.items} />;
  }

  return <RunEventItem item={item.item} />;
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

function QueuedPromptTimeline({
  queuedPrompts,
  activeRunId,
  directPrompt,
  onSteerPrompt,
  onEditPrompt,
  onMovePrompt,
  onRemovePrompt,
}: {
  queuedPrompts: QueuedPrompt[];
  activeRunId: string | null;
  directPrompt: string | null;
  onSteerPrompt: (queuedPrompt: QueuedPrompt) => void;
  onEditPrompt: (queuedPrompt: QueuedPrompt) => void;
  onMovePrompt: (fromIndex: number, toIndex: number) => void;
  onRemovePrompt: (queuedPromptId: string) => void;
}) {
  return (
    <div className="mt-3 flex flex-col gap-2" aria-label="Queued prompts">
      {queuedPrompts.map((queuedPrompt, index) => (
        <Message key={queuedPrompt.id} className="justify-end">
          <div className="min-w-0 max-w-[80%] rounded-lg border border-border bg-muted-foreground p-2 text-background">
            <div className="mb-1.5 flex flex-wrap items-center justify-between gap-2">
              <div className="flex min-w-0 items-center gap-2">
                <CircularLoader size="sm" className="shrink-0 border-background border-t-transparent" />
                <span className="shrink-0 text-xs text-background/75">#{index + 1}</span>
              </div>
              <div className="flex shrink-0 items-center gap-0.5">
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="text-background hover:bg-background/15 hover:text-background"
                  disabled={!activeRunId || !directPrompt?.trim()}
                  aria-label={`${index + 1}번 대기 prompt 즉시 전송`}
                  onClick={() => onSteerPrompt(queuedPrompt)}
                >
                  <PlayIcon className="size-3" />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="text-background hover:bg-background/15 hover:text-background"
                  aria-label={`${index + 1}번 대기 prompt 편집`}
                  onClick={() => onEditPrompt(queuedPrompt)}
                >
                  <PencilIcon className="size-3" />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="text-background hover:bg-background/15 hover:text-background"
                  disabled={index === 0}
                  aria-label={`${index + 1}번 대기 prompt 위로 이동`}
                  onClick={() => onMovePrompt(index, index - 1)}
                >
                  <ArrowUpIcon className="size-3" />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="text-background hover:bg-background/15 hover:text-background"
                  disabled={index === queuedPrompts.length - 1}
                  aria-label={`${index + 1}번 대기 prompt 아래로 이동`}
                  onClick={() => onMovePrompt(index, index + 1)}
                >
                  <ArrowDownIcon className="size-3" />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="text-background hover:bg-background/15 hover:text-background"
                  aria-label={`${index + 1}번 대기 prompt 제거`}
                  onClick={() => onRemovePrompt(queuedPrompt.id)}
                >
                  <XIcon className="size-3" />
                </Button>
              </div>
            </div>
            <div className="whitespace-pre-wrap break-words text-sm">{queuedPrompt.text}</div>
          </div>
        </Message>
      ))}
    </div>
  );
}

function addRunEventItem(items: TimelineItem[], runId: string, event: TimelineRunEvent) {
  return appendOneTimelineItem(items, toTimelineItem(runId, event));
}

function isIdleThreadStatusEvent(event: TimelineRunEvent) {
  if (event.type !== "raw" || event.method !== "session/update") {
    return false;
  }

  const payload = event.payload;
  if (!payload || typeof payload !== "object") {
    return false;
  }

  const update = "update" in payload ? (payload as { update?: unknown }).update : payload;
  if (!update || typeof update !== "object") {
    return false;
  }

  const sessionUpdate = (update as { sessionUpdate?: unknown }).sessionUpdate;
  if (sessionUpdate !== "session_info_update") {
    return false;
  }

  const meta = (update as { _meta?: unknown })._meta;
  if (!meta || typeof meta !== "object") {
    return false;
  }

  const codex = (meta as { codex?: unknown }).codex;
  if (!codex || typeof codex !== "object") {
    return false;
  }

  const threadStatus = (codex as { threadStatus?: unknown }).threadStatus;
  if (!threadStatus || typeof threadStatus !== "object") {
    return false;
  }

  return (threadStatus as { type?: unknown }).type === "idle";
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
  const locations = uniqueToolPaths(tool?.locations ?? []);
  const toolCallId = tool?.toolCallId;
  const fileChanges = tool?.fileChanges ?? [];
  const shouldShowLocationRows = fileChanges.length === 0;
  const defaultOpen = shouldOpenToolStepByDefault(item.title, fileChanges);

  return (
    <div>
      <Steps className="" defaultOpen={defaultOpen}>
        <StepsTrigger leftIcon={<ToolStatusIcon status={status} />} swapIconOnHover={false}>
          <span className="flex min-w-0 w-full flex-wrap items-start gap-x-2 gap-y-1 text-left">
            <span className="min-w-0 break-words text-left">{item.title || "tool"}</span>
          </span>
        </StepsTrigger>
        <StepsContent>
          {shouldShowLocationRows &&
            locations.map((path) => (
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
          {fileChanges.length > 0 && (
            <StepsItem className="block">
              <div className="space-y-2">
                {fileChanges.map((change) => (
                  <ToolFileChangeView
                    key={`${change.kind}:${change.path}:${change.oldPath ?? ""}`}
                    change={change}
                  />
                ))}
              </div>
            </StepsItem>
          )}
        </StepsContent>
      </Steps>
    </div>
  );
}

function shouldOpenToolStepByDefault(title: string, fileChanges: ToolFileChange[]) {
  if (fileChanges.length > 0) {
    return true;
  }
  return /^edit(?:ing)?\b/i.test(title.trim());
}

function uniqueToolPaths(paths: string[]) {
  return paths.filter((path, index, list) => list.indexOf(path) === index);
}

function ToolFileChangeView({ change }: { change: ToolFileChange }) {
  const content = change.diff ?? change.content;
  const fallback = toolFileChangeFallback(change);

  return (
    <details className="group rounded-md border bg-background" open>
      <summary className="flex cursor-pointer list-none flex-wrap items-center gap-2 px-3 py-2 text-sm">
        <code className="min-w-0 max-w-full rounded bg-muted px-1.5 py-0.5 font-mono text-xs">
          <EllipsisPopoverText
            value={change.path}
            className="font-mono text-xs"
            contentClassName="font-mono text-xs"
          />
        </code>
        {change.status !== "completed" && (
          <Badge variant={toolFileChangeStatusVariant(change.status)} className="shrink-0">
            {toolFileChangeStatusLabel(change.status)}
          </Badge>
        )}
        {change.truncated && (
          <Badge variant="secondary" className="shrink-0">
            truncated
          </Badge>
        )}
      </summary>
      <div className="border-t">
        {content ? (
          change.diff ? (
            <DiffViewer content={content} className="max-h-72 w-full rounded-none border-0 text-[11px]" />
          ) : (
            <pre className="max-h-72 w-full overflow-auto bg-muted/40 p-3 whitespace-pre-wrap break-words font-mono text-xs">
              {content}
            </pre>
          )
        ) : (
          <div className="bg-muted/40 p-3 text-xs text-muted-foreground">
            {fallback}
          </div>
        )}
      </div>
    </details>
  );
}

function toolFileChangeFallback(change: ToolFileChange) {
  if (change.message) {
    return change.message;
  }
  if (change.binary) {
    return "Binary content cannot be displayed.";
  }
  return "No text diff available.";
}

function toolFileChangeKindLabel(kind: ToolFileChange["kind"]) {
  const labels = {
    added: "added",
    modified: "modified",
    deleted: "deleted",
    renamed: "renamed",
    unknown: "unknown",
  } satisfies Record<ToolFileChange["kind"], string>;

  return labels[kind];
}

function toolFileChangeStatusLabel(status: ToolFileChange["status"]) {
  const labels = {
    inProgress: "in progress",
    completed: "completed",
    failed: "failed",
    unavailable: "unavailable",
  } satisfies Record<ToolFileChange["status"], string>;

  return labels[status];
}

function toolFileChangeStatusVariant(status: ToolFileChange["status"]) {
  if (status === "failed" || status === "unavailable") {
    return "destructive";
  }
  if (status === "completed") {
    return "secondary";
  }
  return "outline";
}

function ToolStepGroup({ items }: { items: TimelineItem[] }) {
  return (
    <div className="ml-11 space-y-3 border-l-[6px] border-border pl-2">
      {items.map((item) => (
        <ToolStep key={item.id} item={item} />
      ))}
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
