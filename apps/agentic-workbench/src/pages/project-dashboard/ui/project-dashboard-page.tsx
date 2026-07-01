import {
  AlertCircleIcon,
  Clock3Icon,
  FolderKanbanIcon,
  GitBranchIcon,
  ListChecksIcon,
  LoaderCircleIcon,
  SparklesIcon,
} from "lucide-react";

import type {
  DashboardAction,
  ProjectDashboard,
  ProjectDashboardItem,
} from "@/entities/project/model";
import { ProjectDashboardActions } from "@/features/project-dashboard/ui/project-dashboard-actions";
import { Badge } from "@/components/ui/badge";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { EllipsisPopoverText } from "@/shared/ui/ellipsis-popover-text";
import { cn } from "@/lib/utils";

type ProjectDashboardPageProps = {
  dashboard: ProjectDashboard;
  onAction: (action: DashboardAction) => void;
};

export function ProjectDashboardPage({
  dashboard,
  onAction,
}: ProjectDashboardPageProps) {
  return (
    <div className="flex min-w-0 flex-col gap-5">
      <header className="flex min-w-0 flex-col gap-4 border-b pb-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="flex min-w-0 flex-col gap-2">
          <Badge variant="secondary" className="w-fit">
            Project Dashboard
          </Badge>
          <div className="flex min-w-0 flex-col gap-1">
            <h1 className="text-2xl font-semibold tracking-normal">
              작업 대시보드
            </h1>
            <p className="max-w-3xl text-sm text-muted-foreground">
              최근 프로젝트 상태를 확인하고 바로 이어서 작업합니다.
            </p>
          </div>
        </div>
        <ProjectDashboardActions
          actions={dashboard.quickActions.filter(
            (action) => action.kind !== "retry" || dashboard.status === "error",
          )}
          onAction={onAction}
        />
      </header>

      {dashboard.status === "loading" && (
        <DashboardState
          icon={<LoaderCircleIcon className="animate-spin" />}
          title="프로젝트를 불러오는 중입니다."
          description="저장된 프로젝트와 최근 작업 상태를 확인하고 있습니다."
        />
      )}

      {dashboard.status === "error" && (
        <DashboardState
          icon={<AlertCircleIcon />}
          title="프로젝트 정보를 불러오지 못했습니다."
          description={dashboard.errorMessage ?? "잠시 후 다시 시도하세요."}
          actions={dashboard.quickActions.filter(
            (action) => action.kind === "retry" || action.kind === "createProject",
          )}
          onAction={onAction}
        />
      )}

      {dashboard.status === "empty" && (
        <DashboardState
          icon={<FolderKanbanIcon />}
          title="등록된 프로젝트가 없습니다."
          description="새 프로젝트를 만들거나 로컬 디렉토리를 선택해 작업을 시작하세요."
          actions={dashboard.quickActions.filter(
            (action) =>
              action.kind === "createProject" ||
              action.kind === "openExistingProject",
          )}
          onAction={onAction}
        />
      )}

      {dashboard.status === "ready" && (
        <section className="flex min-w-0 flex-col gap-3">
          <div className="flex min-w-0 items-center justify-between gap-3">
            <div className="min-w-0">
              <h2 className="text-base font-medium tracking-normal">
                최근 프로젝트
              </h2>
              <p className="text-sm text-muted-foreground">
                총 {dashboard.projects.length.toLocaleString("ko-KR")}개 프로젝트
              </p>
            </div>
          </div>

          <div className="grid min-w-0 gap-3">
            {dashboard.projects.map((project) => (
              <ProjectDashboardRow
                key={project.projectId}
                project={project}
                onAction={onAction}
              />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function ProjectDashboardRow({
  project,
  onAction,
}: {
  project: ProjectDashboardItem;
  onAction: (action: DashboardAction) => void;
}) {
  return (
    <article className="grid min-w-0 gap-3 rounded-lg border bg-background p-3 shadow-xs lg:grid-cols-[minmax(0,1fr)_auto] lg:items-center">
      <div className="grid min-w-0 gap-3 md:grid-cols-[minmax(0,1.2fr)_minmax(16rem,0.8fr)]">
        <div className="flex min-w-0 flex-col gap-2">
          <div className="flex min-w-0 items-center gap-2">
            <FolderKanbanIcon className="size-4 shrink-0 text-muted-foreground" />
            <EllipsisPopoverText
              value={project.name}
              className="text-sm font-medium"
            />
            <SummaryBadge project={project} />
          </div>
          <EllipsisPopoverText
            value={project.workingDirectory}
            className="font-mono text-xs text-muted-foreground"
            contentClassName="font-mono text-xs"
          />
          {project.description && (
            <EllipsisPopoverText
              value={project.description}
              className="text-xs text-muted-foreground"
            />
          )}
        </div>

        <div className="grid min-w-0 gap-2 text-xs text-muted-foreground sm:grid-cols-3 md:grid-cols-1 xl:grid-cols-3">
          <SummaryMetric
            icon={<Clock3Icon />}
            label="최근 활동"
            value={project.lastActivityLabel ?? "활동 정보 없음"}
          />
          <SummaryMetric
            icon={<GitBranchIcon />}
            label="Worktree"
            value={formatWorktreeSummary(project)}
          />
          <SummaryMetric
            icon={<ListChecksIcon />}
            label="변경 상태"
            value={formatChangeSummary(project)}
          />
        </div>
      </div>

      <div className="flex min-w-0 flex-wrap items-center gap-2 lg:justify-end">
        <ProjectDashboardActions
          actions={[project.primaryAction, ...project.secondaryActions]}
          onAction={onAction}
          compact
        />
      </div>
    </article>
  );
}

function DashboardState({
  icon,
  title,
  description,
  actions = [],
  onAction,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
  actions?: DashboardAction[];
  onAction?: (action: DashboardAction) => void;
}) {
  return (
    <Empty className="min-h-[22rem] bg-background">
      <EmptyHeader>
        <EmptyMedia variant="icon">{icon}</EmptyMedia>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
      {actions.length > 0 && onAction && (
        <EmptyContent>
          <ProjectDashboardActions actions={actions} onAction={onAction} />
        </EmptyContent>
      )}
    </Empty>
  );
}

function SummaryBadge({ project }: { project: ProjectDashboardItem }) {
  if (project.summaryStatus === "complete") {
    return (
      <Badge variant="outline" className="hidden sm:inline-flex">
        정상
      </Badge>
    );
  }

  if (project.summaryStatus === "partial") {
    return (
      <Badge variant="secondary" className="hidden sm:inline-flex">
        일부 확인
      </Badge>
    );
  }

  return (
    <Badge variant="destructive" className="hidden sm:inline-flex">
      확인 불가
    </Badge>
  );
}

function SummaryMetric({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="flex min-w-0 items-center gap-2 rounded-md bg-muted/50 px-2 py-1.5">
      <span className="shrink-0 text-muted-foreground [&_svg]:size-3.5">
        {icon}
      </span>
      <span className="shrink-0 text-muted-foreground">{label}</span>
      <EllipsisPopoverText
        value={value}
        focusable={false}
        className={cn(
          "text-right text-foreground sm:text-left",
          value === "확인 불가" && "text-destructive",
        )}
      />
    </div>
  );
}

function formatWorktreeSummary(project: ProjectDashboardItem) {
  if (!project.worktreeSummary) {
    return "요약 없음";
  }

  if (project.worktreeSummary.status === "unavailable") {
    return "확인 불가";
  }

  if (project.worktreeSummary.status === "loading") {
    return "확인 중";
  }

  return `${project.worktreeSummary.activeCount}/${project.worktreeSummary.count}개 활성`;
}

function formatChangeSummary(project: ProjectDashboardItem) {
  if (!project.changeSummary) {
    return "요약 없음";
  }

  if (project.changeSummary.status === "unavailable") {
    return "확인 불가";
  }

  if (project.changeSummary.status === "loading") {
    return "확인 중";
  }

  if (typeof project.changeSummary.changedFileCount === "number") {
    return project.changeSummary.changedFileCount > 0
      ? `${project.changeSummary.changedFileCount}개 변경`
      : "변경 없음";
  }

  return project.changeSummary.hasChanges ? "변경 있음" : "변경 없음";
}
