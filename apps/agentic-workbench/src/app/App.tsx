import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Route,
  Routes,
  useLocation,
  useNavigate,
  useParams,
  useSearchParams,
} from "react-router-dom";

import { useProjectUiStore } from "@/app/model/project-ui-store";
import {
  createProject,
  deleteProject,
  listProjects,
  updateProject,
} from "@/entities/project/api/project-repository";
import {
  buildProjectWorktreeRoute,
  readWorktreePath,
} from "@/app/model/session-route";
import {
  listGitWorktrees,
  openWorktreeWindow,
} from "@/entities/project/api/git-worktree-repository";
import { gitStateRefreshQueryOptions } from "@/entities/project/api/query-options";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import { buildProjectDashboard } from "@/entities/project/lib/dashboard-summary";
import { formatWorktreeWindowTitle } from "@/entities/project/lib/worktree-window-title";
import type { DashboardAction } from "@/entities/project/model";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { Project, ProjectInput } from "@/entities/project/model/types";
import { DeleteProjectDialog } from "@/features/project-delete/ui/delete-project-dialog";
import { ProjectFormDialog } from "@/features/project-form/ui/project-form-dialog";
import type { OpenWorktreeMode } from "@/features/project-worktree/ui/project-worktree-card";
import { ProjectDashboardPage } from "@/pages/project-dashboard/ui/project-dashboard-page";
import { ProjectDetailPage } from "@/pages/project-detail/ui/project-detail-page";
import { ProjectListPage } from "@/pages/project-list/ui/project-list-page";
import { ProjectWorktreeSessionPage } from "@/pages/project-worktree-session/ui/project-worktree-session-page";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export function App() {
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const projectsQuery = useQuery({
    queryKey: projectQueryKeys.all,
    queryFn: listProjects,
  });
  const projects = projectsQuery.data ?? [];
  const {
    editingProject,
    deletingProject,
    isFormOpen,
    error,
    openCreateDialog,
    openEditDialog,
    openDeleteDialog,
    closeFormDialog,
    closeDeleteDialog,
    setError,
  } = useProjectUiStore();

  const createProjectMutation = useMutation({
    mutationFn: createProject,
    onSuccess: async (project) => {
      await queryClient.invalidateQueries({ queryKey: projectQueryKeys.all });
      navigate(`/projects/${project.id}`);
    },
  });
  const updateProjectMutation = useMutation({
    mutationFn: ({ id, input }: { id: string; input: ProjectInput }) =>
      updateProject(id, input),
    onSuccess: async (project) => {
      await queryClient.invalidateQueries({ queryKey: projectQueryKeys.all });
      navigate(`/projects/${project.id}`);
    },
  });
  const deleteProjectMutation = useMutation({
    mutationFn: deleteProject,
    onSuccess: async (_, deletedProjectId) => {
      await queryClient.invalidateQueries({ queryKey: projectQueryKeys.all });

      if (location.pathname === `/projects/${deletedProjectId}`) {
        navigate("/");
      }
    },
  });

  async function saveProject(input: ProjectInput) {
    setError(null);

    try {
      if (editingProject) {
        await updateProjectMutation.mutateAsync({
          id: editingProject.id,
          input,
        });
      } else {
        await createProjectMutation.mutateAsync(input);
      }

      closeFormDialog();
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  function openWorktree(
    project: Project,
    worktree: GitWorktree,
    mode: OpenWorktreeMode,
  ) {
    if (mode === "current") {
      navigate(buildProjectWorktreeRoute(project.id, worktree.path));
      return;
    }

    void openWorktreeWindow(project.id, project.name, worktree.path, mode).catch((caughtError) =>
      setError(String(caughtError)),
    );
  }

  function handleDashboardAction(action: DashboardAction) {
    if (!action.enabled) {
      return;
    }

    if (action.kind === "createProject" || action.kind === "openExistingProject") {
      openCreateDialog();
      return;
    }

    if (action.kind === "retry") {
      void projectsQuery.refetch();
      return;
    }

    if (action.target?.type === "route") {
      navigate(action.target.to);
      return;
    }

    if (action.target?.type === "worktree") {
      navigate(
        buildProjectWorktreeRoute(
          action.target.projectId,
          action.target.worktreePath,
        ),
      );
    }
  }

  async function confirmDeleteProject() {
    if (!deletingProject) {
      return;
    }

    setError(null);

    try {
      await deleteProjectMutation.mutateAsync(deletingProject.id);
      closeDeleteDialog();
    } catch (caughtError) {
      setError(String(caughtError));
    }
  }

  const isWorktreeSessionPage =
    location.pathname.startsWith("/session/") ||
    location.pathname.endsWith("/worktrees");
  const projectDashboard = buildProjectDashboard({
    projects,
    isLoading: projectsQuery.isLoading,
    errorMessage: projectsQuery.error ? String(projectsQuery.error) : null,
  });

  return (
    <main
      className={cn(
        "min-h-svh bg-muted/30",
        isWorktreeSessionPage ? "p-0" : "p-6",
      )}
    >
      <div
        className={cn(
          "mx-auto flex w-full flex-col",
          isWorktreeSessionPage ? "max-w-none gap-0" : "max-w-6xl gap-6",
        )}
      >
        {error && (
          <p className="rounded-md border bg-background px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        )}

        <Routes>
          <Route
            path="/"
            element={
              <ProjectDashboardPage
                dashboard={projectDashboard}
                onAction={handleDashboardAction}
              />
            }
          />
          <Route
            path="/projects"
            element={
              <ProjectListPage
                projects={projects}
                isLoading={projectsQuery.isLoading}
                onRefresh={() => void projectsQuery.refetch()}
                onCreateProject={openCreateDialog}
                onSelectProject={(project) => navigate(`/projects/${project.id}`)}
                onEditProject={openEditDialog}
                onDeleteProject={openDeleteDialog}
              />
            }
          />
          <Route
            path="/projects/:projectId"
            element={
              <ProjectDetailRoute
                projects={projects}
                isLoading={projectsQuery.isLoading}
                onBack={() => navigate("/")}
                onEditProject={openEditDialog}
                onDeleteProject={openDeleteDialog}
                onOpenWorktree={openWorktree}
              />
            }
          />
          <Route
            path="/projects/:projectId/worktrees"
            element={
              <ProjectWorktreeSessionRoute
                projects={projects}
                isLoading={projectsQuery.isLoading}
                onBack={(projectId) => navigate(`/projects/${projectId}`)}
              />
            }
          />
          <Route
            path="/session/:projectId"
            element={
              <ProjectWorktreeSessionRoute
                projects={projects}
                isLoading={projectsQuery.isLoading}
                standalone
              />
            }
          />
        </Routes>
      </div>

      <ProjectFormDialog
        project={editingProject}
        open={isFormOpen}
        error={error}
        onOpenChange={(open) => {
          if (!open) {
            closeFormDialog();
          }
        }}
        onSubmit={saveProject}
        onError={setError}
      />

      <DeleteProjectDialog
        project={deletingProject}
        onOpenChange={(open) => {
          if (!open) {
            closeDeleteDialog();
          }
        }}
        onConfirm={confirmDeleteProject}
      />
    </main>
  );
}

function ProjectDetailRoute({
  projects,
  isLoading,
  onBack,
  onEditProject,
  onDeleteProject,
  onOpenWorktree,
}: {
  projects: Project[];
  isLoading: boolean;
  onBack: () => void;
  onEditProject: (project: Project) => void;
  onDeleteProject: (project: Project) => void;
  onOpenWorktree: (
    project: Project,
    worktree: GitWorktree,
    mode: OpenWorktreeMode,
  ) => void;
}) {
  const { projectId } = useParams();
  const project = projects.find((project) => project.id === projectId);

  if (project) {
    return (
      <ProjectDetailPage
        project={project}
        onBack={onBack}
        onEditProject={onEditProject}
        onDeleteProject={onDeleteProject}
        onOpenWorktree={(worktree, mode) => onOpenWorktree(project, worktree, mode)}
      />
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <p className="rounded-md border bg-background px-3 py-2 text-sm text-muted-foreground">
        {isLoading
          ? "프로젝트를 불러오는 중입니다."
          : "프로젝트를 찾을 수 없습니다."}
      </p>
      <Button type="button" variant="outline" className="w-fit" onClick={onBack}>
        목록
      </Button>
    </div>
  );
}

function ProjectWorktreeSessionRoute({
  projects,
  isLoading,
  onBack,
  standalone = false,
}: {
  projects: Project[];
  isLoading: boolean;
  onBack?: (projectId: string) => void;
  standalone?: boolean;
}) {
  const { projectId } = useParams();
  const [searchParams] = useSearchParams();
  const project = projects.find((project) => project.id === projectId);
  const decodedWorktreePath = readWorktreePath(searchParams);
  const worktreesQuery = useQuery({
    queryKey: project
      ? projectQueryKeys.gitWorktrees(project.workingDirectory)
      : ["git-worktrees", "missing-project"],
    queryFn: () => listGitWorktrees(project?.workingDirectory ?? ""),
    enabled: Boolean(project),
    ...gitStateRefreshQueryOptions,
  });
  const worktree = worktreesQuery.data?.find(
    (worktree) => worktree.path === decodedWorktreePath,
  );
  const windowTitle =
    project && worktree
      ? formatWorktreeWindowTitle(project.name, worktree.path)
      : "ACP Worktree Session";

  useEffect(() => {
    if (!standalone) {
      return;
    }

    void getCurrentWindow().setTitle(windowTitle);
  }, [standalone, windowTitle]);

  if (project && worktree) {
    return (
      <ProjectWorktreeSessionPage
        project={project}
        worktree={worktree}
        onBack={standalone || !onBack ? undefined : () => onBack(project.id)}
      />
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <p className="rounded-md border bg-background px-3 py-2 text-sm text-muted-foreground">
        {isLoading || worktreesQuery.isLoading
          ? "작업 화면을 불러오는 중입니다."
          : "선택한 worktree를 찾을 수 없습니다."}
      </p>
      {!standalone && onBack && (
        <Button
          type="button"
          variant="outline"
          className="w-fit"
          onClick={() => (projectId ? onBack(projectId) : undefined)}
        >
          프로젝트
        </Button>
      )}
    </div>
  );
}
