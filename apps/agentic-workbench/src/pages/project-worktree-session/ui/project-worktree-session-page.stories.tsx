import type { Meta, StoryObj } from "@storybook/react-vite";

import { ProjectWorktreeSessionPage } from "./project-worktree-session-page";

const meta = {
  title: "Pages/Project Worktree/Orchestrated Session",
  component: ProjectWorktreeSessionPage,
  args: {
    project: {
      id: "project-story",
      name: "Agentic Workspace",
      workingDirectory: "/workspace/agentic-workspace",
      description: "멀티 에이전트 협업 개발 환경",
    },
    worktree: {
      path: "/workspace/agentic-workspace",
      branch: "codex/agent-orchestration",
      head: "0123456",
      status: "clean",
      canDelete: true,
    },
  },
} satisfies Meta<typeof ProjectWorktreeSessionPage>;

export default meta;
type Story = StoryObj<typeof meta>;
export const TabAndTileWorkspace: Story = {};
