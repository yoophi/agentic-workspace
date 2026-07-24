export const workspacePanelIds = ["git", "files", "markdown", "speckit"] as const;

export type WorkspacePanelId = (typeof workspacePanelIds)[number];
export type WorkspacePanelWidths = Partial<Record<WorkspacePanelId, number>>;

export function toggleWorkspacePanel(
  current: WorkspacePanelId | null,
  next: WorkspacePanelId,
): WorkspacePanelId | null {
  return current === next ? null : next;
}

export function normalizePanelWidth(value: number | null | undefined): number | undefined {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? Math.round(value)
    : undefined;
}

export function clampPanelWidth({
  preferredWidth,
  containerWidth,
  minimumA,
  minimumB,
}: {
  preferredWidth: number | undefined;
  containerWidth: number | undefined;
  minimumA: number;
  minimumB: number;
}): number | undefined {
  const normalized = normalizePanelWidth(preferredWidth);
  if (!normalized || !containerWidth || containerWidth <= 0) return normalized;
  return Math.max(minimumB, Math.min(normalized, Math.max(minimumB, containerWidth - minimumA)));
}
