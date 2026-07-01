import type {
  ChangeSummary,
  DashboardAction,
  ProjectDashboard,
  ProjectDashboardItem,
  SessionSummary,
  WorktreeSummary,
} from "@/entities/project/model/dashboard";
import type { Project } from "@/entities/project/model/types";

type SummaryMap<T> = Partial<Record<string, T>>;

export type BuildProjectDashboardInput = {
  projects: Project[];
  isLoading: boolean;
  errorMessage?: string | null;
  sessionsByProjectId?: SummaryMap<SessionSummary>;
  worktreesByProjectId?: SummaryMap<WorktreeSummary>;
  changesByProjectId?: SummaryMap<ChangeSummary>;
};

export function buildProjectDashboard({
  projects,
  isLoading,
  errorMessage,
  sessionsByProjectId = {},
  worktreesByProjectId = {},
  changesByProjectId = {},
}: BuildProjectDashboardInput): ProjectDashboard {
  const items = sortDashboardItems(
    projects.map((project, index) =>
      buildProjectDashboardItem({
        project,
        originalIndex: index,
        sessionSummary: sessionsByProjectId[project.id],
        worktreeSummary: worktreesByProjectId[project.id],
        changeSummary: changesByProjectId[project.id],
      }),
    ),
  );

  return {
    projects: items,
    quickActions: buildDashboardQuickActions({
      hasProjects: projects.length > 0,
      canRetry: Boolean(errorMessage),
    }),
    status: getDashboardStatus({
      isLoading,
      projectCount: projects.length,
      errorMessage,
    }),
    errorMessage: errorMessage ?? undefined,
  };
}

export function buildProjectDashboardItem({
  project,
  sessionSummary,
  worktreeSummary,
  changeSummary,
  originalIndex = 0,
}: {
  project: Project;
  sessionSummary?: SessionSummary;
  worktreeSummary?: WorktreeSummary;
  changeSummary?: ChangeSummary;
  originalIndex?: number;
}): ProjectDashboardItem {
  const openProjectAction = buildOpenProjectAction(project.id);
  const openWorktreeAction =
    worktreeSummary?.primaryWorktreePath && worktreeSummary.status === "ready"
      ? buildOpenWorktreeAction(project.id, worktreeSummary.primaryWorktreePath)
      : undefined;
  const resumeAction =
    sessionSummary?.resumable && sessionSummary.routeTarget
      ? buildResumeSessionAction(sessionSummary)
      : undefined;

  const primaryAction = resumeAction ?? openWorktreeAction ?? openProjectAction;
  const secondaryActions = [
    primaryAction.id !== openProjectAction.id ? openProjectAction : undefined,
    openWorktreeAction && primaryAction.id !== openWorktreeAction.id
      ? openWorktreeAction
      : undefined,
  ].filter((action): action is DashboardAction => Boolean(action));
  const lastActivityMs = sessionSummary?.lastActivityMs;

  return {
    projectId: project.id,
    name: project.name,
    workingDirectory: project.workingDirectory,
    description: project.description,
    lastActivityLabel: sessionSummary?.lastActivityLabel,
    lastActivityMs,
    primaryAction,
    secondaryActions,
    sessionSummary,
    worktreeSummary,
    changeSummary,
    summaryStatus: getSummaryStatus({
      sessionSummary,
      worktreeSummary,
      changeSummary,
    }),
    unavailableReason: getUnavailableReason({
      worktreeSummary,
      changeSummary,
    }),
    originalIndex,
  } as ProjectDashboardItem & { originalIndex: number };
}

export function sortDashboardItems(items: ProjectDashboardItem[]) {
  return [...items].sort((first, second) => {
    const firstActivity = first.lastActivityMs ?? Number.NEGATIVE_INFINITY;
    const secondActivity = second.lastActivityMs ?? Number.NEGATIVE_INFINITY;

    if (firstActivity !== secondActivity) {
      return secondActivity - firstActivity;
    }

    const firstIndex = getOriginalIndex(first);
    const secondIndex = getOriginalIndex(second);

    if (firstIndex !== secondIndex) {
      return firstIndex - secondIndex;
    }

    return first.name.localeCompare(second.name, "ko");
  });
}

export function getDashboardStatus({
  isLoading,
  projectCount,
  errorMessage,
}: {
  isLoading: boolean;
  projectCount: number;
  errorMessage?: string | null;
}) {
  if (errorMessage) {
    return "error" as const;
  }

  if (isLoading) {
    return "loading" as const;
  }

  return projectCount === 0 ? ("empty" as const) : ("ready" as const);
}

export function getSummaryStatus({
  sessionSummary,
  worktreeSummary,
  changeSummary,
}: {
  sessionSummary?: SessionSummary;
  worktreeSummary?: WorktreeSummary;
  changeSummary?: ChangeSummary;
}) {
  const statuses = [worktreeSummary?.status, changeSummary?.status].filter(
    Boolean,
  );

  if (statuses.includes("unavailable")) {
    return sessionSummary ? ("partial" as const) : ("unavailable" as const);
  }

  if (statuses.includes("loading")) {
    return "partial" as const;
  }

  return "complete" as const;
}

export function buildDashboardQuickActions({
  hasProjects,
  canRetry = false,
}: {
  hasProjects: boolean;
  canRetry?: boolean;
}): DashboardAction[] {
  return [
    {
      id: "create-project",
      label: "프로젝트 생성",
      kind: "createProject",
      enabled: true,
      target: { type: "dialog", name: "createProject" },
    },
    {
      id: "open-existing-project",
      label: hasProjects ? "프로젝트 열기" : "기존 프로젝트 열기",
      kind: "openExistingProject",
      enabled: true,
      target: { type: "dialog", name: "openExistingProject" },
    },
    {
      id: "retry-projects",
      label: "다시 시도",
      kind: "retry",
      enabled: canRetry,
      disabledReason: canRetry ? undefined : "다시 시도할 오류가 없습니다.",
    },
  ];
}

function buildOpenProjectAction(projectId: string): DashboardAction {
  return {
    id: `open-project-${projectId}`,
    label: "프로젝트 열기",
    kind: "openProject",
    enabled: true,
    target: { type: "route", to: `/projects/${projectId}` },
  };
}

function buildOpenWorktreeAction(
  projectId: string,
  worktreePath: string,
): DashboardAction {
  return {
    id: `open-worktree-${projectId}`,
    label: "Worktree 열기",
    kind: "openWorktree",
    enabled: true,
    target: { type: "worktree", projectId, worktreePath },
  };
}

function buildResumeSessionAction(sessionSummary: SessionSummary): DashboardAction {
  return {
    id: `resume-session-${sessionSummary.projectId}-${sessionSummary.sessionId}`,
    label: "세션 재개",
    kind: "resumeSession",
    enabled: true,
    target: { type: "route", to: sessionSummary.routeTarget ?? "" },
  };
}

function getUnavailableReason({
  worktreeSummary,
  changeSummary,
}: {
  worktreeSummary?: WorktreeSummary;
  changeSummary?: ChangeSummary;
}) {
  if (worktreeSummary?.status === "unavailable") {
    return "worktree 상태를 확인할 수 없습니다.";
  }

  if (changeSummary?.status === "unavailable") {
    return "변경 상태를 확인할 수 없습니다.";
  }

  return undefined;
}

function getOriginalIndex(item: ProjectDashboardItem) {
  return (item as ProjectDashboardItem & { originalIndex?: number }).originalIndex ?? 0;
}
