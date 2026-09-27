// 037–040: 생성 타입과 OperationMap. 043(4단계): 운영용 호출 클라이언트·연결 수명·오류 문자열.
export type * from "./generated/workbench";
export type {
  AgentDescriptor,
  AgentExchange,
  AgentPanelEndpoint,
  AgentRun,
  AgentRunRequest,
  AgentRunSettings,
  AgentToolCandidateResponse,
  AgentWorkspaceSyncResponse,
  BenchCloseOutput,
  BenchOpenInput,
  BenchOpenOutput,
  BenchTitleRequested,
  Call,
  CallReply,
  CallReplyByOperation,
  CallRequest,
  DescribeOutput,
  EventBySchema,
  EventEnvelope,
  EventFrame,
  EventItem,
  EventMap,
  EventSchemaId,
  ExchangeRequested,
  FaultCode,
  GapNotice,
  GapReason,
  GitBranch,
  GitCommitDetail,
  GitCommitGraph,
  GitCommitHistory,
  GitFileDiff,
  GitRemote,
  GitWorktree,
  GitWorktreeChanges,
  Goal,
  GoalStatus,
  OperationDescriptor,
  OperationId,
  OperationMap,
  OrchestrationWorkspaceUpdated,
  Outcome,
  Project,
  ProjectCreateInput,
  ProviderSession,
  RunEvent,
  SavedPrompt,
  Schemas,
  StreamCursor,
  TitleChangeResult,
  WorkbenchFault,
  WorktreeChange,
  WorktreeChanged,
  WorktreeFileEntry,
  WorktreeTextFile,
} from "./operation-map";
export { CALLS_PATH, PROTOCOL_VERSION } from "./operation-map";
export { createWorkbenchClient } from "./call-client";
export type { CallOptions, CallOutcome, ConnectionPort, WorkbenchClient } from "./call-client";
export { createConnection, PROTOCOL_VERSIONS } from "./connection";
export type { Connection, ConnectionInfo, ConnectionOptions, ConnectionState, Handshake } from "./connection";
export { faultToString } from "./fault-string";
export type { FaultFlavor } from "./fault-string";
export { OPERATION_KINDS, isQueryOperation } from "./operation-kinds";
export type { OperationKind } from "./operation-kinds";
export { createEventClient } from "./event-client";
export type {
  EventClient,
  EventClientOptions,
  EventConnectionPort,
  SnapshotSource,
  SocketLike,
  StreamListener,
  ResetContext,
  SubscribeOptions,
} from "./event-client";
