export const workspacePanelIds = ["git", "files", "markdown", "speckit"] as const;

export type WorkspacePanelId = (typeof workspacePanelIds)[number];
export type WorkspacePanelWidths = Partial<Record<WorkspacePanelId, number>>;

// Worktree 하나가 소유하는 레이아웃 레코드. 창 위치·크기는 전용 저장소가 소유하므로
// 이 타입에는 포함하지 않는다. (docs/window-state-recovery-attempts.md 참고)
export type WorktreeWorkspaceLayout = {
  workingDirectory: string;
  outerPanelWidthPx?: number;
  panelWidthsPx: WorkspacePanelWidths;
};

export function emptyWorktreeWorkspaceLayout(workingDirectory: string): WorktreeWorkspaceLayout {
  return { workingDirectory, panelWidthsPx: {} };
}

// 바깥 B 폭만 교체한다. 내부 B 폭은 그대로 유지해 서로 덮어쓰지 않는다.
export function withOuterPanelWidth(
  layout: WorktreeWorkspaceLayout,
  widthPx: number,
): WorktreeWorkspaceLayout {
  return { ...layout, outerPanelWidthPx: widthPx };
}

// 지정한 패널 종류의 내부 B 폭만 교체한다. 다른 패널 종류와 바깥 폭은 유지한다.
export function withPanelWidth(
  layout: WorktreeWorkspaceLayout,
  panel: WorkspacePanelId,
  widthPx: number,
): WorktreeWorkspaceLayout {
  return { ...layout, panelWidthsPx: { ...layout.panelWidthsPx, [panel]: widthPx } };
}
