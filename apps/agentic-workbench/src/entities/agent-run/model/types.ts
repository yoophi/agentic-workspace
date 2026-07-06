export type AgentDescriptor = {
  id: string;
  label: string;
  command: string;
  models?: AgentOptionDescriptor[];
  contextSizes?: AgentOptionDescriptor[];
};

export type AgentOptionDescriptor = {
  id: string;
  label: string;
};

export type ResumePolicy = "fresh" | "resumeIfAvailable" | "resumeRequired";
export type AgentRunSessionMode = "new" | "reuse";

/** Ralph loop: 목표가 끝날 때까지 동일 prompt를 자동 반복 실행하는 설정. */
export type RalphLoopRequest = {
  enabled: boolean;
  maxIterations: number;
  promptTemplate: string;
  stopOnError: boolean;
  stopOnPermission: boolean;
  delayMs: number;
};

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

export type AgentRunRequest = {
  runId?: string;
  goal: string;
  agentId: string;
  cwd: string;
  agentCommand?: string;
  /** 해석·병합 완료된 환경변수(globalEnv ⊕ profile.env). runner는 주입만 한다. */
  agentEnv?: Record<string, string>;
  stdioBufferLimitMb?: number;
  autoAllow?: boolean;
  resumeSessionId?: string;
  resumePolicy?: ResumePolicy;
  permissionMode?: PermissionMode;
  modelId?: string;
  contextSize?: ContextSizePreset;
  ralphLoop?: RalphLoopRequest;
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

export type PermissionMode =
  | "default"
  | "auto"
  | "readOnly"
  | "plan"
  | "acceptEdits"
  | "dangerouslySkipAllPermissions";

export type ContextSizePreset = "default" | "medium" | "large" | "xLarge";

export type AgentRun = {
  id: string;
  goal: string;
  agentId: string;
};

export type ToolFileChangeKind =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "unknown";

export type ToolFileChangeStatus =
  | "inProgress"
  | "completed"
  | "failed"
  | "unavailable";

export type ToolFileChange = {
  path: string;
  oldPath: string | null;
  kind: ToolFileChangeKind;
  status: ToolFileChangeStatus;
  diff: string | null;
  content: string | null;
  binary: boolean;
  truncated: boolean;
  message: string | null;
};

export type RunEventEnvelope = {
  runId: string;
  event: RunEvent;
};

export type RunEvent =
  | { type: "lifecycle"; status: LifecycleStatus; message: string }
  | { type: "userMessage"; text: string }
  | { type: "agentMessage"; text: string }
  | { type: "thought"; text: string }
  | { type: "plan"; entries: PlanEntry[] }
  | {
      type: "tool";
      toolCallId?: string;
      status: string;
      title: string;
      locations: string[];
      fileChanges?: ToolFileChange[];
    }
  | { type: "usage"; used: number; size: number }
  | {
      type: "permission";
      permissionId?: string;
      title: string;
      input?: unknown;
      options: PermissionOption[];
      selected?: string;
      requiresResponse: boolean;
    }
  | { type: "fileSystem"; operation: string; path: string }
  | { type: "terminal"; operation: string; terminalId?: string; message: string }
  | { type: "diagnostic"; message: string }
  | {
      type: "ralphLoop";
      iteration: number;
      maxIterations: number;
      status: RalphLoopStatus;
    }
  | { type: "raw"; method: string; payload: unknown }
  | { type: "error"; message: string };

export type LifecycleStatus =
  | "started"
  | "initialized"
  | "sessionCreated"
  | "promptSent"
  | "promptCompleted"
  | "completed"
  | "cancelled";

export type RalphLoopStatus = "started" | "completed" | "failed" | "stopped";

export type PlanEntry = {
  status: string;
  content: string;
};

export type PermissionOption = {
  name: string;
  kind: string;
  optionId: string;
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
