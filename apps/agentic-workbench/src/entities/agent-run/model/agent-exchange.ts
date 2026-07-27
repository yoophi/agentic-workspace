export type AgentPromptDelivery = "send" | "queue" | "draft";

export type AgentExchangeStatus =
  | "pending"
  | "accepted"
  | "delivered"
  | "rejected"
  | "failed"
  | "cancelled";

export type AgentPanelEndpoint = {
  panelId: string;
  title: string;
  runId: string | null;
  status: "idle" | "running" | "closing";
};

export type AgentExchangeEndpoint = AgentPanelEndpoint & {
  worktreePath?: string;
};

export type AgentExchange = {
  requestId: string;
  source: AgentExchangeEndpoint;
  target: AgentExchangeEndpoint;
  message: string;
  delivery: AgentPromptDelivery;
  status: AgentExchangeStatus;
  failureCode?: string | null;
  failureReason?: string | null;
  createdAt: string;
  updatedAt: string;
};

export type AgentExchangeRequestedEvent = Pick<
  AgentExchange,
  "requestId" | "source" | "target" | "message" | "delivery" | "createdAt"
>;

export type AgentWorkspaceSnapshotInput = {
  worktreePath: string;
  revision: number;
  focusedPanelId: string;
  panels: AgentPanelEndpoint[];
};
