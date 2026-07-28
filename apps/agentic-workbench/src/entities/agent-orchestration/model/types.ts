export const MAIN_AGENT_NODE_ID = "main-agent-run";
export const MAX_ORCHESTRATION_NODES = 8;
export const MAX_ORCHESTRATION_PROMPT_BYTES = 16 * 1024;

export type AgentNodeKind = "main" | "child";
export type AgentNodeCreator = "user" | "coordinator";
export type TaskStatus =
  | "pending"
  | "ready"
  | "running"
  | "inputRequired"
  | "blocked"
  | "completed"
  | "failed"
  | "cancelled";
export type ExecutionStatus =
  | "unassigned"
  | "starting"
  | "active"
  | "idle"
  | "stopped";
export type PresentationStatus =
  | "background"
  | "attentionRequired"
  | "promoting"
  | "panel"
  | "detached"
  | "archived";
export type PromotionPolicy =
  | "manual"
  | "onAttention"
  | "always"
  | "onFailure"
  | "onCompletion";

export type AgentRoleProfile = {
  id: string;
  name: string;
  responsibility: string;
  expectedOutput: string;
  systemInstructions?: string | null;
};

export type WorkerRuntimeProfile = {
  agentProfileId: string;
  providerId: string;
  modelId: string | null;
  accessPolicy: "readOnly";
  supportsReadOnly: boolean;
};

export type AgentNode = {
  id: string;
  kind: AgentNodeKind;
  parentNodeId: string | null;
  role: AgentRoleProfile;
  currentRunId: string | null;
  assignedTaskId: string | null;
  executionStatus: ExecutionStatus;
  presentationStatus: PresentationStatus;
  promotionPolicy: PromotionPolicy;
  runtimeProfile: WorkerRuntimeProfile | null;
  lastActivityAt: string | null;
  createdBy: AgentNodeCreator;
  createdAt: string;
};

export type CoordinatorGeneration = {
  id: string;
  ordinal: number;
  mainNodeId: string;
  runId: string;
  previousGenerationId: string | null;
  status: "active" | "ended" | "superseded";
  startedAt: string;
  endedAt: string | null;
  handoffSummary: string | null;
  successorGenerationId: string | null;
};

export type TaskFailure = {
  code: string;
  message: string;
  retryable: boolean;
  partialResultReportIds: string[];
};

export type OrchestrationTask = {
  id: string;
  parentTaskId: string | null;
  coordinatorGenerationId: string;
  assignedNodeId: string | null;
  title: string;
  objective: string;
  constraints: string[];
  expectedResult: string;
  dependencyTaskIds: string[];
  status: TaskStatus;
  awaitingHandoff: boolean;
  accessPolicy: "readOnly";
  attempt: number;
  latestResultReportId: string | null;
  failure: TaskFailure | null;
  revision: number;
  createdAt: string;
  startedAt: string | null;
  completedAt: string | null;
  updatedAt: string;
};

export type TaskReportType =
  | "progress"
  | "result"
  | "inputRequest"
  | "blocked"
  | "message";

export type TaskReport = {
  id: string;
  requestId: string;
  taskId: string;
  reporterNodeId: string;
  reporterRunId: string;
  type: TaskReportType;
  progressPercent: number | null;
  summary: string;
  findings: Array<{
    title: string;
    detail: string;
    evidence: string[];
    severity: "info" | "warning" | "critical";
  }>;
  artifactRefs: Array<{
    kind: "file" | "url" | "text";
    uri: string;
    label: string;
    description?: string | null;
  }>;
  unresolved: string[];
  confidence: number | null;
  createdAt: string;
};

export type TaskCommandKind =
  | "message"
  | "inputResponse"
  | "interrupt"
  | "cancel";
export type TaskCommandStatus =
  | "pending"
  | "dispatching"
  | "accepted"
  | "failed"
  | "cancelled";
export type CommandFailure = {
  code: string;
  message: string;
  retryable: boolean;
};
export type TaskCommand = {
  id: string;
  requestId: string;
  payloadFingerprint: string;
  taskId: string;
  nodeId: string;
  runId: string;
  attempt: number;
  kind: TaskCommandKind;
  message: string | null;
  inputReportId: string | null;
  delivery: "send" | "queue" | "draft";
  source: "user" | "coordinator" | "recovery";
  status: TaskCommandStatus;
  failure: CommandFailure | null;
  createdAt: string;
  updatedAt: string;
};

export type CoordinatorNotificationStatus =
  | "pending"
  | "dispatching"
  | "accepted"
  | "delivered"
  | "processed"
  | "failed"
  | "superseded";
export type CoordinatorNotification = {
  id: string;
  reportId: string;
  taskId: string;
  reportType: TaskReportType;
  generationId: string;
  mainRunId: string | null;
  status: CoordinatorNotificationStatus;
  attemptCount: number;
  failure: CommandFailure | null;
  collectedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type PromptTargetMode =
  | "focused"
  | "selected"
  | "all"
  | "coordinator";
export type PromptDispatchTargetStatus =
  | "pending"
  | "accepted"
  | "delivered"
  | "rejected"
  | "failed"
  | "cancelled";

export type PromptDispatchTarget = {
  panelId: string;
  runId: string | null;
  requestId: string;
  status: PromptDispatchTargetStatus;
  failureCode: string | null;
  failureReason: string | null;
};

export type PromptDispatch = {
  id: string;
  intent: "direct" | "delegate";
  targetMode: PromptTargetMode;
  message: string;
  delivery: "send" | "queue" | "draft";
  targets: PromptDispatchTarget[];
  createdBy: string;
  createdAt: string;
  updatedAt: string;
};

export type OrchestrationSession = {
  schemaVersion: number;
  id: string;
  worktreePath: string;
  boundWindowLabel: string | null;
  mainNodeId: string;
  activeCoordinatorGenerationId: string | null;
  nodes: AgentNode[];
  generations: CoordinatorGeneration[];
  tasks: OrchestrationTask[];
  reports: TaskReport[];
  commands: TaskCommand[];
  coordinatorNotifications: CoordinatorNotification[];
  dispatches: PromptDispatch[];
  revision: number;
  createdAt: string;
  updatedAt: string;
};

export type OrchestrationEvent = {
  workspaceId: string;
  revision: number;
  reason: string;
  taskId?: string | null;
  nodeId?: string | null;
};
