import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import type { Project } from "@/entities/project/model/types";
import { AgentRunPanel } from "@/features/agent-run/ui/agent-run-panel";
import { DeleteProjectDialog } from "@/features/project-delete/ui/delete-project-dialog";
import { ProjectFormDialog } from "@/features/project-form/ui/project-form-dialog";
import { ProjectTable } from "@/features/project-list/ui/project-table";
import { GitReferenceCombobox } from "@/features/project-worktree/ui/git-reference-combobox";
import { ProjectWorktreeCard } from "@/features/project-worktree/ui/project-worktree-card";
import { SavedPromptToolbar } from "@/features/saved-prompt/ui/saved-prompt-toolbar";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { sampleProjects } from "@/shared/storybook/sample-data";

const meta = {
  title: "Atomic Design/Organisms/Registered Components",
  parameters: {
    docs: {
      description: {
        component: "Feature-level components that compose multiple molecules and own user workflows.",
      },
    },
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const ProjectManagement: Story = {
  render: () => {
    const [editingProject, setEditingProject] = useState<Project | null | undefined>(sampleProjects[0]);
    const [deletingProject, setDeletingProject] = useState<Project | null>(null);

    return (
      <div className="grid gap-6">
        <ProjectTable
          projects={sampleProjects}
          onSelectProject={() => undefined}
          onEditProject={setEditingProject}
          onDeleteProject={setDeletingProject}
        />
        <div className="flex gap-2">
          <Button onClick={() => setEditingProject(null)}>Open create form</Button>
          <Button variant="destructive" onClick={() => setDeletingProject(sampleProjects[0])}>
            Open delete dialog
          </Button>
        </div>
        <ProjectFormDialog
          project={editingProject ?? null}
          open={editingProject !== undefined}
          error={null}
          onOpenChange={() => setEditingProject(undefined)}
          onSubmit={async () => undefined}
          onError={() => undefined}
        />
        <DeleteProjectDialog
          project={deletingProject}
          onOpenChange={(open) => {
            if (!open) {
              setDeletingProject(null);
            }
          }}
          onConfirm={async () => setDeletingProject(null)}
        />
      </div>
    );
  },
};

export const WorktreeManagement: Story = {
  render: () => {
    const [reference, setReference] = useState("main");

    return (
      <div className="grid gap-6">
        <Card>
          <CardHeader>
            <CardTitle>Git reference combobox</CardTitle>
            <CardDescription>Branch query data is supplied by Storybook Tauri mocks.</CardDescription>
          </CardHeader>
          <CardContent>
            <GitReferenceCombobox
              workingDirectory={sampleProjects[0].workingDirectory}
              value={reference}
              onValueChange={setReference}
            />
          </CardContent>
        </Card>
        <ProjectWorktreeCard
          workingDirectory={sampleProjects[0].workingDirectory}
          onOpenWorktree={() => undefined}
        />
      </div>
    );
  },
};

export const AgentRun: Story = {
  render: () => (
    <AgentRunPanel workingDirectory="/Users/yoophi/project/acp-minimal-app" />
  ),
};

function SavedPromptToolbarStory({ disabled = false }: { disabled?: boolean }) {
  const [sentPrompt, setSentPrompt] = useState<string | null>(null);

  return (
    <Card className="max-w-3xl">
      <CardHeader>
        <CardTitle>Saved prompt toolbar</CardTitle>
        <CardDescription>
          Saved prompt query data and mutations are supplied by Storybook Tauri mocks.
        </CardDescription>
      </CardHeader>
      <CardContent className="grid gap-3 p-0">
        <SavedPromptToolbar disabled={disabled} onSendPrompt={setSentPrompt} />
        <div className="px-4 pb-4 text-xs text-muted-foreground">
          Last sent prompt: {sentPrompt ?? "None"}
        </div>
      </CardContent>
    </Card>
  );
}

export const SavedPrompts: Story = {
  render: () => <SavedPromptToolbarStory />,
};

export const SavedPromptsDisabled: Story = {
  render: () => <SavedPromptToolbarStory disabled />,
};

export const AgentRunResizablePrompt: Story = {
  parameters: {
    docs: {
      description: {
        story: "Shows the workspace in a constrained viewport so the bottom prompt panel resize handle and textarea resizing can be inspected.",
      },
    },
  },
  render: () => (
    <div className="mx-auto h-[720px] max-w-5xl">
      <AgentRunPanel
        workingDirectory="/Users/yoophi/project/worktrees/acp-minimal-app/resizable-prompt-story"
        scrollHeader={
          <Card>
            <CardHeader>
              <CardTitle>Resizable prompt workspace</CardTitle>
              <CardDescription>
                Drag the small handle above the prompt area to adjust the bottom panel height.
              </CardDescription>
            </CardHeader>
          </Card>
        }
      />
    </div>
  ),
};

export const AgentRunNarrowLongPath: Story = {
  parameters: {
    viewport: {
      defaultViewport: "mobile1",
    },
  },
  render: () => (
    <div className="mx-auto h-[720px] max-w-sm">
      <AgentRunPanel
        workingDirectory="/Users/yoophi/project/worktrees/acp-minimal-app/feature/really-long-worktree-name-for-layout-validation"
      />
    </div>
  ),
};
