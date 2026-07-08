import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { AgentCommandOverrideEditor } from "@/features/agent-command-override/ui/agent-command-override-editor";
import {
  createCommandOverrideDraft,
  type CommandOverrideDraft,
} from "@/features/agent-command-override/model/command-override-form";
import { createPlaceholderWorktree } from "@/entities/project/model/git-worktree";
import { ProjectDetailPage } from "@/pages/project-detail/ui/project-detail-page";
import { ProjectDashboardPage } from "@/pages/project-dashboard/ui/project-dashboard-page";
import { ProjectListPage } from "@/pages/project-list/ui/project-list-page";
import { ProjectWorktreeSessionPage } from "@/pages/project-worktree-session/ui/project-worktree-session-page";
import { SettingsPageLayout } from "@/pages/settings/ui/settings-page";
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

const settingsAgents = [
  {
    id: "codex",
    label: "Codex",
    command: "npx -y @agentclientprotocol/codex-acp",
  },
  {
    id: "claude-code",
    label: "Claude Code",
    command: "npx -y @agentclientprotocol/claude-agent-acp",
  },
];

function SettingsPageStory({
  longContent = false,
  loadError = null,
}: {
  longContent?: boolean;
  loadError?: string | null;
}) {
  const [draft, setDraft] = useState<CommandOverrideDraft>(() =>
    createCommandOverrideDraft({
      globalCommand: longContent
        ? "npx -y @agentclientprotocol/codex-acp --profile storybook --with-a-long-command-fragment-for-window-layout-validation"
        : "npx -y @agentclientprotocol/codex-acp",
      globalEnv: longContent
        ? {
            HTTPS_PROXY: "http://127.0.0.1:8888",
            STORYBOOK_LONG_ENVIRONMENT_VARIABLE_NAME: "long-value-for-layout-validation",
          }
        : { HTTPS_PROXY: "http://127.0.0.1:8888" },
      profiles: [
        {
          id: "codex",
          name: longContent ? "Codex default profile with long display name" : "Codex",
          agentType: "codex",
          command: longContent
            ? "custom-codex-acp --with-long-flag --and-another-long-flag"
            : "custom-codex-acp",
          env: { OPENAI_API_KEY: "sk-..." },
          enabled: true,
          builtIn: true,
        },
        {
          id: "custom-claude-proxy",
          name: "Claude (프록시 경유)",
          agentType: "claude-code",
          command: null,
          env: { ANTHROPIC_BASE_URL: "http://127.0.0.1:8080" },
          enabled: true,
          builtIn: false,
        },
      ],
    }),
  );

  return (
    <div className="max-w-[920px] rounded-md border bg-muted/30 p-6">
      <SettingsPageLayout>
        <AgentCommandOverrideEditor
          agents={settingsAgents}
          draft={draft}
          loadError={loadError}
          onDraftChange={setDraft}
          onSave={() => undefined}
        />
      </SettingsPageLayout>
    </div>
  );
}

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

// worktree 목록 도착 전 placeholder 상태: status "확인 중" badge가 표시된다.
export const WorktreeSessionMetadataLoading: Story = {
  render: () => (
    <ProjectWorktreeSessionPage
      project={sampleProjects[0]}
      worktree={createPlaceholderWorktree(sampleWorktrees[0].path)}
      onBack={() => undefined}
    />
  ),
};

export const SettingsWindow: Story = {
  render: () => <SettingsPageStory />,
};

export const SettingsWindowLoading: Story = {
  render: () => (
    <div className="max-w-[920px] rounded-md border bg-muted/30 p-6">
      <SettingsPageLayout>
        <p className="rounded-md border bg-background px-3 py-2 text-sm text-muted-foreground">
          설정을 불러오는 중입니다.
        </p>
      </SettingsPageLayout>
    </div>
  ),
};

export const SettingsWindowError: Story = {
  render: () => <SettingsPageStory loadError="설정을 불러오지 못했습니다." />,
};

export const SettingsWindowLongContent: Story = {
  render: () => <SettingsPageStory longContent />,
};
