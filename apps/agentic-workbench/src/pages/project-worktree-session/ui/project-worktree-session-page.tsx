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
import { WorkspacePanelSelector } from "@/features/worktree-workspace/ui/workspace-panel-selector";
import {
  toggleWorkspacePanel,
  type WorkspacePanelId,
} from "@/features/worktree-workspace/model/workspace-layout";
import { useSplitPersistence } from "@/features/worktree-workspace/model/use-split-persistence";
import { useWorktreeWorkspaceLayout } from "@/features/worktree-workspace/model/use-worktree-workspace-layout";
import { measureSessionMilestone } from "@/shared/lib/session-perf";

type ProjectWorktreeSessionPageProps = {
  project: Project;
  worktree: GitWorktree;
  onBack?: () => void;
  onOpenSettings?: () => void;
};

/// 바깥 분할의 최소 폭. A(에이전트)와 B(Workspace)가 모두 조작 가능한 범위를 보장한다.
const OUTER_MINIMUM_A_PX = 360;
const OUTER_MINIMUM_B_PX = 480;
const OUTER_FALLBACK_B_SIZE = "60%";

export function ProjectWorktreeSessionPage({
  worktree,
  onOpenSettings,
}: ProjectWorktreeSessionPageProps) {
  const [workspacePromptRequest, setWorkspacePromptRequest] =
    useState<AgentPromptRequest | null>(null);
  const [selectedPanel, setSelectedPanel] = useState<WorkspacePanelId | null>("git");
  const { layout, hydrated, persistOuterWidth, persistPanelWidth } = useWorktreeWorkspaceLayout(
    worktree.path,
  );

  useEffect(() => {
    measureSessionMilestone("session:shell-rendered");
  }, []);

  const outerSplit = useSplitPersistence({
    preferredWidth: layout?.outerPanelWidthPx,
    hydrated,
    onPersist: persistOuterWidth,
    minimumA: OUTER_MINIMUM_A_PX,
    minimumB: OUTER_MINIMUM_B_PX,
    fallbackSize: OUTER_FALLBACK_B_SIZE,
  });

  return (
    <div className="flex h-[calc(100svh-3rem)] min-h-0 flex-col gap-4 overflow-hidden">
      <div className="flex min-h-0 flex-1">
        <ResizablePanelGroup
          orientation="horizontal"
          className="min-h-0 flex-1"
          {...outerSplit.groupProps}
        >
          <ResizablePanel id="project-worktree-session-agent" minSize={`${OUTER_MINIMUM_A_PX}px`}>
            <div className="h-full min-h-0">
              <WorktreeAgentRunArea
                worktree={worktree}
                externalPromptRequest={workspacePromptRequest}
                onOpenSettings={onOpenSettings}
              />
            </div>
          </ResizablePanel>

          {/* 선택 없음 상태에서는 B와 분할 핸들을 렌더링하지 않아 A가 전체 폭을 사용한다. */}
          {selectedPanel ? (
            <ResizableHandle
              aria-label="Workspace 영역 크기 조정"
              className="relative flex w-2 shrink-0 cursor-ew-resize items-center justify-center bg-transparent transition-colors after:absolute after:bottom-0 after:top-0 after:w-px after:bg-border hover:after:bg-muted-foreground/60 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              {...outerSplit.separatorProps}
            />
          ) : null}

          {/* 저장된 레이아웃을 먼저 읽은 뒤 B를 마운트해, 기본 폭으로 한 번 그렸다가
              다시 그리는 재마운트를 만들지 않는다. (research.md 결정 5) */}
          {selectedPanel && hydrated ? (
            <ResizablePanel id="project-worktree-session-workspace" {...outerSplit.panelProps}>
              <WorktreeWorkspacePanel
                worktree={worktree}
                selectedPanel={selectedPanel}
                panelWidthsPx={layout?.panelWidthsPx}
                layoutHydrated={hydrated}
                onPersistPanelWidth={persistPanelWidth}
                onSendAnnotationPrompt={(text) =>
                  setWorkspacePromptRequest({ id: crypto.randomUUID(), text })
                }
                onSendSddPrompt={(request) =>
                  setWorkspacePromptRequest({
                    id: crypto.randomUUID(),
                    text: request.text,
                    delivery: request.delivery,
                  })
                }
              />
            </ResizablePanel>
          ) : null}
        </ResizablePanelGroup>
        <WorkspacePanelSelector
          selectedPanel={selectedPanel}
          onSelect={(panel) => setSelectedPanel((current) => toggleWorkspacePanel(current, panel))}
        />
      </div>
    </div>
  );
}
