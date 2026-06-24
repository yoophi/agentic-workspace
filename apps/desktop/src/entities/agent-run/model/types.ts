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
};

export type AgentRunSettingsRalphLoop = {
  enabled: boolean;
  maxIterations: number;
  delayMs: number;
  stopOnError: boolean;
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
  | { type: "tool"; toolCallId?: string; status: string; title: string; locations: string[] }
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
