import {
  exchangeContinuation,
  exchangeDeliveryKey,
} from "@/features/agent-run/model/exchange-reconciler";
import type { KeyboardEvent, ReactNode, RefObject } from "react";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
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
  CheckCircleIcon,
  ChevronDownIcon,
  ClockIcon,
  InfoIcon,
  Loader2Icon,
  PanelRightCloseIcon,
  PanelRightOpenIcon,
  PencilIcon,
  PlayIcon,
  SettingsIcon,
  SquareIcon,
  XCircleIcon,
  XIcon,
} from "lucide-react";

import {
  cancelAgentRun,
  cancelCurrentPromptAndSendToRun,
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
  steerPromptToRun,
} from "@/entities/agent-run/api/agent-run-repository";
import { discardAgentExchangeDelivery } from "@/entities/agent-run/api/agent-exchange-repository";
import { unsettledCall } from "@/shared/api/transport";
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
  appendSessionLifecycleStatusMessage,
  availableCommandCandidatesFromSessionUpdate,
  clampHighlightedIndex,
  createSessionIdleLifecycleStatusMessage,
  createSessionStartLifecycleStatusMessage,
  eventGroups,
  filterPromptAutocompleteCandidates,
  findPromptAutocompleteTrigger,
  formatAvailableCommandsSummary,
  formatSessionFreshnessLabel,
  isAvailableCommandsSessionUpdate,
  isSessionInfoUpdateEvent,
  normalizeSessionUpdatedAt,
  readAvailableCommandMetadata,
  readAgentThreadStatus,
  readSessionInfoUpdateMetadata,
  replacePromptAutocompleteTrigger,
  projectTimelineToMinimapEntries,
  toTimelineItem,
} from "@/entities/agent-run/model";
import type { TimelineRunEvent } from "@/entities/agent-run/model";
import type {
  ContextSizePreset,
  EventGroup,
  AgentThreadStatus,
  AgentRunSessionMode,
  AgentRunSettings,
  AgentToolCommandCandidate,
  AvailableCommandMetadata,
  GoalStatus,
  PermissionMode,
  ProviderSession,
  RunEvent,
  ThreadGoal,
  TimelineItem,
  ToolFileChange,
} from "@/entities/agent-run/model";
import {
  applyPendingSeek,
  createPendingSeek,
  createViewportIndicator,
  EMPTY_TIMELINE_LAYOUT_SNAPSHOT,
  scrollTopForTimelineRatio,
  type MinimapSeekInput,
  type PendingSeek,
  type TimelineLayoutSnapshot,
} from "@/features/agent-run/model/agent-run-minimap";
import {
  buildGoalContinuationPrompt,
  shouldStartGoalContinuation,
} from "@/features/agent-run/model/goal-continuation";
import type {
  AgentPanelRunState,
  AgentPromptRequest,
  AgentRunPanelKind,
} from "@/features/agent-run/model/agent-run-panel-slots";
import {
  activateRunStartQueuedPrompt,
  addUserMessage,
  appendQueuedPrompt,
  appendPendingSteer,
  appendPromptHistory,
  buildSteerPrompt,
  createSteerInput,
  createQueuedPrompt,
  createRunStartQueuedPrompt,
  acceptPendingSteer,
  initialPromptHistoryState,
  hasUserMessage,
  isOverrideCommandFailure,
  isPromptHistoryNavigationBoundary,
  moveQueuedPrompt as reorderQueuedPrompt,
  navigatePromptHistory,
  moveRejectedSteerToQueue,
  partitionReplayedRunEvents,
  prepareQueuedPromptSteer,
  rejectPendingSteer,
  removeRejectedSteer,
  removeUserMessage,
  retryRejectedSteer,
  resolveRequestAgentLaunch,
  resolveSelectedProfileId,
  resetPromptHistoryCursor,
  shouldAutoDispatchQueuedPromptWithSteers,
  unsettledDispatchWasApplied,
  updateQueuedPrompt,
} from "@/features/agent-run/model/run-panel-state";
import type {
  PromptHistoryDirection,
  QueuedPrompt,
  QueuedPromptSource,
  SteerInput,
  UsageContext,
} from "@/features/agent-run/model/run-panel-state";
import {
  APP_COMMAND_OVERRIDE_SETTINGS_KEY,
  builtInProfileDefaultName,
  effectiveProfiles,
} from "@/features/agent-command-override/model/command-overrides";
import { formatSessionLabel } from "@/features/agent-run/model/session-label";
import { StreamingMarkdown } from "@/features/agent-run/ui/agent-run-markdown";
import { AgentRunMinimap } from "@/features/agent-run/ui/agent-run-minimap";
import { PermissionRequestDialog } from "@/features/agent-run/ui/permission-request-dialog";
import { PromptCommandAutocomplete } from "@/features/agent-run/ui/prompt-command-autocomplete";
import { SavedPromptToolbar } from "@/features/saved-prompt/ui/saved-prompt-toolbar";
import { dispatchMcpWindowTitle } from "@/shared/lib/workspace-window-title";
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
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";

type AgentRunPanelProps = {
  panelId?: string;
  workingDirectory: string;
  scrollHeader?: ReactNode;
  onRunSettled?: () => void;
  onRunStateChange?: (state: AgentPanelRunState) => void;
  onBeforeRunStart?: (runId: string) => Promise<void>;
  initialInputMode?: AgentInputMode;
  externalPromptRequest?: AgentPromptRequest | null;
  variant?: AgentRunPanelKind;
  onOpenSettings?: () => void;
  initialTimelineItems?: TimelineItem[];
  timelineItems?: TimelineItem[];
  replayedEvents?: unknown[];
  initialMinimapVisible?: boolean;
  showPromptComposer?: boolean;
  runConfigurationPortal?: HTMLElement | null;
  worktreeRunConfiguration?: WorktreeRunConfiguration | null;
  onWorktreeRunConfigurationChange?: (configuration: WorktreeRunConfiguration) => void;
  existingRunId?: string | null;
  existingIsRunning?: boolean;
  runtimeHydrated?: boolean;
  initialPermissionMode?: PermissionMode;
};

export type WorktreeRunConfiguration = {
  modelId: string;
  effortId: string;
};

type AgentInputMode = "prompt" | "ralphLoop";

export type { AgentPromptRequest };

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

const providerDefaultEffortOption: SelectOption = {
  value: "providerDefault",
  label: "Provider default",
  description: "Use the selected agent/provider default reasoning effort.",
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
  "gpt-5.6": "Use OpenAI's current flagship model for coding and reasoning.",
  "gpt-5.6-sol": "Use GPT-5.6 Sol for the highest capability.",
  "gpt-5.6-terra": "Use GPT-5.6 Terra to balance capability and cost.",
  "gpt-5.6-luna": "Use GPT-5.6 Luna for efficient, high-volume workloads.",
  "gpt-5.5": "Use OpenAI's previous flagship model for coding and reasoning.",
  "gpt-5.4": "Use OpenAI's more affordable current-generation model.",
  "gpt-5.4-mini": "Use OpenAI's lower-latency mini model.",
  "gpt-5.4-nano": "Use OpenAI's lowest-latency nano model.",
  "gpt-5.3-codex": "Use OpenAI's newer Codex model for coding tasks.",
  "gpt-5.3-codex-spark": "Use OpenAI's faster Codex Spark model when available.",
  "gpt-5.2-codex": "Use GPT-5.2 Codex when the selected provider advertises it.",
  "gpt-5.1-codex": "Use GPT-5.1 Codex when the selected provider advertises it.",
  "gpt-5-codex": "Use GPT-5 Codex when the selected provider advertises it.",
  best: "Use the best Claude model available to the current account.",
  fable: "Use Claude Fable for the hardest and longest-running tasks.",
  opus: "Use Claude Code's latest Opus alias.",
  sonnet: "Use Claude Code's latest Sonnet alias.",
  haiku: "Use Claude Code's fast and efficient Haiku alias.",
  opusplan: "Use Opus for planning and Sonnet for execution.",
  "opus[1m]": "Use the latest Opus model with a 1M-token context window.",
  "sonnet[1m]": "Use the latest Sonnet model with a 1M-token context window.",
  "claude-fable-5": "Use Claude Fable 5 for the most demanding agentic tasks.",
  "claude-opus-5": "Use Claude Opus 5 for complex agentic coding and long-running tasks.",
  "claude-opus-4-8": "Use the pinned Claude Opus 4.8 model.",
  "claude-sonnet-5": "Use Claude Sonnet 5 for a strong speed and capability balance.",
  "claude-sonnet-4-6": "Use the pinned Claude Sonnet 4.6 model.",
  "claude-haiku-4-5": "Use Claude's fast Haiku model.",
};

export const AgentRunPanel = memo(function AgentRunPanel({
  panelId = "agent-run",
  workingDirectory,
  scrollHeader,
  onRunSettled,
  onRunStateChange,
  onBeforeRunStart,
  initialInputMode = "prompt",
  externalPromptRequest = null,
  variant = "main",
  onOpenSettings,
  initialTimelineItems = [],
  timelineItems,
  replayedEvents = [],
  initialMinimapVisible = true,
  showPromptComposer = true,
  runConfigurationPortal = null,
  worktreeRunConfiguration = null,
  onWorktreeRunConfigurationChange,
  existingRunId,
  existingIsRunning = false,
  runtimeHydrated = true,
  initialPermissionMode = "default",
}: AgentRunPanelProps) {
  const enableGoalContinuation = variant === "main";
  const persistSettings = variant === "main";
  const queryClient = useQueryClient();
  const [selectedAgentId, setSelectedAgentId] = useState<string>("");
  const [sessionMode, setSessionMode] = useState<AgentRunSessionMode>("new");
  const [selectedSessionId, setSelectedSessionId] = useState<string>("");
  const [permissionMode, setPermissionMode] =
    useState<PermissionMode>(initialPermissionMode);
  const [isChangingPermissionMode, setIsChangingPermissionMode] = useState(false);
  const [modelId, setModelId] = useState("providerDefault");
  const [effortId, setEffortId] = useState("providerDefault");
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
  const [availableCommandMetadata, setAvailableCommandMetadata] =
    useState<AvailableCommandMetadata | null>(null);
  const [autocompleteSuppression, setAutocompleteSuppression] = useState<{
    text: string;
    cursorStart: number;
    cursorEnd: number;
  } | null>(null);
  const [promptHistory, setPromptHistory] = useState(initialPromptHistoryState);
  const [activeRunId, setActiveRunId] = useState<string | null>(
    existingRunId ?? null,
  );
  const [isRunning, setIsRunning] = useState(existingIsRunning);
  const [isAwaitingPromptResponse, setIsAwaitingPromptResponse] = useState(false);
  /** 답을 기다리는 취소 호출 수(Codex r10). turn 응답 대기(`isAwaitingPromptResponse`, run lifecycle이 정한다)와 다른 상태다:
   *  취소 중인 run에는 대기열을 보내지 않지만, 취소가 끝나지 않았다고 응답 대기 값을 호출 전으로 되돌리지 않는다. */
  const [cancelsInFlight, setCancelsInFlight] = useState(0);
  /** run lifecycle이 응답 대기를 바꾼 횟수(Codex r10): 호출 실패 때 되돌리기는 그 사이 lifecycle이 바꾸지 않았을 때만 한다. */
  const promptLifecycleSeqRef = useRef(0);
  /** run별로 관측한 `promptSent` 수(Codex r12): 결과를 모르는(unknown) 전송이 실제로 적용됐는지를 그 뒤의 turn 시작으로 판단한다. */
  const promptSentCountsRef = useRef(new Map<string, number>());
  /** unknown이 먼저 돌아온 직접 prompt(OCR r12): 늦은 `promptSent`가 오면 복원했던 입력을 다시 보내게 두지 않고 transcript를
   *  적용 상태로 맞춘다. 직접 prompt에는 멱등 키가 없으므로 자동 재전송하지 않는다. */
  const unsettledDirectPromptRef = useRef<{
    runId: string;
    text: string;
    promptSentBefore: number;
  } | null>(null);
  const [isPreparingRun, setIsPreparingRun] = useState(false);
  const [agentThreadStatus, setAgentThreadStatus] = useState<AgentThreadStatus>({
    type: "unknown",
  });
  const [sessionUpdatedAt, setSessionUpdatedAt] = useState<string | null>(null);
  const [directPrompt, setDirectPrompt] = useState<string | null>(null);
  const [queuedPrompts, setQueuedPrompts] = useState<QueuedPrompt[]>([]);
  const [pendingSteers, setPendingSteers] = useState<SteerInput[]>([]);
  const [rejectedSteers, setRejectedSteers] = useState<SteerInput[]>([]);
  const [filter, setFilter] = useState<EventGroup | "all">("all");
  const [items, setItems] = useState<TimelineItem[]>(initialTimelineItems);
  const [isMinimapVisible, setIsMinimapVisible] = useState(initialMinimapVisible);
  const [timelineLayout, setTimelineLayout] = useState<TimelineLayoutSnapshot>(
    EMPTY_TIMELINE_LAYOUT_SNAPSHOT,
  );
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
  const agentThreadStatusRef = useRef<AgentThreadStatus>({ type: "unknown" });
  const onRunStateChangeRef = useRef(onRunStateChange);
  const activeGoalRef = useRef<ThreadGoal | null>(null);
  const activePromptSentRef = useRef(false);
  const queuedPromptsRef = useRef<QueuedPrompt[]>([]);
  const pendingSteersRef = useRef<SteerInput[]>([]);
  const rejectedSteersRef = useRef<SteerInput[]>([]);
  /** 취소 결과를 모르는(unknown) 재시작(Codex r8): 그 run의 취소 끝 이벤트가 오면 한 번 이어 간다. 그 run이 새 turn을 받으면
   *  (살아 있다 — 취소가 적용되지 않았다) 버린다. */
  const unsettledRestartRef = useRef<{ attemptId: number; runId: string; resume: () => void } | null>(null);
  /** 지금 살아 있는 재시작 의도(Codex r9): 한 번의 "Full restart" 조작이 하나의 시도 id를 갖고, 재시작은 그 id로 **한 번만**
   *  소비된다(취소 성공 답, 결과를 몰랐던 취소의 복구된 끝 이벤트 중 먼저 온 쪽). 새 재시작·취소 조작은 앞 의도를 대체한다. */
  const restartIntentRef = useRef<{ attemptId: number; runId: string } | null>(null);
  const restartAttemptSeqRef = useRef(0);
  /** 끝난 run의 끝 이벤트가 비운 대기열(Codex r9): 취소 답보다 끝 이벤트가 먼저 오면 대기열은 이미 비어 있다 — 재시작·취소
   *  정리는 그 run의 마지막 대기열(취소를 기다리는 동안 들어온 항목 포함)로 판단한다. */
  const lastClearedQueueRef = useRef<{ runId: string; queue: QueuedPrompt[] } | null>(null);
  /** 마지막으로 끝난 run과 그 끝(Codex r8): 취소 호출의 결과보다 run 끝 이벤트가 먼저 올 수 있다(서버가 취소를 적용하면 끝
   *  이벤트를 먼저 보내고 답한다). 결과를 받은 쪽이 이 기록으로 run이 이미 끝났는지 본다. */
  const lastRunEndRef = useRef<{ runId: string; status: string } | null>(null);
  const steerSequenceRef = useRef(0);
  const runStartedAtRef = useRef<number | null>(null);
  const usageContextRef = useRef<UsageContext | null>(null);
  const goalContinuationPendingRef = useRef(false);
  const settingsHydratedRef = useRef(false);
  const timelineScrollRef = useRef<HTMLDivElement | null>(null);
  const promptTextareaElementRef = useRef<HTMLTextAreaElement | null>(null);
  const handledExternalPromptRequestIdRef = useRef<string | null>(null);
  const pendingMinimapSeekRef = useRef<PendingSeek | null>(null);
  const pendingMinimapResizeSeekRef = useRef<{
    targetRatio: number;
    requestedRevision: number;
  } | null>(null);
  const hiddenMinimapRatioRef = useRef<number | null>(null);
  const replayedEventCursorRef = useRef({ runId: "", count: 0 });

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
    enabled: persistSettings,
    ...agentRunSettingsQueryOptions,
  });
  const appCommandSettingsQuery = useQuery({
    queryKey: agentRunQueryKeys.settings(APP_COMMAND_OVERRIDE_SETTINGS_KEY),
    queryFn: () => getAgentRunSettings(APP_COMMAND_OVERRIDE_SETTINGS_KEY),
    ...agentRunSettingsQueryOptions,
  });

  useEffect(() => {
    if (timelineItems) {
      setItems(timelineItems);
    }
  }, [timelineItems]);

  useEffect(() => {
    if (!existingRunId) return;
    if (replayedEventCursorRef.current.runId !== existingRunId) {
      replayedEventCursorRef.current = { runId: existingRunId, count: 0 };
    }
    const nextEvents = replayedEvents.slice(
      replayedEventCursorRef.current.count,
    );
    if (nextEvents.length === 0) return;
    const replay = partitionReplayedRunEvents(nextEvents);
    if (replay.usageContext) {
      setUsageContext(replay.usageContext);
      usageContextRef.current = replay.usageContext;
    }
    if (replay.timelineEvents.length > 0) {
      setItems((currentItems) =>
        replay.timelineEvents.reduce(
          (nextItems, event) =>
            addRunEventItem(nextItems, existingRunId, event),
          currentItems,
        ),
      );
    }
    replayedEventCursorRef.current.count = replayedEvents.length;
  }, [existingRunId, replayedEvents]);
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
    enabled: enableGoalContinuation,
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
    if (!enableGoalContinuation) {
      runStartedAtRef.current = null;
      return;
    }
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
  }, [enableGoalContinuation, goalQueryKey, queryClient, workingDirectory]);

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
    if (!persistSettings) {
      settingsHydratedRef.current = true;
      return;
    }
    if (settingsHydratedRef.current || settingsQuery.isLoading) {
      return;
    }
    if (settingsQuery.isError) {
      setError(String(settingsQuery.error));
      return;
    }

    const savedSettings = settingsQuery.data;
    let hydratedRunConfiguration: WorktreeRunConfiguration = worktreeRunConfiguration ?? {
      modelId: "providerDefault",
      effortId: "providerDefault",
    };
    if (savedSettings) {
      hydratedRunConfiguration =
        worktreeRunConfiguration ??
        {
          modelId: isModelOptionValue(savedSettings.modelId)
            ? savedSettings.modelId
            : "providerDefault",
          effortId: isModelOptionValue(savedSettings.effortId)
            ? savedSettings.effortId
            : "providerDefault",
        };
      setSelectedAgentId(savedSettings.agentId);
      setPermissionMode(savedSettings.permissionMode);
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
    setModelId(hydratedRunConfiguration.modelId);
    setEffortId(hydratedRunConfiguration.effortId);

    settingsHydratedRef.current = true;
    onWorktreeRunConfigurationChange?.(hydratedRunConfiguration);
  }, [
    onWorktreeRunConfigurationChange,
    settingsQuery.data,
    settingsQuery.error,
    settingsQuery.isError,
    settingsQuery.isLoading,
    persistSettings,
    worktreeRunConfiguration,
  ]);

  useEffect(() => {
    if (!settingsHydratedRef.current || !worktreeRunConfiguration) {
      return;
    }
    setModelId(worktreeRunConfiguration.modelId);
    setEffortId(worktreeRunConfiguration.effortId);
  }, [
    worktreeRunConfiguration,
    worktreeRunConfiguration?.effortId,
    worktreeRunConfiguration?.modelId,
  ]);

  const changeModelId = useCallback(
    (nextModelId: string) => {
      setModelId(nextModelId);
      onWorktreeRunConfigurationChange?.({ modelId: nextModelId, effortId });
    },
    [effortId, onWorktreeRunConfigurationChange],
  );

  const changeEffortId = useCallback(
    (nextEffortId: string) => {
      setEffortId(nextEffortId);
      onWorktreeRunConfigurationChange?.({ modelId, effortId: nextEffortId });
    },
    [modelId, onWorktreeRunConfigurationChange],
  );

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
    if (!persistSettings) {
      return;
    }
    if (!settingsHydratedRef.current || !selectedAgentId || !workingDirectory.trim()) {
      return;
    }

    const settings: AgentRunSettings = {
      workingDirectory,
      agentId: selectedAgentId,
      permissionMode,
      modelId,
      effortId,
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
    effortId,
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
    persistSettings,
  ]);

  useEffect(() => {
    activeRunIdRef.current = activeRunId;
    setAvailableCommandCandidates([]);
    setAvailableCommandMetadata(null);
    if (!activeRunId) {
      setSessionUpdatedAt(null);
    }
  }, [activeRunId]);

  useEffect(() => {
    if (existingRunId === undefined) return;
    setActiveRunId(existingRunId);
    activeRunIdRef.current = existingRunId;
    setIsRunning(existingIsRunning);
  }, [existingIsRunning, existingRunId]);

  useEffect(() => {
    agentThreadStatusRef.current = agentThreadStatus;
  }, [agentThreadStatus]);

  useEffect(() => {
    onRunStateChangeRef.current = onRunStateChange;
  }, [onRunStateChange]);

  useEffect(() => {
    if (existingRunId !== undefined && !runtimeHydrated) return;
    onRunStateChangeRef.current?.({ panelId, isRunning, activeRunId });
  }, [
    activeRunId,
    existingRunId,
    isRunning,
    panelId,
    runtimeHydrated,
  ]);

  useEffect(() => {
    queuedPromptsRef.current = queuedPrompts;
  }, [queuedPrompts]);

  useEffect(() => {
    pendingSteersRef.current = pendingSteers;
  }, [pendingSteers]);

  useEffect(() => {
    rejectedSteersRef.current = rejectedSteers;
  }, [rejectedSteers]);

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
    if (existingRunId !== undefined) {
      return;
    }
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
      if (isSessionInfoUpdateEvent(timelineEvent)) {
        const metadata = readSessionInfoUpdateMetadata(timelineEvent);
        const nextThreadStatus = readAgentThreadStatus(timelineEvent);
        if (metadata?.title) {
          dispatchMcpWindowTitle(metadata.title);
        }
        const nextSessionUpdatedAt = normalizeSessionUpdatedAt(metadata?.updatedAt);
        if (nextSessionUpdatedAt) {
          setSessionUpdatedAt(nextSessionUpdatedAt);
        }
        const statusMessages =
          nextThreadStatus?.type === "active"
            ? [createSessionStartLifecycleStatusMessage(envelope.runId)]
            : [
                createSessionIdleLifecycleStatusMessage({
                  runId: envelope.runId,
                  previousStatus: agentThreadStatusRef.current,
                  nextStatus: nextThreadStatus,
                }),
              ];
        setItems((currentItems) =>
          statusMessages.reduce(
            (nextItems, message) =>
              appendSessionLifecycleStatusMessage(nextItems, envelope.runId, message),
            currentItems,
          ),
        );
        if (nextThreadStatus) {
          agentThreadStatusRef.current = nextThreadStatus;
          setAgentThreadStatus(nextThreadStatus);
        }
        if (nextThreadStatus?.type === "idle") {
          setAwaitingFromLifecycle(false);
        }
        return;
      }

      if (timelineEvent.type === "raw") {
        const nextAvailableCommandMetadata = readAvailableCommandMetadata(
          timelineEvent.payload,
        );
        const nextCandidates = availableCommandCandidatesFromSessionUpdate(
          timelineEvent.payload,
          {
            runId: envelope.runId,
            agentId: providerAgentId,
            workingDirectory,
          },
        );
        if (isAvailableCommandsSessionUpdate(timelineEvent.payload)) {
          setAvailableCommandMetadata(nextAvailableCommandMetadata);
          setAvailableCommandCandidates(nextCandidates);
          return;
        }
      }

      setItems((currentItems) =>
        addRunEventItem(currentItems, envelope.runId, timelineEvent),
      );

      if (timelineEvent.type === "error") {
        if (unsettledDirectPromptRef.current?.runId === envelope.runId) {
          unsettledDirectPromptRef.current = null;
        }
        lastClearedQueueRef.current = { runId: envelope.runId, queue: queuedPromptsRef.current };
        setAwaitingFromLifecycle(false);
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
          const promptSentCount = (promptSentCountsRef.current.get(envelope.runId) ?? 0) + 1;
          promptSentCountsRef.current.set(envelope.runId, promptSentCount);
          const unsettledDirectPrompt = unsettledDirectPromptRef.current;
          if (
            unsettledDirectPrompt?.runId === envelope.runId &&
            promptSentCount > unsettledDirectPrompt.promptSentBefore
          ) {
            unsettledDirectPromptRef.current = null;
            setPrompt((current) => (current === unsettledDirectPrompt.text ? defaultPrompt : current));
            setDirectPrompt(unsettledDirectPrompt.text);
            setItems((currentItems) =>
              hasUserMessage(currentItems, envelope.runId, unsettledDirectPrompt.text)
                ? currentItems
                : addUserMessage(currentItems, envelope.runId, unsettledDirectPrompt.text),
            );
            recordPromptHistory(unsettledDirectPrompt.text);
            setError(null);
          }
          if (unsettledRestartRef.current?.runId === envelope.runId) {
            // 새 turn을 받았다: run은 살아 있고 취소는 적용되지 않았다 — 재시작을 잇지 않는다(의도도 버린다).
            const dropped = unsettledRestartRef.current;
            unsettledRestartRef.current = null;
            if (restartIntentRef.current?.attemptId === dropped.attemptId) {
              restartIntentRef.current = null;
            }
          }
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
          setAwaitingFromLifecycle(true);
        }
        if (timelineEvent.status === "promptCompleted") {
          setAwaitingFromLifecycle(false);
        }
        if (timelineEvent.status === "steerAccepted") {
          const [accepted] = pendingSteersRef.current;
          if (accepted) {
            const next = acceptPendingSteer(pendingSteersRef.current, accepted.id);
            pendingSteersRef.current = next;
            setPendingSteers(next);
          }
          setAwaitingFromLifecycle(false);
        }
        if (timelineEvent.status === "steerRejected") {
          const [rejected] = pendingSteersRef.current;
          if (rejected) {
            const result = rejectPendingSteer({
              pendingSteers: pendingSteersRef.current,
              rejectedSteers: rejectedSteersRef.current,
              steerInputId: rejected.id,
              reason: timelineEvent.message,
            });
            pendingSteersRef.current = result.pendingSteers;
            rejectedSteersRef.current = result.rejectedSteers;
            setPendingSteers(result.pendingSteers);
            setRejectedSteers(result.rejectedSteers);
          }
          setAwaitingFromLifecycle(false);
        }
        if (["completed", "cancelled"].includes(timelineEvent.status)) {
          if (unsettledDirectPromptRef.current?.runId === envelope.runId) {
            unsettledDirectPromptRef.current = null;
          }
          lastClearedQueueRef.current = { runId: envelope.runId, queue: queuedPromptsRef.current };
          setAwaitingFromLifecycle(false);
          setQueuedPrompts([]);
          setPendingSteers([]);
          setRejectedSteers([]);
          setDirectPrompt(null);
          setIsRunning(false);
          activePromptSentRef.current = false;
          activeRunIdRef.current = null;
          setActiveRunId(null);
          onRunSettled?.();
          void recordRunGoalProgress();
          lastRunEndRef.current = { runId: envelope.runId, status: timelineEvent.status };
          const unsettledRestart = unsettledRestartRef.current;
          if (unsettledRestart?.runId === envelope.runId) {
            unsettledRestartRef.current = null;
            // 결과를 몰랐던 취소가 실제로 적용됐다(취소 끝): 재시작을 한 번 이어 간다. 스스로 끝난 run은 잇지 않는다.
            if (timelineEvent.status === "cancelled") {
              unsettledRestart.resume();
            }
          }
        }
      }
    });

    return () => {
      unlisten();
    };
  }, [
    existingRunId,
    onRunSettled,
    providerAgentId,
    recordRunGoalProgress,
    workingDirectory,
  ]);

  useEffect(() => {
    if (
      !activeRunId ||
      !isRunning ||
      isAwaitingPromptResponse ||
      unsettledDirectPromptRef.current !== null ||
      cancelsInFlight > 0 ||
      !shouldAutoDispatchQueuedPromptWithSteers({ queue: queuedPrompts, pendingSteers })
    ) {
      return;
    }

    const nextPrompt = queuedPrompts[0];
    if (!queuedPromptBelongsToRun(nextPrompt, activeRunId)) {
      // 다른 run이 대상인 교환(Codex r11): 이 run에 보내면 서버가 거절한다 — 보내지 않고 뺀다.
      setQueuedPrompts((current) => current.filter((item) => item.id !== nextPrompt.id));
      return;
    }
    if (unsettledDispatchWasApplied(nextPrompt, promptSentCount)) {
      // 결과를 몰랐던 앞 전송이 그 뒤 관측된 turn으로 적용됐다(Codex r12): 다시 보내지 않는다 — 같은 키 재전송은 저장된 결과만
      // 재생해 새 turn 이벤트가 없어 응답 대기가 풀리지 않는다. 대화에는 보낸 prompt로 남긴다.
      setQueuedPrompts((current) => current.filter((item) => item.id !== nextPrompt.id));
      setItems((currentItems) => addUserMessage(currentItems, activeRunId, nextPrompt.text));
      recordPromptHistory(nextPrompt.text);
      return;
    }
    const previousDirectPrompt = directPrompt;
    const lifecycleSeq = promptLifecycleSeqRef.current;
    const dispatchRunId = activeRunId;
    const promptSentBefore = promptSentCount(activeRunId);
    setIsAwaitingPromptResponse(true);
    setQueuedPrompts((current) => current.slice(1));
    setDirectPrompt(nextPrompt.text);
    setItems((currentItems) =>
      addUserMessage(currentItems, activeRunId, nextPrompt.text),
    );
    void sendPromptToRun(
      activeRunId,
      nextPrompt.text,
      nextPrompt.idempotencyKey ? { idempotencyKey: nextPrompt.idempotencyKey } : undefined,
      nextPrompt.exchangeRequestId ? exchangeContinuation(nextPrompt.exchangeRequestId) : undefined,
    )
      .then(() => {
        recordPromptHistory(nextPrompt.text);
      })
      .catch((caughtError) => {
        const unsettled = unsettledCall(caughtError);
        if (unsettled === "unknown" && promptSentCount(dispatchRunId) > promptSentBefore) {
          // 결과는 몰랐지만 그 사이 이 run의 turn 시작을 봤다(Codex r12): 전송이 적용됐다 — 다시 넣지 않고, 대화·응답 대기는
          // lifecycle이 정한 대로 둔다.
          recordPromptHistory(nextPrompt.text);
          return;
        }
        // 교환 항목은 그 run에 묶인다(Codex r11): 서버가 거절했거나(서버의 답인 오류 — 다시 보내도 같다) 그 사이 run이 바뀌었으면
        // 대기열에 다시 넣지 않는다(선두에서 계속 거절돼 뒤 prompt를 막는다). 서버에 닿지 않았거나 결과를 모르는 전달만 다시 넣는다.
        const requeue =
          !nextPrompt.exchangeRequestId ||
          (unsettled !== null && activeRunIdRef.current === dispatchRunId);
        if (requeue) {
          // 결과를 모르면 앞 전송의 기준을 남긴다(Codex r12): 다시 보내기 전에 그 turn이 관측되면 적용된 것으로 본다. 여러 번
          // 실패해도 처음 전송의 기준을 쓴다.
          const retried: QueuedPrompt =
            unsettled === "unknown"
              ? {
                  ...nextPrompt,
                  unsettledDispatch: nextPrompt.unsettledDispatch ?? {
                    runId: dispatchRunId,
                    promptSentBefore,
                  },
                }
              : nextPrompt;
          setQueuedPrompts((current) => [retried, ...current]);
        }
        setItems((currentItems) =>
          removeUserMessage(currentItems, activeRunId, nextPrompt.text),
        );
        setDirectPrompt(previousDirectPrompt);
        restoreAwaitingIfUnchanged(lifecycleSeq, false);
        setError(String(caughtError));
      });
  }, [
    activeRunId,
    cancelsInFlight,
    directPrompt,
    isAwaitingPromptResponse,
    isRunning,
    pendingSteers,
    queuedPrompts,
  ]);

  useEffect(() => {
    if (!enableGoalContinuation) {
      goalContinuationPendingRef.current = false;
      return;
    }
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
    enableGoalContinuation,
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
  const effortOptions = useMemo<SelectOption[]>(() => {
    const advertisedEfforts = selectedAgent?.efforts ?? [];
    return [
      providerDefaultEffortOption,
      ...advertisedEfforts.map((effort) => ({
        value: effort.id,
        label: effort.label,
        description: `Use ${effort.label} reasoning effort with ${selectedAgent?.label ?? "the selected agent"}.`,
      })),
    ];
  }, [selectedAgent]);
  // 모델·effort 컨트롤은 선택된 agent가 실제로 광고하는 경우에만 노출한다.
  const supportsModelSelection = (selectedAgent?.models?.length ?? 0) > 0;
  const supportsEffortSelection = (selectedAgent?.efforts?.length ?? 0) > 0;
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
    if (agentsQuery.isLoading || appCommandSettingsQuery.isLoading || !selectedAgent) {
      return;
    }
    if (!modelOptions.some((option) => option.value === modelId)) {
      changeModelId(providerDefaultModelOption.value);
    }
    if (!effortOptions.some((option) => option.value === effortId)) {
      changeEffortId(providerDefaultEffortOption.value);
    }
    if (!contextSizeOptions.some((option) => option.value === contextSize)) {
      setContextSize(defaultContextSizeOption.value);
    }
  }, [
    agentsQuery.isLoading,
    appCommandSettingsQuery.isLoading,
    changeEffortId,
    changeModelId,
    contextSize,
    contextSizeOptions,
    effortId,
    effortOptions,
    modelId,
    modelOptions,
    selectedAgent,
  ]);
  const selectedPermissionModeOption = permissionModeOptions.find(
    (option) => option.value === permissionMode,
  );
  const selectedModelOption = modelOptions.find((option) => option.value === modelId);
  const selectedEffortOption = effortOptions.find((option) => option.value === effortId);
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
  const autocompletePrefixCandidates = useMemo(
    () =>
      autocompleteTrigger
        ? filterPromptAutocompleteCandidates(autocompleteSourceCandidates, {
            prefix: autocompleteTrigger.prefix,
            query: "",
          })
        : [],
    [autocompleteSourceCandidates, autocompleteTrigger],
  );
  const autocompleteCandidates = useMemo(
    () =>
      autocompleteTrigger
        ? filterPromptAutocompleteCandidates(
            autocompleteSourceCandidates,
            autocompleteTrigger,
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
    if (autocompletePrefixCandidates.length === 0) {
      return "empty" as const;
    }
    return "noMatch" as const;
  }, [
    autocompleteCandidates.length,
    autocompletePrefixCandidates.length,
    autocompleteTrigger,
    toolCommandCandidatesQuery.isError,
    toolCommandCandidatesQuery.isLoading,
  ]);
  const isAutocompleteOpen = Boolean(autocompleteTrigger);
  const visibleItems = useMemo(
    () => (filter === "all" ? items : items.filter((item) => item.group === filter)),
    [filter, items],
  );
  const minimapEntries = useMemo(() => projectTimelineToMinimapEntries(items), [items]);
  const handleTimelineLayoutChange = useCallback((snapshot: TimelineLayoutSnapshot) => {
    setTimelineLayout(snapshot);
  }, []);
  const performMinimapSeek = useCallback(
    (targetRatio: number, snapshot: TimelineLayoutSnapshot) => {
      const scrollElement = timelineScrollRef.current;
      if (!scrollElement) {
        return;
      }
      const maxScrollTop = Math.max(
        0,
        scrollElement.scrollHeight - scrollElement.clientHeight,
      );
      scrollElement.scrollTop = scrollTopForTimelineRatio(
        snapshot,
        targetRatio,
        maxScrollTop,
      );
      scrollElement.dispatchEvent(new Event("scroll"));
    },
    [],
  );
  const handleMinimapSeek = useCallback(
    (targetRatio: number, _input: MinimapSeekInput) => {
      if (filter !== "all") {
        pendingMinimapSeekRef.current = createPendingSeek(
          targetRatio,
          timelineLayout.revision,
        );
        setFilter("all");
        return;
      }
      performMinimapSeek(targetRatio, timelineLayout);
    },
    [filter, performMinimapSeek, timelineLayout],
  );
  const handleMinimapVisibilityToggle = useCallback(() => {
    const visibleRatio = createViewportIndicator(timelineLayout).startRatio;
    const currentRatio = isMinimapVisible
      ? visibleRatio
      : (hiddenMinimapRatioRef.current ?? visibleRatio);
    if (isMinimapVisible) {
      hiddenMinimapRatioRef.current = currentRatio;
    }
    pendingMinimapResizeSeekRef.current = {
      targetRatio: currentRatio,
      requestedRevision: timelineLayout.revision,
    };
    setIsMinimapVisible((visible) => !visible);
  }, [isMinimapVisible, timelineLayout]);

  useEffect(() => {
    const pending = pendingMinimapResizeSeekRef.current;
    if (!pending || timelineLayout.revision <= pending.requestedRevision) {
      return;
    }
    pendingMinimapResizeSeekRef.current = null;
    performMinimapSeek(pending.targetRatio, timelineLayout);
  }, [performMinimapSeek, timelineLayout]);

  useEffect(() => {
    const result = applyPendingSeek(
      pendingMinimapSeekRef.current,
      filter,
      timelineLayout,
    );
    pendingMinimapSeekRef.current = result.pending;
    if (result.targetRatio !== null) {
      performMinimapSeek(result.targetRatio, timelineLayout);
    }
  }, [filter, performMinimapSeek, timelineLayout]);
  const usagePercent =
    usageContext && usageContext.size > 0
      ? Math.min(100, Math.round((usageContext.used / usageContext.size) * 100))
      : null;
  const sessionFreshnessLabel = formatSessionFreshnessLabel(sessionUpdatedAt);
  const availableCommandsSummary = formatAvailableCommandsSummary(
    availableCommandMetadata,
  );
  const pendingPermission = useMemo(() => findPendingPermission(items), [items]);
  const canStartRun = Boolean(
    selectedAgentId &&
      !isRunning &&
      !isPreparingRun &&
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
      unsettledDirectPromptRef.current === null &&
      queuedPrompts.length > 0,
  );
  const shouldSendDirectPrompt = Boolean(
      activeRunId &&
      isRunning &&
      !isAwaitingPromptResponse &&
      unsettledDirectPromptRef.current === null &&
      queuedPrompts.length === 0,
  );
  const canSendPrompt = shouldQueueSendPrompt || shouldSendDirectPrompt
    ? canQueuePrompt
    : canSteerPrompt;
  const canCancel = Boolean(activeRunId && isRunning);
  const isRunConfigurationLocked = isRunning || isPreparingRun;

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
    // 교환 prompt는 run 전송에 요청 id로 만든 키를 싣는다 — 원장이 없는 새 창에서 다시 라우팅돼도 같은 세대에서는
    // agent에 한 번만 간다(043 T036).
    const deliveryKey = externalPromptRequest.exchangeRequestId
      ? exchangeDeliveryKey(externalPromptRequest.exchangeRequestId)
      : undefined;

    if (externalPromptRequest.delivery === "draft") {
      handledExternalPromptRequestIdRef.current = externalPromptRequest.id;
      setInputMode("prompt");
      setRalphLoopEnabled(false);
      setIsRalphSettingsDialogOpen(false);
      setPrompt(nextPrompt);
      return;
    }

    if (externalPromptRequest.delivery === "queue") {
      handledExternalPromptRequestIdRef.current = externalPromptRequest.id;
      setInputMode("prompt");
      setRalphLoopEnabled(false);
      setIsRalphSettingsDialogOpen(false);
      enqueuePrompt(nextPrompt, "external-request", deliveryKey, externalPromptRequest.exchangeRequestId);
      return;
    }

    if (activeRunIdRef.current && isRunning) {
      handledExternalPromptRequestIdRef.current = externalPromptRequest.id;
      enqueuePrompt(nextPrompt, "external-request", deliveryKey, externalPromptRequest.exchangeRequestId);
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
      idempotencyKey: deliveryKey,
    }).then((started) => {
      if (!started) {
        setPrompt(nextPrompt);
      }
    });
  }, [externalPromptRequest, isRunning, selectedAgentId, selectedSessionId, sessionMode]);

  async function run() {
    const goal = prompt.trim();
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
      /** run 시작 멱등성 키(043: 교환 prompt의 즉시 전달). */
      idempotencyKey?: string;
    } = {},
  ) {
    if (!selectedAgentId) {
      return false;
    }

    const runId = crypto.randomUUID();
    const displayPrompt = options.displayPrompt ?? goal;
    if (onBeforeRunStart) {
      setIsPreparingRun(true);
      try {
        await onBeforeRunStart(runId);
      } catch (caughtError) {
        setError(String(caughtError));
        setPrompt(displayPrompt);
        return false;
      } finally {
        setIsPreparingRun(false);
      }
    }
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
    setAgentThreadStatus({ type: "unknown" });
    agentThreadStatusRef.current = { type: "unknown" };
    setSessionUpdatedAt(null);
    setAvailableCommandMetadata(null);
    queuedPromptsRef.current = nextQueuedPrompts;
    setQueuedPrompts(nextQueuedPrompts);
    pendingSteersRef.current = [];
    rejectedSteersRef.current = [];
    setPendingSteers([]);
    setRejectedSteers([]);
    setUsageContext(null);
    usageContextRef.current = null;
    runStartedAtRef.current = Date.now();
    setItems((currentItems) =>
      appendSessionLifecycleStatusMessage(
        currentItems,
        runId,
        createSessionStartLifecycleStatusMessage(runId),
      ),
    );
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
      await startAgentRun(
        {
          runId,
          goal,
          agentId: launch?.agentId ?? selectedAgentId,
          cwd: workingDirectory,
          ...(launch?.agentCommand ? { agentCommand: launch.agentCommand } : {}),
          ...(launch?.agentEnv ? { agentEnv: launch.agentEnv } : {}),
          stdioBufferLimitMb: 50,
          permissionMode,
          ...(modelId !== "providerDefault" ? { modelId } : {}),
          ...(effortId !== "providerDefault" ? { effortId } : {}),
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
        },
        panelId,
        options.idempotencyKey ? { idempotencyKey: options.idempotencyKey } : undefined,
      );
      recordPromptHistory(displayPrompt);
      return true;
    } catch (caughtError) {
      setError(String(caughtError));
      setPrompt(displayPrompt);
      setItems((currentItems) => removeUserMessage(currentItems, runId, displayPrompt));
      queuedPromptsRef.current = [];
      setQueuedPrompts([]);
      pendingSteersRef.current = [];
      rejectedSteersRef.current = [];
      setPendingSteers([]);
      setRejectedSteers([]);
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
    idempotencyKey?: string,
    exchangeRequestId?: string,
  ) {
    const nextPrompt = promptText.trim();
    if (!nextPrompt) {
      return;
    }

    setQueuedPrompts((current) => {
      const next = appendQueuedPrompt(
        current,
        createQueuedPrompt({
          id: crypto.randomUUID(),
          text: nextPrompt,
          source,
          idempotencyKey,
          exchangeRequestId,
          exchangeRunId: exchangeRequestId ? (activeRunIdRef.current ?? undefined) : undefined,
        }),
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

  /** 화면 대기열에서 버린 교환 prompt의 전달 포기(Codex r7). 서버에는 이미 확인(`delivered`)된 교환이라, 알리지 않으면
   *  대상 run이 살아 있는 동안 미소비 교환으로 남아 wait-stop이 끝나지 않는다. 실패는 오류로 보이고 다시 시도하지 않는다. */
  async function discardExchangeDeliveries(requestIds: string[]) {
    for (const requestId of requestIds) {
      try {
        await discardAgentExchangeDelivery(requestId);
      } catch (caughtError) {
        setError(`에이전트 메시지 전달 포기 실패: ${String(caughtError)}`);
      }
    }
  }

  /** 대기 prompt 제거. 교환 항목이면 먼저 대기열에서 빼고(자동 전송이 집어 가지 못하게) 서버에 전달 포기를 알린다. 포기가
   *  실패하면 서버에는 아직 전달할 교환이므로 항목을 제자리로 되돌린다. */
  async function removeQueuedPrompt(queuedPromptId: string) {
    const index = queuedPromptsRef.current.findIndex((item) => item.id === queuedPromptId);
    if (index < 0) {
      return;
    }
    const removed = queuedPromptsRef.current[index];
    const removalRunId = activeRunIdRef.current;
    const next = queuedPromptsRef.current.filter((item) => item.id !== queuedPromptId);
    queuedPromptsRef.current = next;
    setQueuedPrompts(next);
    if (editingPrompt?.id === queuedPromptId) {
      closeQueuedPromptEditor();
    }
    if (!removed.exchangeRequestId) {
      return;
    }
    try {
      await discardAgentExchangeDelivery(removed.exchangeRequestId);
    } catch (caughtError) {
      setError(`에이전트 메시지 전달 포기 실패: ${String(caughtError)}`);
      // 되돌리기는 지울 때의 run이 아직 활성이고 그 교환이 그 run에 묶였을 때만(Codex r11). 그 사이 run이 끝나거나 바뀌었으면
      // 그 교환은 끝난 run이 대상이다 — 새 run의 대기열에 넣지 않는다(서버는 대상 run이 없는 교환을 세지 않는다).
      if (activeRunIdRef.current !== removalRunId || !queuedPromptBelongsToRun(removed, removalRunId)) {
        return;
      }
      setQueuedPrompts((current) => {
        const restored = [...current.slice(0, index), removed, ...current.slice(index)];
        queuedPromptsRef.current = restored;
        return restored;
      });
    }
  }

  /** run lifecycle이 정한 응답 대기(Codex r10): 이벤트 처리기만 부른다. */
  /** 그 run에서 관측한 `promptSent` 수(Codex r12). */
  function promptSentCount(runId: string) {
    return promptSentCountsRef.current.get(runId) ?? 0;
  }

  function setAwaitingFromLifecycle(value: boolean) {
    promptLifecycleSeqRef.current += 1;
    setIsAwaitingPromptResponse(value);
  }

  /** 이 조작이 바꾼 응답 대기를 되돌린다 — 그 사이 run lifecycle이 바꿨으면(turn이 끝남·시작함) 최신 값을 둔다(Codex r10). */
  function restoreAwaitingIfUnchanged(sinceSeq: number, value: boolean) {
    if (promptLifecycleSeqRef.current === sinceSeq) {
      setIsAwaitingPromptResponse(value);
    }
  }

  /** 취소 호출 하나(Codex r10): 답을 받을 때까지 취소 진행으로 센다 — 자동 전송이 취소 중인 run에 보내지 않는다. */
  async function trackCancel<T>(call: () => Promise<T>): Promise<T> {
    setCancelsInFlight((count) => count + 1);
    try {
      return await call();
    } finally {
      setCancelsInFlight((count) => count - 1);
    }
  }

  /** 재시작 의도를 그 시도 id로 한 번만 소비한다(Codex r9). 대체됐거나 이미 소비됐으면 false. */
  function claimRestart(attemptId: number) {
    if (restartIntentRef.current?.attemptId !== attemptId) {
      return false;
    }
    restartIntentRef.current = null;
    unsettledRestartRef.current = null;
    return true;
  }

  /** 그 run의 지금 대기열: 끝 이벤트가 이미 비웠으면 비우기 직전의 대기열(Codex r9). */
  function queueOfRun(runId: string) {
    const cleared = lastClearedQueueRef.current;
    if (cleared?.runId === runId && activeRunIdRef.current !== runId) {
      return cleared.queue;
    }
    return queuedPromptsRef.current;
  }

  async function cancel() {
    if (!activeRunId) {
      return;
    }

    // 사용자가 취소를 골랐다: 살아 있는 재시작 의도(결과를 몰랐던 앞 재시작 포함)는 버린다(Codex r9).
    restartIntentRef.current = null;
    unsettledRestartRef.current = null;
    const runIdToCancel = activeRunId;
    try {
      await trackCancel(() => cancelAgentRun(runIdToCancel));
    } catch (caughtError) {
      setError(String(caughtError));
      // 취소가 서버에 닿았는지 모른다(Codex r8): 보내지 않았거나(notApplied) 답을 못 받았다(unknown). run은 살아 있을 수
      // 있으므로 대기열(교환 항목 포함)과 run을 그대로 둔다 — 살아 있으면 turn 끝에 교환이 전달되고, 실제로 취소됐으면
      // 복구된 run 끝 이벤트가 패널을 정리한다(대상 run이 없는 교환은 서버가 세지 않는다).
      if (unsettledCall(caughtError)) {
        return;
      }
    }
    if (unsettledDirectPromptRef.current?.runId === runIdToCancel) {
      // 사용자가 명시적으로 취소했고 서버가 결과를 확정했다. 늦은 promptSent를 기다리지 않아도 이 run에 새 prompt가 적용될 수 없다.
      unsettledDirectPromptRef.current = null;
    }
    // 취소로 버리는 대기열의 교환 항목은 서버에서도 끝낸다 — 서버가 거절해 run이 살아 있으면 확인된 미소비 교환이 남아
    // wait-stop을 막는다(Codex r7). 대기열은 답을 받은 지금의 것이다(기다리는 동안 들어온 교환 포함, Codex r9).
    const droppedExchanges = exchangeRequestIdsOf(queueOfRun(runIdToCancel));
    await recordRunGoalProgress();
    queuedPromptsRef.current = [];
    setQueuedPrompts([]);
    void discardExchangeDeliveries(droppedExchanges);
    pendingSteersRef.current = [];
    rejectedSteersRef.current = [];
    setPendingSteers([]);
    setRejectedSteers([]);
    setDirectPrompt(null);
    setIsAwaitingPromptResponse(false);
    setIsRunning(false);
    activeRunIdRef.current = null;
    setActiveRunId(null);
  }

  async function changePermissionMode(nextMode: PermissionMode) {
    if (nextMode === permissionMode) {
      return;
    }

    const previousMode = permissionMode;
    setPermissionMode(nextMode);

    if (!activeRunId) {
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
    const targetRunId = activeRunId;

    if (
      !targetRunId ||
      !directPrompt?.trim() ||
      !steerPrompt ||
      unsettledDirectPromptRef.current !== null
    ) {
      return;
    }

    const steerInput = createSteerInput({
      id: crypto.randomUUID(),
      targetRunId,
      text: steerPrompt,
      createdAtSequence: ++steerSequenceRef.current,
    });
    if (!steerInput) {
      return;
    }

    setError(null);
    setPrompt(defaultPrompt);
    pendingSteersRef.current = appendPendingSteer(
      pendingSteersRef.current,
      steerInput,
    );
    setPendingSteers(pendingSteersRef.current);

    try {
      await steerPromptToRun(targetRunId, steerPrompt);
      const next = acceptPendingSteer(pendingSteersRef.current, steerInput.id);
      pendingSteersRef.current = next;
      setPendingSteers(next);
      recordPromptHistory(steerPrompt);
    } catch (caughtError) {
      const result = rejectPendingSteer({
        pendingSteers: pendingSteersRef.current,
        rejectedSteers: rejectedSteersRef.current,
        steerInputId: steerInput.id,
        reason: String(caughtError),
      });
      pendingSteersRef.current = result.pendingSteers;
      rejectedSteersRef.current = result.rejectedSteers;
      setPendingSteers(result.pendingSteers);
      setRejectedSteers(result.rejectedSteers);
      if (!isSteerUnsupportedError(caughtError)) {
        setError(String(caughtError));
      }
    }
  }

  function sendPrompt() {
    if (unsettledDirectPromptRef.current !== null) {
      return;
    }
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
    const lifecycleSeq = promptLifecycleSeqRef.current;
    const promptSentBefore = promptSentCount(runId);
    setIsAwaitingPromptResponse(true);
    setDirectPrompt(nextPrompt);
    setItems((currentItems) => addUserMessage(currentItems, runId, nextPrompt));

    try {
      await sendPromptToRun(runId, nextPrompt);
      recordPromptHistory(nextPrompt);
    } catch (caughtError) {
      if (unsettledCall(caughtError) === "unknown") {
        if (promptSentCount(runId) > promptSentBefore) {
          // 결과는 몰랐지만 그 사이 이 run의 turn 시작을 봤다(Codex r12): 적용됐다 — 입력창에 되돌려 다시 보내게 하지 않는다.
          recordPromptHistory(nextPrompt);
          return;
        }
        // unknown이 먼저 돌아와도 늦은 turn을 놓치지 않는다(OCR r12). 일단 입력은 복원해 사용자가 결과를 알 수 있게 하되,
        // 같은 run의 다음 promptSent가 오면 listener가 적용 상태로 맞춘다. 직접 prompt는 자동 재전송하지 않는다.
        unsettledDirectPromptRef.current = { runId, text: nextPrompt, promptSentBefore };
      }
      setPrompt(nextPrompt);
      setItems((currentItems) => removeUserMessage(currentItems, runId, nextPrompt));
      setDirectPrompt(previousDirectPrompt);
      restoreAwaitingIfUnchanged(lifecycleSeq, false);
      setError(String(caughtError));
    }
  }

  async function steerQueuedPrompt(queuedPrompt: QueuedPrompt) {
    const targetRunId = activeRunId;
    if (!targetRunId || !directPrompt?.trim()) {
      return;
    }
    // 교환 prompt는 steer로 보내지 않는다: steer는 교환 소비(이어 가기 표지)를 싣지 못해, 서버에는 미소비 교환이 남는다
    // (Codex r7). 대기열 전송(`run.sendPrompt` + continuation)으로만 전달한다.
    if (queuedPrompt.exchangeRequestId) {
      setError("에이전트 메시지는 즉시 전송할 수 없습니다. 차례가 되면 전달됩니다.");
      return;
    }

    const result = prepareQueuedPromptSteer({
      queue: queuedPrompts,
      queuedPromptId: queuedPrompt.id,
      targetRunId,
      steerInputId: crypto.randomUUID(),
      createdAtSequence: ++steerSequenceRef.current,
    });
    if (!result.removedPrompt || !result.steerInput) {
      setError("전송하려던 prompt가 이미 queue에서 제거되었습니다.");
      return;
    }

    setError(null);
    setQueuedPrompts(result.queue);
    queuedPromptsRef.current = result.queue;
    pendingSteersRef.current = appendPendingSteer(
      pendingSteersRef.current,
      result.steerInput,
    );
    setPendingSteers(pendingSteersRef.current);
    if (editingPrompt?.id === result.removedPrompt.id) {
      closeQueuedPromptEditor();
    }

    try {
      await steerPromptToRun(targetRunId, result.steerInput.text);
      const next = acceptPendingSteer(pendingSteersRef.current, result.steerInput.id);
      pendingSteersRef.current = next;
      setPendingSteers(next);
      recordPromptHistory(result.steerInput.text);
    } catch (caughtError) {
      const rejected = rejectPendingSteer({
        pendingSteers: pendingSteersRef.current,
        rejectedSteers: rejectedSteersRef.current,
        steerInputId: result.steerInput.id,
        reason: String(caughtError),
      });
      pendingSteersRef.current = rejected.pendingSteers;
      rejectedSteersRef.current = rejected.rejectedSteers;
      setPendingSteers(rejected.pendingSteers);
      setRejectedSteers(rejected.rejectedSteers);
      if (!isSteerUnsupportedError(caughtError)) {
        setError(String(caughtError));
      }
      setPrompt(defaultPrompt);
    }
  }

  function queueRejectedSteer(steerInput: SteerInput) {
    const result = moveRejectedSteerToQueue({
      queue: queuedPromptsRef.current,
      rejectedSteers: rejectedSteersRef.current,
      steerInputId: steerInput.id,
    });
    queuedPromptsRef.current = result.queue;
    rejectedSteersRef.current = result.rejectedSteers;
    setQueuedPrompts(result.queue);
    setRejectedSteers(result.rejectedSteers);
    setError(null);
  }

  function removeRejectedSteerInput(steerInput: SteerInput) {
    const next = removeRejectedSteer(rejectedSteersRef.current, steerInput.id);
    rejectedSteersRef.current = next;
    setRejectedSteers(next);
  }

  async function retryRejectedSteerInput(steerInput: SteerInput) {
    const targetRunId = activeRunId;
    if (!targetRunId || !isRunning || unsettledDirectPromptRef.current !== null) {
      return;
    }

    const retry = retryRejectedSteer({
      pendingSteers: pendingSteersRef.current,
      rejectedSteers: rejectedSteersRef.current,
      steerInputId: steerInput.id,
      nextId: crypto.randomUUID(),
      createdAtSequence: ++steerSequenceRef.current,
    });
    if (!retry.steerInput) {
      return;
    }

    pendingSteersRef.current = retry.pendingSteers;
    rejectedSteersRef.current = retry.rejectedSteers;
    setPendingSteers(retry.pendingSteers);
    setRejectedSteers(retry.rejectedSteers);
    setError(null);

    try {
      await steerPromptToRun(targetRunId, retry.steerInput.text);
      const next = acceptPendingSteer(pendingSteersRef.current, retry.steerInput.id);
      pendingSteersRef.current = next;
      setPendingSteers(next);
    } catch (caughtError) {
      const rejected = rejectPendingSteer({
        pendingSteers: pendingSteersRef.current,
        rejectedSteers: rejectedSteersRef.current,
        steerInputId: retry.steerInput.id,
        reason: String(caughtError),
      });
      pendingSteersRef.current = rejected.pendingSteers;
      rejectedSteersRef.current = rejected.rejectedSteers;
      setPendingSteers(rejected.pendingSteers);
      setRejectedSteers(rejected.rejectedSteers);
      if (!isSteerUnsupportedError(caughtError)) {
        setError(String(caughtError));
      }
    }
  }

  async function cancelCurrentPromptAndSendRejectedSteer(steerInput: SteerInput) {
    const originalPrompt = directPrompt?.trim();
    const targetRunId = activeRunId;
    if (!targetRunId || !originalPrompt || unsettledDirectPromptRef.current !== null) {
      return;
    }

    const nextGoal = buildSteerPrompt(originalPrompt, steerInput.text);
    const nextRejected = removeRejectedSteer(rejectedSteersRef.current, steerInput.id);
    rejectedSteersRef.current = nextRejected;
    setRejectedSteers(nextRejected);
    setError(null);
    const lifecycleSeq = promptLifecycleSeqRef.current;
    setIsAwaitingPromptResponse(true);
    setDirectPrompt(nextGoal);
    setItems((currentItems) =>
      addUserMessage(currentItems, targetRunId, steerInput.text),
    );

    try {
      await cancelCurrentPromptAndSendToRun(targetRunId, nextGoal);
      recordPromptHistory(steerInput.text);
    } catch (caughtError) {
      rejectedSteersRef.current = [...rejectedSteersRef.current, steerInput];
      setRejectedSteers(rejectedSteersRef.current);
      if (promptLifecycleSeqRef.current === lifecycleSeq) {
        // 그 사이 run lifecycle이 바꾸지 않았을 때만 이 조작이 바꾼 것을 되돌린다(Codex r10).
        setDirectPrompt(originalPrompt);
      }
      setItems((currentItems) =>
        removeUserMessage(currentItems, targetRunId, steerInput.text),
      );
      setError(String(caughtError));
      restoreAwaitingIfUnchanged(lifecycleSeq, false);
    }
  }

  async function fullRestartWithRejectedSteer(steerInput: SteerInput) {
    const originalPrompt = directPrompt?.trim();
    const runIdToCancel = activeRunId;
    if (!runIdToCancel || !originalPrompt) {
      return;
    }
    // 이 조작의 재시작 의도(Codex r9): 앞 의도(결과를 몰랐던 앞 재시작의 보류 포함)를 대체한다. 재시작은 이 id로 한 번만 한다.
    restartAttemptSeqRef.current += 1;
    const attemptId = restartAttemptSeqRef.current;
    restartIntentRef.current = { attemptId, runId: runIdToCancel };
    unsettledRestartRef.current = null;

    const nextGoal = buildSteerPrompt(originalPrompt, steerInput.text);
    const nextRejected = removeRejectedSteer(rejectedSteersRef.current, steerInput.id);
    rejectedSteersRef.current = nextRejected;
    setRejectedSteers(nextRejected);
    setError(null);
    // 취소를 기다리는 동안은 취소 진행(`trackCancel`)이 자동 전송을 막는다. 응답 대기는 건드리지 않는다 — run lifecycle이
    // 정하는 값이라, 기다리는 동안 turn이 끝나면(`promptCompleted`) 그 최신 값이 남아야 한다(Codex r10).

    // 취소가 끝난 뒤: 버린 교환을 서버에서도 끝내고 새 run을 시작한다 — 이 시도의 의도가 아직 살아 있을 때 한 번만. 대기열은
    // 호출 전 스냅샷이 아니라 취소한 run의 지금 대기열이다(기다리는 동안 들어온 항목 포함, Codex r9). 교환 항목은 취소한 run이
    // 대상이라 새 run으로 옮기지 않고(서버가 다른 run으로의 전달을 거절한다) 버린 뒤 서버에서도 끝낸다(Codex r7·r8).
    async function restartAfterCancel() {
      if (!claimRestart(attemptId)) {
        return;
      }
      const queue = queueOfRun(runIdToCancel!);
      const queuedPromptsToKeep = queue.filter((item) => !item.exchangeRequestId);
      void discardExchangeDeliveries(exchangeRequestIdsOf(queue));
      if (unsettledDirectPromptRef.current?.runId === runIdToCancel) {
        // 확정된 취소 뒤에는 옛 run의 늦은 lifecycle이 replacement run의 composer를 조정해서는 안 된다.
        unsettledDirectPromptRef.current = null;
      }
      try {
        const started = await startRun(nextGoal, {
          queuedPrompts: queuedPromptsToKeep,
          displayPrompt: steerInput.text,
        });
        if (!started) {
          rejectedSteersRef.current = [...rejectedSteersRef.current, steerInput];
          setRejectedSteers(rejectedSteersRef.current);
          setQueuedPrompts(queuedPromptsToKeep);
        }
      } catch (caughtError) {
        rejectedSteersRef.current = [...rejectedSteersRef.current, steerInput];
        setRejectedSteers(rejectedSteersRef.current);
        setQueuedPrompts(queuedPromptsToKeep);
        setError(String(caughtError));
        setIsAwaitingPromptResponse(false);
      }
    }

    try {
      await trackCancel(() => cancelAgentRun(runIdToCancel));
    } catch (caughtError) {
      // 취소가 적용되지 않았거나(서버가 거절, 보내지 않음) 결과를 모른다(Codex r8): 원래 run이 살아 있을 수 있으므로 지금은
      // 새 run을 시작하지 않고 거절된 steer를 되돌린다. 대기열은 건드리지 않는다 — 재시작은 호출 전에 아무것도 빼지 않았고,
      // 기다리는 동안 들어온 항목(이미 확인된 새 교환 포함)을 호출 전 스냅샷으로 덮어쓰면 잃는다(Codex r9). 결과를 모르면 실제
      // run 상태를 복구된 run 이벤트로 맞춘다: 그 run의 취소 끝이 오면 재시작을 한 번 잇고, 새 turn을 받으면(살아 있다)
      // 버린다 — 그때는 turn 끝에 교환이 전달된다.
      const unsettled = unsettledCall(caughtError);
      if (restartIntentRef.current?.attemptId !== attemptId) {
        // 그 사이 다른 조작(새 재시작·취소)이 이 시도를 대체했다: 화면 상태는 그 조작이 맡는다.
        return;
      }
      const endedMeanwhile =
        lastRunEndRef.current?.runId === runIdToCancel ? lastRunEndRef.current.status : null;
      if (unsettled === "unknown" && endedMeanwhile === "cancelled") {
        // 결과는 몰랐지만 그 run의 취소 끝이 이미 왔다: 취소가 적용됐다 — 재시작을 잇는다.
        await restartAfterCancel();
        return;
      }
      rejectedSteersRef.current = [...rejectedSteersRef.current, steerInput];
      setRejectedSteers(rejectedSteersRef.current);
      setError(String(caughtError));
      if (unsettled !== "unknown" || endedMeanwhile) {
        // 재시작하지 않는다: 이 시도의 의도를 끝낸다.
        restartIntentRef.current = null;
      }
      if (endedMeanwhile) {
        // run은 이미 끝났다(끝 이벤트가 패널을 정리했다): 대기열을 되살리지 않는다.
        return;
      }
      if (unsettled === "unknown") {
        unsettledRestartRef.current = {
          attemptId,
          runId: runIdToCancel,
          resume: () => {
            const next = removeRejectedSteer(rejectedSteersRef.current, steerInput.id);
            rejectedSteersRef.current = next;
            setRejectedSteers(next);
            setError(null);
            void restartAfterCancel();
          },
        };
      }
      return;
    }

    await restartAfterCancel();
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

  const compactRunConfigurationControls = (supportsModelSelection ||
    supportsEffortSelection) && (
    <div className="flex min-w-0 flex-wrap items-center gap-2">
      {supportsModelSelection && (
        <label className="flex items-center gap-1 text-xs text-muted-foreground">
          <span>Model</span>
          <Select value={modelId} onValueChange={changeModelId} disabled={isRunConfigurationLocked}>
            <SelectTrigger
              size="sm"
              className="max-w-44"
              aria-label={`${panelId} model`}
            >
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
        </label>
      )}
      {supportsEffortSelection && (
        <label className="flex items-center gap-1 text-xs text-muted-foreground">
          <span>Effort</span>
          <Select value={effortId} onValueChange={changeEffortId} disabled={isRunConfigurationLocked}>
            <SelectTrigger
              size="sm"
              className="max-w-36"
              aria-label={`${panelId} effort`}
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                {effortOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectGroup>
            </SelectContent>
          </Select>
        </label>
      )}
    </div>
  );

  return (
    <div className="h-full min-h-0">
      {!showPromptComposer && runConfigurationPortal && compactRunConfigurationControls
        ? createPortal(compactRunConfigurationControls, runConfigurationPortal)
        : null}
      <ResizablePanelGroup
        orientation="vertical"
        className="flex h-full min-h-0 w-full flex-col"
      >
        <ResizablePanel id={`${panelId}-timeline`} minSize="220px">
          <div className="flex h-full min-h-0 w-full">
          <div ref={timelineScrollRef} className="h-full min-w-0 flex-1 overflow-auto">
            <div className="flex flex-col">
          {scrollHeader}

          <div className="m-4 flex flex-col gap-4">
            {!isRunning && (
              <div>
                <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
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
                  <div className="flex shrink-0 items-center">
                    <Button
                      type="button"
                      size="sm"
                      disabled={!canStartRun}
                      onClick={() => void run()}
                    >
                      <PlayIcon data-icon="inline-start" />
                      시작
                    </Button>
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

              {enableGoalContinuation && (
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
              )}

              <div className="flex flex-col">
                <div className="flex flex-wrap items-center justify-between gap-2 border-b pb-3">
                  <div className="flex flex-wrap gap-1.5" role="tablist" aria-label="ACP event filter">
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
                  <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                    {(activeRunId || isRunning || agentThreadStatus.type !== "unknown") && (
                      <>
                      {sessionFreshnessLabel && (
                        <span aria-label="Session updated at">
                          {sessionFreshnessLabel}
                        </span>
                      )}
                      {availableCommandMetadata && (
                        <AvailableCommandsPopover
                          metadata={availableCommandMetadata}
                          summary={availableCommandsSummary}
                        />
                      )}
                      <AgentThreadStatusBadge status={agentThreadStatus} />
                      </>
                    )}
                    <div className="flex items-center gap-1.5">
                      <span className="hidden font-medium sm:inline">Permission</span>
                      <Select
                        value={permissionMode}
                        onValueChange={(value) =>
                          void changePermissionMode(value as PermissionMode)
                        }
                        disabled={isChangingPermissionMode}
                      >
                        <SelectTrigger
                          size="sm"
                          className="max-w-40"
                          aria-label={`${panelId} permission mode`}
                          data-testid="agent-run-permission-mode"
                        >
                          {isChangingPermissionMode ? (
                            <Loader2Icon className="animate-spin" />
                          ) : (
                            <SelectValue />
                          )}
                        </SelectTrigger>
                        <SelectContent align="end">
                          <SelectGroup>
                            {permissionModeOptions.map((option) => (
                              <SelectItem key={option.value} value={option.value}>
                                {option.label}
                              </SelectItem>
                            ))}
                          </SelectGroup>
                        </SelectContent>
                      </Select>
                    </div>
                    <TooltipProvider>
                      <Tooltip>
                        <TooltipTrigger asChild>
                          <Button
                            type="button"
                            size="icon-sm"
                            variant="outline"
                            aria-label={isMinimapVisible ? "대화 미니맵 숨기기" : "대화 미니맵 표시"}
                            aria-pressed={isMinimapVisible}
                            onClick={handleMinimapVisibilityToggle}
                          >
                            {isMinimapVisible ? <PanelRightCloseIcon /> : <PanelRightOpenIcon />}
                          </Button>
                        </TooltipTrigger>
                        <TooltipContent side="left">
                          {isMinimapVisible ? "대화 미니맵 숨기기" : "대화 미니맵 표시"}
                        </TooltipContent>
                      </Tooltip>
                    </TooltipProvider>
                  </div>
                </div>
                <VirtualizedRunTimeline
                  items={visibleItems}
                  scrollParentRef={timelineScrollRef}
                  onLayoutChange={handleTimelineLayoutChange}
                />
                {inputMode === "prompt" && pendingSteers.length > 0 && (
                  <PendingSteerTimeline pendingSteers={pendingSteers} />
                )}
                {inputMode === "prompt" && rejectedSteers.length > 0 && (
                  <RejectedSteerTimeline
                    rejectedSteers={rejectedSteers}
                    isRunning={isRunning}
                    onQueueSteer={queueRejectedSteer}
                    onRetrySteer={(steerInput) => void retryRejectedSteerInput(steerInput)}
                    onCancelAndSendSteer={(steerInput) =>
                      void cancelCurrentPromptAndSendRejectedSteer(steerInput)
                    }
                    onFullRestartSteer={(steerInput) =>
                      void fullRestartWithRejectedSteer(steerInput)
                    }
                    onRemoveSteer={removeRejectedSteerInput}
                  />
                )}
                {inputMode === "prompt" && queuedPrompts.length > 0 && (
                  <QueuedPromptTimeline
                    queuedPrompts={queuedPrompts}
                    activeRunId={activeRunId}
                    directPrompt={directPrompt}
                    onSteerPrompt={(queuedPrompt) => void steerQueuedPrompt(queuedPrompt)}
                    onEditPrompt={openQueuedPromptEditor}
                    onMovePrompt={moveQueuedPrompt}
                    onRemovePrompt={(queuedPromptId) => void removeQueuedPrompt(queuedPromptId)}
                  />
                )}
              </div>

          </div>
            </div>
          </div>
          </div>
          {isMinimapVisible && (
            <AgentRunMinimap
              entries={minimapEntries}
              layoutSnapshot={timelineLayout}
              onSeek={handleMinimapSeek}
            />
          )}
          </div>
        </ResizablePanel>

        {showPromptComposer && (
          <>
        <ResizableHandle
          aria-label="프롬프트 영역 크기 조정"
          className="relative flex h-2 shrink-0 cursor-ns-resize items-center justify-center bg-transparent transition-colors after:absolute after:left-0 after:right-0 after:h-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <div className="relative z-10 h-1 w-12 rounded-full bg-border transition-colors" />
        </ResizableHandle>

        <ResizablePanel
          id={`${panelId}-prompt`}
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
          <div className="flex shrink-0 flex-wrap items-center justify-between gap-2 border-b px-4 py-1">
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
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={() => setIsRunSettingsDialogOpen(true)}
              >
                <SettingsIcon data-icon="inline-start" />
                Settings
              </Button>
              {compactRunConfigurationControls}
            </div>
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
                        {supportsEffortSelection && (
                          <div className="grid gap-1">
                            <span className="text-xs font-medium text-muted-foreground">Effort</span>
                            <span>
                              {selectedEffortOption?.label ?? providerDefaultEffortOption.label}
                            </span>
                            <span className="text-xs text-muted-foreground">
                              {selectedEffortOption?.description}
                            </span>
                          </div>
                        )}
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
          </>
        )}
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
              권한 모드, 모델, effort, 컨텍스트 크기를 설정합니다.
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
                  onValueChange={changeModelId}
                  disabled={isRunConfigurationLocked}
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
                  {isRunConfigurationLocked
                    ? "실행 중에는 모델을 변경할 수 없습니다."
                    : selectedModelOption?.description}
                </span>
              </label>
              {supportsEffortSelection && (
                <label className="flex flex-col gap-2 text-sm font-medium">
                  Effort
                  <Select
                    value={effortId}
                    onValueChange={changeEffortId}
                    disabled={isRunConfigurationLocked}
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        {effortOptions.map((option) => (
                          <SelectItem key={option.value} value={option.value}>
                            {option.label}
                          </SelectItem>
                        ))}
                      </SelectGroup>
                    </SelectContent>
                  </Select>
                  <span className="text-xs font-normal text-muted-foreground">
                    {isRunConfigurationLocked
                      ? "실행 중에는 effort를 변경할 수 없습니다."
                      : selectedEffortOption?.description}
                  </span>
                </label>
              )}
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
});

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
  onLayoutChange,
}: {
  items: TimelineItem[];
  scrollParentRef: RefObject<HTMLDivElement | null>;
  onLayoutChange: (snapshot: TimelineLayoutSnapshot) => void;
}) {
  const timelineRef = useRef<HTMLDivElement | null>(null);
  const stickToBottomRef = useRef(true);
  const revisionRef = useRef(0);
  const [viewportHeight, setViewportHeight] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [timelineOffsetInScroller, setTimelineOffsetInScroller] = useState(0);
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
      const timelineViewportCapacity = Math.min(
        scrollElement.clientHeight,
        timelineRect.height,
      );
      const nextTimelineOffset =
        timelineRect.top - scrollRect.top + scrollElement.scrollTop;
      const distanceFromBottom =
        scrollElement.scrollHeight - scrollElement.scrollTop - scrollElement.clientHeight;

      stickToBottomRef.current = distanceFromBottom < 48;
      setScrollTop(visibleTop);
      setViewportHeight(Math.max(visibleHeight, timelineViewportCapacity));
      setTimelineOffsetInScroller(Math.max(0, nextTimelineOffset));
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

  useEffect(() => {
    revisionRef.current += 1;
    onLayoutChange({
      timelineOffsetInScroller,
      totalHeight,
      visibleStart: Math.min(totalHeight, Math.max(0, scrollTop)),
      visibleEnd: Math.min(totalHeight, Math.max(0, scrollTop + viewportHeight)),
      viewportHeight,
      itemLayouts: itemLayouts.map(({ item, start, end, height }) => ({
        id: item.id,
        start,
        end,
        height,
        measured: Object.prototype.hasOwnProperty.call(measuredHeights, item.id),
      })),
      revision: revisionRef.current,
    });
  }, [
    itemLayouts,
    measuredHeights,
    onLayoutChange,
    scrollTop,
    timelineOffsetInScroller,
    totalHeight,
    viewportHeight,
  ]);

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

function PendingSteerTimeline({ pendingSteers }: { pendingSteers: SteerInput[] }) {
  return (
    <div className="mt-3 flex flex-col gap-2" aria-label="Pending steer inputs">
      {pendingSteers.map((steerInput, index) => (
        <Message key={steerInput.id} className="justify-end">
          <div className="min-w-0 max-w-[80%] rounded-lg border border-primary/30 bg-primary/10 p-2 text-foreground">
            <div className="mb-1.5 flex items-center gap-2 text-xs text-muted-foreground">
              <Loader2Icon className="size-3 animate-spin text-primary" />
              <span>Steer pending #{index + 1}</span>
            </div>
            <div className="whitespace-pre-wrap break-words text-sm">{steerInput.text}</div>
          </div>
        </Message>
      ))}
    </div>
  );
}

function RejectedSteerTimeline({
  rejectedSteers,
  isRunning,
  onQueueSteer,
  onRetrySteer,
  onCancelAndSendSteer,
  onFullRestartSteer,
  onRemoveSteer,
}: {
  rejectedSteers: SteerInput[];
  isRunning: boolean;
  onQueueSteer: (steerInput: SteerInput) => void;
  onRetrySteer: (steerInput: SteerInput) => void;
  onCancelAndSendSteer: (steerInput: SteerInput) => void;
  onFullRestartSteer: (steerInput: SteerInput) => void;
  onRemoveSteer: (steerInput: SteerInput) => void;
}) {
  return (
    <div className="mt-3 flex flex-col gap-2" aria-label="Rejected steer inputs">
      {rejectedSteers.map((steerInput, index) => (
        <Message key={steerInput.id} className="justify-end">
          <div className="min-w-0 max-w-[80%] rounded-lg border border-destructive/30 bg-destructive/10 p-2 text-foreground">
            <div className="mb-1.5 flex flex-wrap items-center justify-between gap-2">
              <div className="flex min-w-0 items-center gap-2 text-xs text-destructive">
                <XCircleIcon className="size-3 shrink-0" />
                <span>Steer rejected #{index + 1}</span>
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  onClick={() => onQueueSteer(steerInput)}
                >
                  Queue
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  disabled={!isRunning}
                  onClick={() => onRetrySteer(steerInput)}
                >
                  Retry
                </Button>
                <Button
                  type="button"
                  variant="default"
                  size="xs"
                  disabled={!isRunning}
                  onClick={() => onCancelAndSendSteer(steerInput)}
                >
                  Cancel & send
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="xs"
                  disabled={!isRunning}
                  onClick={() => onFullRestartSteer(steerInput)}
                >
                  Full restart
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  aria-label={`${index + 1}번 rejected steer 제거`}
                  onClick={() => onRemoveSteer(steerInput)}
                >
                  <XIcon className="size-3" />
                </Button>
              </div>
            </div>
            <div className="whitespace-pre-wrap break-words text-sm">{steerInput.text}</div>
            {steerInput.errorMessage && (
              <div className="mt-1.5 text-xs text-muted-foreground">
                {steerInput.errorMessage}
              </div>
            )}
          </div>
        </Message>
      ))}
    </div>
  );
}

/** 대기열 항목이 그 run의 대기열에 있을 수 있는가(Codex r11): 교환 항목은 묶인 run에만 속한다(묶인 run이 없으면 제한 없음). */
function queuedPromptBelongsToRun(item: QueuedPrompt, runId: string | null) {
  return !item.exchangeRequestId || !item.exchangeRunId || item.exchangeRunId === runId;
}

function exchangeRequestIdsOf(queue: QueuedPrompt[]) {
  return queue.flatMap((item) => (item.exchangeRequestId ? [item.exchangeRequestId] : []));
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
                  disabled={!activeRunId || !directPrompt?.trim() || Boolean(queuedPrompt.exchangeRequestId)}
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

function AvailableCommandsPopover({
  metadata,
  summary,
}: {
  metadata: AvailableCommandMetadata;
  summary: string;
}) {
  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          type="button"
          size="sm"
          variant="ghost"
          className="h-7 px-2 text-xs text-muted-foreground"
          aria-label="Available commands"
        >
          {summary}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-96 max-w-[calc(100vw-2rem)] min-w-0 p-0">
        <div className="min-w-0 border-b px-3 py-2">
          <div className="text-sm font-medium">Available commands</div>
          <div className="text-xs text-muted-foreground">{summary}</div>
        </div>
        <div className="max-h-80 min-w-0 overflow-y-auto p-2">
          {metadata.commands.length === 0 ? (
            <div className="px-2 py-6 text-center text-xs text-muted-foreground">
              No commands available
            </div>
          ) : (
            <div className="grid min-w-0 gap-1">
              {metadata.commands.map((command) => (
                <div
                  key={command.id}
                  className="min-w-0 rounded-md border px-2 py-1.5 text-xs"
                >
                  <div className="min-w-0 break-all font-mono font-medium text-foreground">
                    {command.name}
                  </div>
                  {command.description && (
                    <div className="mt-1 line-clamp-2 min-w-0 break-all text-muted-foreground">
                      {command.description}
                    </div>
                  )}
                  {command.inputHint && (
                    <div className="mt-1 min-w-0 break-all text-muted-foreground">
                      Input: {command.inputHint}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

function AgentThreadStatusBadge({ status }: { status: AgentThreadStatus }) {
  const label = agentThreadStatusLabel(status);
  if (status.type === "active") {
    return (
      <Badge
        variant="secondary"
        className="shrink-0 gap-1 border-primary/30 bg-primary/10 text-primary"
        aria-label={label}
      >
        <Loader2Icon className="size-3 animate-spin" aria-hidden="true" />
        {label}
      </Badge>
    );
  }
  if (status.type === "idle") {
    return (
      <Badge
        variant="secondary"
        className="shrink-0 gap-1 border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
        aria-label={label}
      >
        <CheckCircleIcon className="size-3" aria-hidden="true" />
        {label}
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="shrink-0 gap-1 text-muted-foreground" aria-label={label}>
      <InfoIcon className="size-3" aria-hidden="true" />
      {label}
    </Badge>
  );
}

function agentThreadStatusLabel(status: AgentThreadStatus) {
  if (status.type === "active") {
    return "Agent active";
  }
  if (status.type === "idle") {
    return "Agent idle";
  }
  return "Agent status unknown";
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
            <DiffViewer content={content} className="max-h-72 w-full rounded-none border-0 text-caption" />
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
  if (status === "sessionCreated") return "Session started";
  if (status === "sessionIdle") return "Agent idle";
  if (status === "promptSent") return "Prompt sent";
  if (status === "promptCompleted") return "Prompt completed";
  if (status === "steerPending") return "Steer pending";
  if (status === "steerAccepted") return "Steer accepted";
  if (status === "steerRejected") return "Steer rejected";
  if (status === "completed") return "Completed";
  if (status === "cancelled") return "Cancelled";
  return status || "Lifecycle";
}

function lifecycleStatusClassName(status: string) {
  if (status === "sessionCreated" || status === "sessionIdle") {
    return "bg-muted text-muted-foreground";
  }
  if (status === "completed" || status === "promptCompleted") {
    return "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300";
  }
  if (status === "steerAccepted") {
    return "bg-emerald-100 text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300";
  }
  if (status === "cancelled") {
    return "bg-amber-100 text-amber-700 dark:bg-amber-950 dark:text-amber-300";
  }
  if (status === "steerRejected") {
    return "bg-destructive/10 text-destructive";
  }
  return "bg-primary/10 text-primary";
}

function isLifecycleTerminal(status: string) {
  return status === "completed" || status === "promptCompleted" || status === "cancelled";
}

function isSteerUnsupportedError(error: unknown) {
  const message = String(error).toLowerCase();
  return (
    message.includes("steer unsupported") ||
    message.includes("steer is not supported") ||
    message.includes("does not support active-turn steer")
  );
}

function toolStatusLabel(status: string) {
  if (status === "completed") return "Completed";
  if (status === "failed") return "Failed";
  if (status === "in_progress") return "In progress";
  if (status === "pending") return "In progress";
  if (status === "running") return "Running";
  return status || "Tool";
}
