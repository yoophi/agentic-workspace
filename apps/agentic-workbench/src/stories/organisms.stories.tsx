import type { Meta, StoryObj } from "@storybook/react-vite";
import { DiffViewer, HistoryGraphView, refsByTarget } from "@yoophi/git-ui";
import type { GitCommitGraph } from "@yoophi/git-graph";
import { useState } from "react";

import type { Project } from "@/entities/project/model/types";
import {
  AgentRunMermaidDiagram,
  StreamingMarkdown,
} from "@/features/agent-run/ui/agent-run-markdown";
import { PromptCommandAutocomplete } from "@/features/agent-run/ui/prompt-command-autocomplete";
import type { AgentToolCommandCandidate } from "@/entities/agent-run/model";
import { AgentCommandOverrideEditor } from "@/features/agent-command-override/ui/agent-command-override-editor";
import {
  createCommandOverrideDraft,
  type CommandOverrideDraft,
} from "@/features/agent-command-override/model/command-override-form";
import { AgentRunPanel } from "@/features/agent-run/ui/agent-run-panel";
import { DeleteProjectDialog } from "@/features/project-delete/ui/delete-project-dialog";
import { ProjectFormDialog } from "@/features/project-form/ui/project-form-dialog";
import { ProjectTable } from "@/features/project-list/ui/project-table";
import { GitReferenceCombobox } from "@/features/project-worktree/ui/git-reference-combobox";
import { ProjectWorktreeCard } from "@/features/project-worktree/ui/project-worktree-card";
import { SavedPromptToolbar } from "@/features/saved-prompt/ui/saved-prompt-toolbar";
import { WorktreeChangesPanel as WorktreeChangeReviewPanel } from "@/features/worktree-change-review/ui/worktree-changes-panel";
import { WorktreeChangesPanel as AgentRunWorktreeChangesPanel } from "@/features/worktree-changes/ui/worktree-changes-panel";
import { WorktreeWorkspacePanel } from "@/features/worktree-workspace/ui/worktree-workspace-panel";
import {
  computeGitGraphRows,
  getMaxGraphLane,
} from "@/features/worktree-workspace/model/git-graph-layout";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  sampleAgentRunToolFileChanges,
  sampleProjects,
  sampleWorktrees,
} from "@/shared/storybook/sample-data";

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

export const WorktreeChangeReview: Story = {
  render: () => (
    <WorktreeChangeReviewPanel workingDirectory="/Users/yoophi/project/worktrees/agentic-workbench/storybook" />
  ),
};

export const WorktreeWorkspace: Story = {
  render: () => (
    <div className="h-[720px] overflow-hidden border">
      <WorktreeWorkspacePanel worktree={sampleWorktrees[1]} />
    </div>
  ),
};

export const AgentCommandOverrideSettings: Story = {
  render: () => {
    const agents = [
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
    const [draft, setDraft] = useState<CommandOverrideDraft>(() =>
      createCommandOverrideDraft({
        globalCommand:
          "npx -y @agentclientprotocol/codex-acp --with-a-very-long-extra-argument-for-layout-validation",
        globalEnv: { HTTPS_PROXY: "http://127.0.0.1:8888" },
        profiles: [
          {
            id: "codex",
            name: "Codex",
            agentType: "codex",
            command: "custom-codex-acp",
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
      <div className="max-w-5xl">
        <AgentCommandOverrideEditor
          agents={agents}
          draft={draft}
          onDraftChange={setDraft}
          onSave={() => undefined}
        />
      </div>
    );
  },
};

// 마지막 활성 기본 프로필 disable 차단(specs/008 US3)과 env 편집기 상태를 보여주는 스토리.
export const AgentProfileEditorLastActiveGuard: Story = {
  render: function AgentProfileEditorLastActiveGuardStory() {
    const agents = [
      { id: "codex", label: "Codex", command: "npx -y @agentclientprotocol/codex-acp" },
      {
        id: "claude-code",
        label: "Claude Code",
        command: "npx -y @agentclientprotocol/claude-agent-acp",
      },
    ];
    const [draft, setDraft] = useState<CommandOverrideDraft>(() => {
      const base = createCommandOverrideDraft({});
      return {
        ...base,
        profiles: base.profiles.map((profile) =>
          profile.id === "codex" ? profile : { ...profile, enabled: false },
        ),
      };
    });

    return (
      <div className="max-w-5xl">
        <AgentCommandOverrideEditor
          agents={agents}
          draft={draft}
          onDraftChange={setDraft}
          onSave={() => undefined}
        />
      </div>
    );
  },
};

export const AgentRunChangedFiles: Story = {
  parameters: {
    docs: {
      description: {
        story:
          "Shows the agent-run changed-files panel with modified, added, and binary file states. Storybook Tauri mocks supply list_worktree_changes.",
      },
    },
  },
  render: () => (
    <div className="max-w-5xl">
      <AgentRunWorktreeChangesPanel
        workingDirectory="/Users/yoophi/project/worktrees/agentic-workbench/storybook"
        isRunning={false}
      />
    </div>
  ),
};

export const AgentRunToolFileChanges: Story = {
  parameters: {
    docs: {
      description: {
        story:
          "Shows tool-use file changes with modified, added, binary, truncated, and long-path states.",
      },
    },
  },
  render: () => (
    <div className="max-w-5xl space-y-3">
      {sampleAgentRunToolFileChanges.map((change) => (
        <Card key={`${change.kind}:${change.path}`}>
          <CardHeader>
            <CardTitle className="text-sm">{change.path}</CardTitle>
            <CardDescription>
              {change.kind} · {change.status}
              {change.truncated ? " · truncated" : ""}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {change.diff ? (
              <DiffViewer content={change.diff} />
            ) : change.content ? (
              <pre className="max-h-72 overflow-auto rounded-md border bg-muted/40 p-3 whitespace-pre-wrap break-words font-mono text-xs">
                {change.content}
              </pre>
            ) : (
              <div className="rounded-md border border-dashed bg-muted/40 p-3 text-xs text-muted-foreground">
                {change.message ?? "No text diff available."}
              </div>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  ),
};

export const AgentRun: Story = {
  render: () => (
    <AgentRunPanel workingDirectory="/Users/yoophi/project/agentic-workbench" />
  ),
};

export const AgentRunQueuedFirstPrompt: Story = {
  parameters: {
    docs: {
      description: {
        story:
          "Starts the Storybook mock run from an external first prompt so the queued prompt state appears before the mock lifecycle response.",
      },
    },
  },
  render: () => (
    <div className="mx-auto h-[720px] max-w-5xl">
      <AgentRunPanel
        workingDirectory="/Users/yoophi/project/agentic-workbench"
        externalPromptRequest={{
          id: "storybook-first-run-queued-prompt",
          text: "첫 실행 프롬프트를 queue 상태로 먼저 표시해 주세요.",
        }}
      />
    </div>
  ),
};

export const AgentRunMermaidOutput: Story = {
  render: () => (
    <div className="max-w-3xl rounded-lg border p-4">
      <StreamingMarkdown
        content={[
          "```mermaid",
          "flowchart TD",
          "  A[Agent starts] --> B[Plan]",
          "  B --> C[Implement]",
          "```",
          "",
          "```ts",
          "const ordinaryCode = true;",
          "```",
        ].join("\n")}
      />
    </div>
  ),
};

export const AgentRunMermaidFallback: Story = {
  render: () => (
    <div className="max-w-3xl rounded-lg border p-4">
      <StreamingMarkdown
        content={[
          "```mermaid",
          "flowchart TD",
          "  A -->",
          "```",
          "",
          "Following text remains readable.",
        ].join("\n")}
      />
    </div>
  ),
};

export const AgentRunLargeMermaidOutput: Story = {
  render: () => (
    <div className="max-w-sm rounded-lg border p-4">
      <StreamingMarkdown
        content={[
          "```mermaid",
          "flowchart LR",
          "  A[Start] --> B[Collect repository context]",
          "  B --> C[Analyze agent run output]",
          "  C --> D[Render a very wide Mermaid diagram inside the panel]",
          "  D --> E[Keep surrounding timeline layout stable]",
          "  E --> F[Finish]",
          "```",
        ].join("\n")}
      />
    </div>
  ),
};

export const AgentRunMermaidExpandedModal: Story = {
  render: () => (
    <div className="max-w-sm rounded-lg border p-4">
      <AgentRunMermaidDiagram
        blockId="storybook-expanded-mermaid"
        defaultExpanded
        source={[
          "flowchart LR",
          "  A[Start] --> B[Collect repository context]",
          "  B --> C[Analyze agent run output]",
          "  C --> D[Open full-screen modal]",
          "  D --> E[Inspect a wider diagram]",
          "  E --> F[Close and return]",
        ].join("\n")}
      />
    </div>
  ),
};

export const AgentRunStreamingMermaidOutput: Story = {
  render: () => (
    <div className="max-w-3xl rounded-lg border p-4">
      <StreamingMarkdown
        content={[
          "Preparing diagram",
          "",
          "```mermaid",
          "flowchart TD",
          "  A[Partial stream] --> B[Still arriving]",
        ].join("\n")}
      />
    </div>
  ),
};

export const AgentRunRalphLoop: Story = {
  render: () => (
    <AgentRunPanel
      workingDirectory="/Users/yoophi/project/agentic-workbench"
      initialInputMode="ralphLoop"
    />
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

const promptCommandCandidates: AgentToolCommandCandidate[] = [
  {
    id: "session:set_window_title",
    name: "set_window_title",
    description: "Change the current Worktree Session window title.",
    insertText: "$set_window_title",
    source: "sessionTool",
    scope: { runId: "run-1", agentId: "codex", workingDirectory: "/repo" },
  },
  {
    id: "app:goal",
    name: "goal",
    description: "Manage the current long-running goal.",
    insertText: "/goal",
    source: "appCommand",
    scope: { agentId: "codex", workingDirectory: "/repo" },
  },
  {
    id: "session:very_long_tool_name",
    name: "very_long_tool_name_that_must_not_resize_the_prompt_panel",
    description:
      "A long description that should remain contained inside the suggestion row without overlapping prompt controls.",
    insertText: "$very_long_tool_name_that_must_not_resize_the_prompt_panel",
    source: "sessionTool",
    scope: { runId: "run-1", agentId: "codex", workingDirectory: "/repo" },
  },
];

export const PromptCommandAutocompleteStates: Story = {
  render: () => (
    <div className="grid gap-6">
      {[
        { title: "Loading", status: "loading" as const, candidates: [], highlightedIndex: -1 },
        {
          title: "Ready",
          status: "ready" as const,
          candidates: promptCommandCandidates.slice(0, 2),
          highlightedIndex: 0,
        },
        {
          title: "Many candidates",
          status: "ready" as const,
          candidates: promptCommandCandidates,
          highlightedIndex: 1,
        },
        { title: "No match", status: "noMatch" as const, candidates: [], highlightedIndex: -1 },
        { title: "Empty", status: "empty" as const, candidates: [], highlightedIndex: -1 },
        {
          title: "Long content",
          status: "ready" as const,
          candidates: [promptCommandCandidates[2]],
          highlightedIndex: 0,
        },
      ].map((state) => (
        <Card key={state.title}>
          <CardHeader>
            <CardTitle>{state.title}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="relative h-24 rounded-md border bg-background">
              <PromptCommandAutocomplete
                open
                status={state.status}
                candidates={state.candidates}
                highlightedIndex={state.highlightedIndex}
                onHighlight={() => undefined}
                onSelect={() => undefined}
              />
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  ),
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
        workingDirectory="/Users/yoophi/project/worktrees/agentic-workbench/resizable-prompt-story"
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
        workingDirectory="/Users/yoophi/project/worktrees/agentic-workbench/feature/really-long-worktree-name-for-layout-validation"
      />
    </div>
  ),
};


// 1,200개 commit을 로드해도 viewport 근처 row만 DOM에 존재하는지 확인하는
// virtualization 스토리(specs/007 US4, R11).
const largeGraph: GitCommitGraph = (() => {
  const commits = Array.from({ length: 1_200 }, (_, index) => ({
    hash: `commit-${index.toString().padStart(6, "0")}`,
    shortHash: `c${index.toString().padStart(6, "0")}`,
    parents: index === 1_199 ? [] : [`commit-${(index + 1).toString().padStart(6, "0")}`],
    message: `feat: incremental change #${1_200 - index}`,
    author: index % 3 === 0 ? "Yoophi" : "Agent",
    date: new Date(Date.UTC(2026, 5, 30) - index * 3_600_000).toISOString(),
    isHead: index === 0,
    isMerge: false,
  }));

  return {
    commits,
    refs: [{ name: "main", target: commits[0].hash, kind: "localBranch" as const }],
    page: { offset: 0, limit: 300, totalCount: 1_200, hasMore: false },
    layoutHints: { rowHeight: 32, maxInitialLanes: 10 },
  };
})();

export const VirtualizedHistoryGraphLargeRepo: Story = {
  render: () => {
    const graphRows = computeGitGraphRows(largeGraph.commits);

    return (
      <div className="h-[480px] overflow-auto rounded-md border bg-background p-2">
        <HistoryGraphView
          graph={largeGraph}
          graphRefs={refsByTarget(largeGraph.refs)}
          graphRows={graphRows}
          maxGraphLane={getMaxGraphLane(graphRows)}
          onSelectCommit={() => undefined}
          hasNextPage={false}
          isFetchingNextPage={false}
          onLoadMore={() => undefined}
        />
      </div>
    );
  },
};
