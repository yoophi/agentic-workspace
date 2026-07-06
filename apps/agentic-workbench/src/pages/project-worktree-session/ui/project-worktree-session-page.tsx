import { useEffect, useState } from "react";
import {
  Group as ResizablePanelGroup,
  Panel as ResizablePanel,
  Separator as ResizableHandle,
} from "react-resizable-panels";

import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { Project } from "@/entities/project/model/types";
import type { AgentPromptRequest } from "@/features/agent-run/ui/agent-run-panel";
import { WorktreeAgentRunArea } from "@/features/agent-run/ui/worktree-agent-run-area";
import { WorktreeWorkspacePanel } from "@/features/worktree-workspace/ui/worktree-workspace-panel";
import { measureSessionMilestone } from "@/shared/lib/session-perf";

type ProjectWorktreeSessionPageProps = {
  project: Project;
  worktree: GitWorktree;
  onBack?: () => void;
  onOpenSettings?: () => void;
};

export function ProjectWorktreeSessionPage({
  worktree,
  onOpenSettings,
}: ProjectWorktreeSessionPageProps) {
  const [workspacePromptRequest, setWorkspacePromptRequest] =
    useState<AgentPromptRequest | null>(null);

  useEffect(() => {
    measureSessionMilestone("session:shell-rendered");
  }, []);

  return (
    <div className="flex h-[calc(100svh-3rem)] min-h-0 flex-col gap-4 overflow-hidden">
      <ResizablePanelGroup orientation="horizontal" className="min-h-0 flex-1">
        <ResizablePanel id="project-worktree-session-agent" defaultSize="40%" minSize="360px">
          <div className="h-full min-h-0">
            <WorktreeAgentRunArea
              worktree={worktree}
              externalPromptRequest={workspacePromptRequest}
              onOpenSettings={onOpenSettings}
            />
          </div>
        </ResizablePanel>

        <ResizableHandle
          aria-label="Workspace 영역 크기 조정"
          className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <div className="relative z-10 h-12 w-1 rounded-full bg-border transition-colors" />
        </ResizableHandle>

        <ResizablePanel id="project-worktree-session-workspace" minSize="480px">
          <WorktreeWorkspacePanel
            worktree={worktree}
            onSendAnnotationPrompt={(text) =>
              setWorkspacePromptRequest({ id: crypto.randomUUID(), text })
            }
          />
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
