import type { WorkspacePanelId, WorkspacePanelWidths } from "@/features/worktree-workspace/model/workspace-layout";

export type WorktreeWorkspaceLayout = {
  workingDirectory: string;
  outerPanelWidthPx?: number;
  panelWidthsPx: WorkspacePanelWidths;
};

export type { WorkspacePanelId };
