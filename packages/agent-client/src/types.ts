// ACP agent-run 통신 계약. Rust crate `acp-agent-core`의 domain(camelCase 직렬화)과 1:1.
// hushline 등 여러 앱이 소비하는 공유 계약이므로 앱 전용 타입(Goal/Worktree/Settings 등)은
// 여기에 두지 않는다.

export type AgentOptionDescriptor = {
  id: string;
  label: string;
};

export type AgentDescriptor = {
  id: string;
  label: string;
  command: string;
  runtimeVersion?: string;
  models?: AgentOptionDescriptor[];
  contextSizes?: AgentOptionDescriptor[];
};

export type ResumePolicy = "fresh" | "resumeIfAvailable" | "resumeRequired";

export type PermissionMode =
  | "default"
  | "auto"
  | "readOnly"
  | "plan"
  | "acceptEdits"
  | "dangerouslySkipAllPermissions";

export type ContextSizePreset = "default" | "medium" | "large" | "xLarge";

/** Ralph loop: 목표가 끝날 때까지 동일 prompt를 자동 반복 실행하는 설정. */
export type RalphLoopRequest = {
  enabled: boolean;
  maxIterations: number;
  promptTemplate: string;
  stopOnError: boolean;
  stopOnPermission: boolean;
  delayMs: number;
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

export type AgentRun = {
  id: string;
  goal: string;
  agentId: string;
};

export type AgentThreadStatus = {
  type: "active" | "idle" | "unknown";
  activeFlags?: string[];
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

export type PlanEntry = {
  status: string;
  content: string;
};

export type PermissionOption = {
  name: string;
  kind: string;
  optionId: string;
};

export type LifecycleStatus =
  | "started"
  | "initialized"
  | "sessionCreated"
  | "sessionIdle"
  | "promptSent"
  | "promptCompleted"
  | "steerPending"
  | "steerAccepted"
  | "steerRejected"
  | "completed"
  | "cancelled";

export type RalphLoopStatus = "started" | "completed" | "failed" | "stopped";

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
      type: "sessionInfo";
      threadStatus?: AgentThreadStatus | null;
      title?: string | null;
      updatedAt?: string | null;
    }
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

export type RunEventEnvelope = {
  runId: string;
  event: RunEvent;
};
