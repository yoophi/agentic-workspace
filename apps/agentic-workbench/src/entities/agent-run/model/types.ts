// ACP agent-run 통신 계약 타입은 공유 패키지 `@yoophi/agent-client`에서 가져온다.
// 여기서는 그 계약을 re-export해 기존 소비 경로(`@/entities/agent-run/...`)를 유지하고,
// workbench 전용 타입(Settings/Goal/Worktree/Timeline/Command candidate 등)만 로컬로 정의한다.
import type {
  AgentThreadStatus,
  ContextSizePreset,
  PermissionMode,
  RunEvent,
} from "@yoophi/agent-client";

export type {
  AgentDescriptor,
  AgentOptionDescriptor,
  AgentRun,
  AgentRunRequest,
  AgentThreadStatus,
  ContextSizePreset,
  LifecycleStatus,
  PermissionMode,
  PermissionOption,
  PlanEntry,
  RalphLoopRequest,
  RalphLoopStatus,
  ResumePolicy,
  RunEvent,
  RunEventEnvelope,
  ToolFileChange,
  ToolFileChangeKind,
  ToolFileChangeStatus,
} from "@yoophi/agent-client";

export type AgentRunSessionMode = "new" | "reuse";

export type AgentRunSettings = {
  workingDirectory: string;
  agentId: string;
  permissionMode: PermissionMode;
  modelId: string;
  contextSize: ContextSizePreset;
  sessionMode: AgentRunSessionMode;
  ralphLoop: AgentRunSettingsRalphLoop;
  commandOverrides?: AgentCommandOverrides;
};

// 실제 agent catalog id와 동일한 집합(이슈 #121의 claude_code/pi는 예시 표기).
export type AgentType = "codex" | "claude-code" | "opencode" | "pi-coding-agent";

/**
 * agent 실행 프로필(specs/008). 같은 type을 서로 다른 command/env 조합으로
 * 복수 등록할 수 있다. 기본 프로필의 id는 catalog agent id와 동일하다.
 */
export type AgentProfile = {
  id: string;
  name: string;
  agentType: AgentType;
  command?: string | null;
  env?: Record<string, string>;
  enabled: boolean;
  builtIn: boolean;
};

export type AgentCommandOverrides = {
  globalCommand?: string | null;
  /** legacy(command-only) 데이터. 신규 UI에서는 편집하지 않고 seed 매핑에만 쓴다. */
  agentCommands?: Record<string, string>;
  globalEnv?: Record<string, string>;
  profiles?: AgentProfile[];
};

export type AgentCommandSource =
  | "agentOverride"
  | "globalOverride"
  | "defaultCommand"
  | "profileCommand";

export type CommandResolutionResult = {
  agentId: string;
  command: string;
  source: AgentCommandSource;
};

export type AgentRunSettingsRalphLoop = {
  enabled: boolean;
  maxIterations: number;
  delayMs: number;
  stopOnError: boolean;
  stopOnPermission: boolean;
  promptTemplate: string;
};

export type GoalStatus =
  | "active"
  | "paused"
  | "blocked"
  | "usageLimited"
  | "budgetLimited"
  | "complete";

export type ThreadGoal = {
  workingDirectory: string;
  objective: string;
  status: GoalStatus;
  tokenBudget?: number | null;
  tokensUsed: number;
  timeUsedSeconds: number;
  createdAt: string;
  updatedAt: string;
};

export type GoalInput = {
  workingDirectory: string;
  objective: string;
  tokenBudget?: number | null;
};

export type WorktreeChangeType =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "untracked";

/** worktree에서 HEAD 대비 변경된 파일 한 건(백엔드 camelCase와 일치). */
export type WorktreeChange = {
  path: string;
  oldPath: string | null;
  changeType: WorktreeChangeType;
  binary: boolean;
  diff: string | null;
  content: string | null;
  truncated: boolean;
};

export type GoalUpdateInput = {
  objective?: string;
  status?: GoalStatus;
  tokenBudget?: number | null;
};

export type GoalProgressInput = {
  tokensUsed: number;
  timeUsedSeconds: number;
};

/** provider가 로컬에 남긴 네이티브 세션 한 건의 요약(백엔드 camelCase와 일치). */
export type ProviderSession = {
  agentId: string;
  id: string;
  cwd: string | null;
  title: string | null;
  file: string;
  messageCount: number;
  createdAt: string | null;
  updatedAt: string | null;
  model: string | null;
  branch: string | null;
  source: string | null;
};

export type AgentToolCommandCandidateStatus = "loading" | "ready" | "empty" | "error";

export type AgentToolCommandCandidateSource = "sessionTool" | "appCommand" | "extension";

export type AgentToolCommandCandidateScope = {
  runId?: string | null;
  agentId?: string | null;
  workingDirectory?: string | null;
};

export type AgentToolCommandCandidate = {
  id: string;
  name: string;
  description: string | null;
  insertText: string;
  source: AgentToolCommandCandidateSource;
  scope: AgentToolCommandCandidateScope;
};

export type CommandDetailItem = {
  id: string;
  name: string;
  description: string | null;
  inputHint: string | null;
  source: AgentToolCommandCandidateSource;
};

export type AvailableCommandMetadata = {
  sessionUpdate: "available_commands_update";
  commands: CommandDetailItem[];
  updatedAt: number | null;
};

export type AgentToolCommandCandidateQuery = {
  runId?: string | null;
  agentId: string;
  workingDirectory: string;
  sessionMode: AgentRunSessionMode;
};

export type AgentToolCommandCandidateResponse = {
  status: AgentToolCommandCandidateStatus;
  candidates: AgentToolCommandCandidate[];
};

export type SessionInfoUpdateMetadata = {
  sessionUpdate: "session_info_update";
  threadStatus: AgentThreadStatus | null;
  title: string | null;
  updatedAt: string | null;
};

export type SessionLifecycleStatusMessage = {
  status: "sessionCreated" | "sessionIdle";
  label: string;
  description: string;
  tone: "info";
  dedupeKey: string;
};

export type EventGroup =
  | "user/message"
  | "assistant/message"
  | "thought"
  | "tool_call/tool_result"
  | "permission"
  | "terminal"
  | "lifecycle"
  | "error"
  | "raw";

export type TimelineItem = {
  id: string;
  runId: string;
  group: EventGroup;
  title: string;
  body: string;
  tone?: "info" | "success" | "warning" | "danger";
  createdAt: number;
  event: RunEvent;
};
