export type DashboardStatus = "loading" | "ready" | "empty" | "error";

export type DashboardSummaryStatus = "complete" | "partial" | "unavailable";

export type DashboardActionKind =
  | "createProject"
  | "openProject"
  | "openExistingProject"
  | "resumeSession"
  | "openWorktree"
  | "retry";

export type DashboardActionTarget =
  | { type: "dialog"; name: "createProject" | "openExistingProject" }
  | { type: "route"; to: string }
  | { type: "worktree"; projectId: string; worktreePath: string };

export type DashboardAction = {
  id: string;
  label: string;
  kind: DashboardActionKind;
  enabled: boolean;
  target?: DashboardActionTarget;
  disabledReason?: string;
};

export type SessionSummary = {
  sessionId: string;
  projectId: string;
  label: string;
  lastActivityLabel?: string;
  lastActivityMs?: number;
  resumable: boolean;
  routeTarget?: string;
};

export type WorktreeSummary = {
  projectId: string;
  count: number;
  activeCount: number;
  primaryWorktreePath?: string;
  status: "ready" | "loading" | "unavailable";
};

export type ChangeSummary = {
  changedFileCount?: number;
  hasChanges?: boolean;
  status: "ready" | "loading" | "unavailable";
};

export type ProjectDashboardItem = {
  projectId: string;
  name: string;
  workingDirectory: string;
  description?: string | null;
  lastActivityLabel?: string;
  lastActivityMs?: number;
  primaryAction: DashboardAction;
  secondaryActions: DashboardAction[];
  worktreeSummary?: WorktreeSummary;
  sessionSummary?: SessionSummary;
  changeSummary?: ChangeSummary;
  summaryStatus: DashboardSummaryStatus;
  unavailableReason?: string;
};

export type ProjectDashboard = {
  projects: ProjectDashboardItem[];
  quickActions: DashboardAction[];
  status: DashboardStatus;
  errorMessage?: string;
};
