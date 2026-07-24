import { useEffect, useRef, useState } from "react";
import {
  Group as ResizablePanelGroup,
  Panel as ResizablePanel,
  Separator as ResizableHandle,
  usePanelRef,
} from "react-resizable-panels";

import type { GitWorktree } from "@/entities/project/model/git-worktree";
import type { Project } from "@/entities/project/model/types";
import type { AgentPromptRequest } from "@/features/agent-run/ui/agent-run-panel";
import { WorktreeAgentRunArea } from "@/features/agent-run/ui/worktree-agent-run-area";
import { WorktreeWorkspacePanel } from "@/features/worktree-workspace/ui/worktree-workspace-panel";
import { WorkspacePanelSelector } from "@/features/worktree-workspace/ui/workspace-panel-selector";
import { toggleWorkspacePanel, type WorkspacePanelId } from "@/features/worktree-workspace/model/workspace-layout";
import { getWorktreeWorkspaceLayout, saveWorktreeWorkspaceLayout } from "@/entities/worktree-workspace-layout/api/worktree-workspace-layout-repository";
import type { WorktreeWorkspaceLayout } from "@/entities/worktree-workspace-layout/model/types";
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
  const [selectedPanel, setSelectedPanel] = useState<WorkspacePanelId | null>("git");
  const [layout, setLayout] = useState<WorktreeWorkspaceLayout | null>(null);
  const layoutLoadedRef = useRef(false);
  const lastOuterWidthRef = useRef<number | undefined>(undefined);
  const workspacePanelRef = usePanelRef();

  useEffect(() => {
    measureSessionMilestone("session:shell-rendered");
  }, []);

  useEffect(() => {
    let cancelled = false;
    layoutLoadedRef.current = false;
    void getWorktreeWorkspaceLayout(worktree.path).then((saved) => {
      if (!cancelled) {
        const next = saved ?? { workingDirectory: worktree.path, panelWidthsPx: {} };
        lastOuterWidthRef.current = next.outerPanelWidthPx;
        setLayout(next);
        layoutLoadedRef.current = true;
      }
    }).catch((error) => console.error("Failed to load workspace layout", error));
    return () => { cancelled = true; };
  }, [worktree.path]);

  useEffect(() => {
    const width = layout?.outerPanelWidthPx;
    if (selectedPanel && width && workspacePanelRef.current) {
      workspacePanelRef.current.resize(width);
    }
  }, [layout?.outerPanelWidthPx, selectedPanel, workspacePanelRef]);

  const saveOuterWidth = (width: number) => {
    const normalizedWidth = Math.round(width);
    if (!layoutLoadedRef.current || !Number.isFinite(normalizedWidth) || normalizedWidth <= 0 || lastOuterWidthRef.current === normalizedWidth) return;
    lastOuterWidthRef.current = normalizedWidth;
    const next = { ...(layout ?? { workingDirectory: worktree.path, panelWidthsPx: {} }), outerPanelWidthPx: normalizedWidth };
    void saveWorktreeWorkspaceLayout(next).catch((error) => console.error("Failed to save workspace layout", error));
  };

  return (
    <div className="flex h-[calc(100svh-3rem)] min-h-0 flex-col gap-4 overflow-hidden">
      <div className="flex min-h-0 flex-1">
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

        {selectedPanel ? <ResizableHandle
          aria-label="Workspace 영역 크기 조정"
          className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        /> : null}

        {selectedPanel ? <ResizablePanel key={layout?.outerPanelWidthPx ?? "workspace-default"} id="project-worktree-session-workspace" minSize="480px"
          defaultSize={layout?.outerPanelWidthPx ? `${layout.outerPanelWidthPx}px` : undefined}
          panelRef={workspacePanelRef}
          groupResizeBehavior="preserve-pixel-size"
          onResize={(size) => saveOuterWidth(size.inPixels)}>
          <WorktreeWorkspacePanel
            worktree={worktree}
            selectedPanel={selectedPanel}
            onSendAnnotationPrompt={(text) =>
              setWorkspacePromptRequest({ id: crypto.randomUUID(), text })
            }
          />
        </ResizablePanel> : null}
      </ResizablePanelGroup>
      <WorkspacePanelSelector selectedPanel={selectedPanel} onSelect={(panel) => setSelectedPanel((current) => toggleWorkspacePanel(current, panel))} />
      </div>
    </div>
  );
}
