import type { Meta, StoryObj } from "@storybook/react-vite";

import { ProjectDetailPage } from "@/pages/project-detail/ui/project-detail-page";
import { ProjectDashboardPage } from "@/pages/project-dashboard/ui/project-dashboard-page";
import { ProjectListPage } from "@/pages/project-list/ui/project-list-page";
import { ProjectWorktreeSessionPage } from "@/pages/project-worktree-session/ui/project-worktree-session-page";
import {
  sampleEmptyProjectDashboard,
  sampleErrorProjectDashboard,
  sampleLoadingProjectDashboard,
  sampleLongContentProjectDashboard,
  samplePartialProjectDashboard,
  sampleProjectDashboard,
  sampleProjectDashboardWithQuickActions,
  sampleProjects,
  sampleWorktrees,
} from "@/shared/storybook/sample-data";

const meta = {
  title: "Atomic Design/Pages/Registered Components",
  parameters: {
    docs: {
      description: {
        component: "Screen-level compositions from the pages layer.",
      },
    },
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const ProjectDashboard: Story = {
  render: () => (
    <ProjectDashboardPage
      dashboard={sampleProjectDashboard}
      onAction={() => undefined}
    />
  ),
};

export const ProjectDashboardQuickActions: Story = {
  render: () => (
    <ProjectDashboardPage
      dashboard={sampleProjectDashboardWithQuickActions}
      onAction={() => undefined}
    />
  ),
};

export const EmptyProjectDashboard: Story = {
  render: () => (
    <ProjectDashboardPage
      dashboard={sampleEmptyProjectDashboard}
      onAction={() => undefined}
    />
  ),
};

export const LoadingProjectDashboard: Story = {
  render: () => (
    <ProjectDashboardPage
      dashboard={sampleLoadingProjectDashboard}
      onAction={() => undefined}
    />
  ),
};

export const ErrorProjectDashboard: Story = {
  render: () => (
    <ProjectDashboardPage
      dashboard={sampleErrorProjectDashboard}
      onAction={() => undefined}
    />
  ),
};

export const PartialProjectDashboard: Story = {
  render: () => (
    <ProjectDashboardPage
      dashboard={samplePartialProjectDashboard}
      onAction={() => undefined}
    />
  ),
};

export const LongContentProjectDashboard: Story = {
  render: () => (
    <ProjectDashboardPage
      dashboard={sampleLongContentProjectDashboard}
      onAction={() => undefined}
    />
  ),
};

export const ProjectList: Story = {
  render: () => (
    <ProjectListPage
      projects={sampleProjects}
      isLoading={false}
      onRefresh={() => undefined}
      onCreateProject={() => undefined}
      onSelectProject={() => undefined}
      onEditProject={() => undefined}
      onDeleteProject={() => undefined}
    />
  ),
};

export const EmptyProjectList: Story = {
  render: () => (
    <ProjectListPage
      projects={[]}
      isLoading={false}
      onRefresh={() => undefined}
      onCreateProject={() => undefined}
      onSelectProject={() => undefined}
      onEditProject={() => undefined}
      onDeleteProject={() => undefined}
    />
  ),
};

export const LoadingProjectList: Story = {
  render: () => (
    <ProjectListPage
      projects={[]}
      isLoading
      onRefresh={() => undefined}
      onCreateProject={() => undefined}
      onSelectProject={() => undefined}
      onEditProject={() => undefined}
      onDeleteProject={() => undefined}
    />
  ),
};

export const ProjectDetail: Story = {
  render: () => (
    <ProjectDetailPage
      project={sampleProjects[0]}
      onBack={() => undefined}
      onEditProject={() => undefined}
      onDeleteProject={() => undefined}
      onOpenWorktree={() => undefined}
    />
  ),
};

export const ProjectDetailLongPath: Story = {
  render: () => (
    <ProjectDetailPage
      project={{
        ...sampleProjects[0],
        id: "project-long-path",
        workingDirectory:
          "/Users/yoophi/project/worktrees/agentic-workbench/feature/storybook-long-path-layout-validation-with-many-segments",
        description:
          "긴 작업 디렉토리, remote URL, worktree 목록의 줄바꿈과 말줄임 처리를 검증합니다.",
      }}
      onBack={() => undefined}
      onEditProject={() => undefined}
      onDeleteProject={() => undefined}
      onOpenWorktree={() => undefined}
    />
  ),
};

export const WorktreeSession: Story = {
  render: () => (
    <ProjectWorktreeSessionPage
      project={sampleProjects[0]}
      worktree={sampleWorktrees[1]}
      onBack={() => undefined}
    />
  ),
};

export const WorktreeSessionClean: Story = {
  render: () => (
    <ProjectWorktreeSessionPage
      project={sampleProjects[0]}
      worktree={sampleWorktrees[0]}
      onBack={() => undefined}
    />
  ),
};
