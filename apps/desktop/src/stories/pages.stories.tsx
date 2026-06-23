import type { Meta, StoryObj } from "@storybook/react-vite";

import { ProjectDetailPage } from "@/pages/project-detail/ui/project-detail-page";
import { ProjectListPage } from "@/pages/project-list/ui/project-list-page";
import { ProjectWorktreeSessionPage } from "@/pages/project-worktree-session/ui/project-worktree-session-page";
import { sampleProjects, sampleWorktrees } from "@/shared/storybook/sample-data";

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

export const WorktreeSession: Story = {
  render: () => (
    <ProjectWorktreeSessionPage
      project={sampleProjects[0]}
      worktree={sampleWorktrees[1]}
      onBack={() => undefined}
    />
  ),
};
