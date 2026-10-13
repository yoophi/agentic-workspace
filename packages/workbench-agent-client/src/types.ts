import type {
  AgentDescriptor,
  AgentRunRequest,
  CallOutcome,
  ConnectionInfo,
  ConnectionState,
  RunEvent,
  SocketLike,
} from "@yoophi/workbench-client";

export type SessionStatus =
  | "creating"
  | "ready"
  | "busy"
  | "closing"
  | "uncertain"
  | "failed"
  | "closed"
  | "lost";
export type CommandOutcome = CallOutcome<void>;
export type MessageDelivery = { kind: "pending" } | CommandOutcome;
export interface ChatMessage {
  readonly id: string;
  readonly role: "user" | "assistant" | "thought";
  readonly text: string;
  readonly delivery?: MessageDelivery;
}
export interface ChatEvent {
  readonly sequence: number;
  readonly event: RunEvent;
}
export interface ChatPermission {
  readonly id: string;
  readonly title: string;
  readonly input: unknown;
  readonly options: ReadonlyArray<{
    optionId: string;
    name: string;
    kind: string;
  }>;
  readonly responding: boolean;
  readonly outcome?: CommandOutcome;
}
export interface ChatSession {
  readonly id: string;
  readonly agentId: string;
  readonly label: string;
  readonly status: SessionStatus;
  readonly messages: readonly ChatMessage[];
  readonly events: readonly ChatEvent[];
  readonly permissions: readonly ChatPermission[];
  readonly lastSequence: number;
  readonly historyIncomplete: boolean;
  readonly error?: string;
  readonly streamError?: string;
  readonly creationOutcome?: CallOutcome<{ id: string }>;
  readonly closeOutcome?: CommandOutcome;
}
export interface AgentChatSnapshot {
  readonly connection: ConnectionState;
  readonly benchId?: string;
  readonly sessions: readonly ChatSession[];
  readonly disposed: boolean;
}
export type SessionOptions = Omit<
  AgentRunRequest,
  "goal" | "cwd" | "workspaceId" | "checkoutId" | "ralphLoop" | "runId"
> & {
  label?: string;
  runId?: string;
};
export interface AgentChatOptions {
  workspace: string;
  connectionProvider: () => Promise<ConnectionInfo>;
  /** Inject browser primitives for hosts and tests; all wire/reconnect logic remains in workbench-client. */
  fetch?: typeof fetch;
  openSocket?: (url: string) => SocketLike;
}
export interface AgentChatClient {
  connect(): Promise<CallOutcome<{ benchId: string }>>;
  listAgents(): Promise<CallOutcome<AgentDescriptor[]>>;
  createSession(options: SessionOptions): Promise<CallOutcome<{ id: string }>>;
  send(sessionId: string, text: string): Promise<CommandOutcome>;
  respondPermission(
    sessionId: string,
    permissionId: string,
    optionId: string,
  ): Promise<CommandOutcome>;
  closeSession(sessionId: string): Promise<CommandOutcome>;
  subscribe(listener: (snapshot: AgentChatSnapshot) => void): () => void;
  getSnapshot(): AgentChatSnapshot;
  dispose(): void;
}

export class AgentChatError extends Error {
  constructor(
    readonly code:
      | "disposed"
      | "notConnected"
      | "sessionNotReady"
      | "sessionMissing"
      | "invalidPermission"
      | "invalidMessage"
      | "duplicateSession"
      | "epochChanged",
    message: string,
  ) {
    super(message);
    this.name = "AgentChatError";
  }
}
