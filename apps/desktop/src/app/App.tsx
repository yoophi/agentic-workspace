import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Route, Routes, useLocation, useNavigate, useParams } from "react-router-dom";

import { useProjectUiStore } from "@/app/model/project-ui-store";
import {
  createProject,
  deleteProject,
  listProjects,
  updateProject,
} from "@/entities/project/api/project-repository";
import { listGitWorktrees } from "@/entities/project/api/git-worktree-repository";
import { projectQueryKeys } from "@/entities/project/api/query-keys";
import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { Project, ProjectInput } from "@/entities/project/model/types";
import { DeleteProjectDialog } from "@/features/project-delete/ui/delete-project-dialog";
import { ProjectFormDialog } from "@/features/project-form/ui/project-form-dialog";
import { ProjectDetailPage } from "@/pages/project-detail/ui/project-detail-page";
import { ProjectListPage } from "@/pages/project-list/ui/project-list-page";
import { ProjectWorktreeSessionPage } from "@/pages/project-worktree-session/ui/project-worktree-session-page";
import { Button } from "@/components/ui/button";

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

  return (
    <main className="min-h-svh bg-muted/30 p-6">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6">
        {error && (
          <p className="rounded-md border bg-background px-3 py-2 text-sm text-destructive">
            {error}
          </p>
        )}

        <Routes>
          <Route
            path="/"
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
                onOpenWorktree={(project, worktree) =>
                  navigate(
                    `/projects/${project.id}/worktrees/${encodeURIComponent(worktree.path)}`,
                  )
                }
              />
            }
          />
          <Route
            path="/projects/:projectId/worktrees/:worktreePath"
            element={
              <ProjectWorktreeSessionRoute
                projects={projects}
                isLoading={projectsQuery.isLoading}
                onBack={(projectId) => navigate(`/projects/${projectId}`)}
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
  onOpenWorktree: (project: Project, worktree: GitWorktree) => void;
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
        onOpenWorktree={(worktree) => onOpenWorktree(project, worktree)}
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
}: {
  projects: Project[];
  isLoading: boolean;
  onBack: (projectId: string) => void;
}) {
  const { projectId, worktreePath } = useParams();
  const project = projects.find((project) => project.id === projectId);
  const decodedWorktreePath = worktreePath ? decodeURIComponent(worktreePath) : "";
  const worktreesQuery = useQuery({
    queryKey: project
      ? projectQueryKeys.gitWorktrees(project.workingDirectory)
      : ["git-worktrees", "missing-project"],
    queryFn: () => listGitWorktrees(project?.workingDirectory ?? ""),
    enabled: Boolean(project),
  });
  const worktree = worktreesQuery.data?.find(
    (worktree) => worktree.path === decodedWorktreePath,
  );

  if (project && worktree) {
    return (
      <ProjectWorktreeSessionPage
        project={project}
        worktree={worktree}
        onBack={() => onBack(project.id)}
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
      <Button
        type="button"
        variant="outline"
        className="w-fit"
        onClick={() => (projectId ? onBack(projectId) : undefined)}
      >
        프로젝트
      </Button>
    </div>
  );
}
