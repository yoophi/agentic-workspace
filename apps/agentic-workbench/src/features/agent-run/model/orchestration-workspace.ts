import type {
  OrchestrationEvent,
  OrchestrationSession,
} from "@/entities/agent-orchestration";

export type OrchestrationWorkspaceControllerState = {
  phase: "idle" | "loading" | "ready" | "error";
  session: OrchestrationSession | null;
  errorMessage: string | null;
};

export function createOrchestrationWorkspaceState(): OrchestrationWorkspaceControllerState {
  return { phase: "idle", session: null, errorMessage: null };
}

export function hydrateOrchestrationWorkspace(
  state: OrchestrationWorkspaceControllerState,
  session: OrchestrationSession,
): OrchestrationWorkspaceControllerState {
  if (
    state.session?.id === session.id &&
    state.session.revision > session.revision
  ) {
    return state;
  }
  return { phase: "ready", session, errorMessage: null };
}

export function applyOrchestrationEvent(
  state: OrchestrationWorkspaceControllerState,
  event: OrchestrationEvent,
): {
  state: OrchestrationWorkspaceControllerState;
  needsRehydrate: boolean;
} {
  const current = state.session;
  if (!current || current.id !== event.workspaceId) {
    return { state, needsRehydrate: true };
  }
  if (event.revision <= current.revision) {
    return { state, needsRehydrate: false };
  }
  return {
    state,
    needsRehydrate: event.revision !== current.revision + 1,
  };
}

export function shouldBindMainRun(
  boundRunId: string | null,
  activeRunId: string | null,
): activeRunId is string {
  return Boolean(activeRunId && boundRunId !== activeRunId);
}

export function selectMainNode(session: OrchestrationSession) {
  return session.nodes.find(
    (node) => node.id === session.mainNodeId && node.kind === "main",
  );
}

export function selectDirectChildren(session: OrchestrationSession) {
  return session.nodes.filter(
    (node) =>
      node.kind === "child" && node.parentNodeId === session.mainNodeId,
  );
}

export function selectChildResultSummaries(
  session: OrchestrationSession,
) {
  return selectDirectChildren(session)
    .map((node) => {
      const task = session.tasks.find(
        (candidate) => candidate.id === node.assignedTaskId,
      );
      const reports = task
        ? session.reports.filter((report) => report.taskId === task.id)
        : [];
      return {
        nodeId: node.id,
        role: node.role.name,
        task,
        reports,
        result:
          reports.find((report) => report.id === task?.latestResultReportId) ??
          null,
        unresolved: reports.flatMap((report) => report.unresolved),
      };
    })
    .filter((summary) => summary.task);
}
