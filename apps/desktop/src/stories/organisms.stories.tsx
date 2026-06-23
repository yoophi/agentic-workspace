import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import type { Project } from "@/entities/project/model/types";
import { AgentRunPanel } from "@/features/agent-run/ui/agent-run-panel";
import { DeleteProjectDialog } from "@/features/project-delete/ui/delete-project-dialog";
import { ProjectFormDialog } from "@/features/project-form/ui/project-form-dialog";
import { ProjectTable } from "@/features/project-list/ui/project-table";
import { GitReferenceCombobox } from "@/features/project-worktree/ui/git-reference-combobox";
import { ProjectWorktreeCard } from "@/features/project-worktree/ui/project-worktree-card";
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
