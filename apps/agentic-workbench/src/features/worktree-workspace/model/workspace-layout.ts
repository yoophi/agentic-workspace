import {
  workspacePanelIds,
  type WorkspacePanelId,
  type WorkspacePanelWidths,
} from "@/entities/worktree-workspace-layout/model/types";

export { workspacePanelIds };
export type { WorkspacePanelId, WorkspacePanelWidths };

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

// 저장된 선호 폭을 현재 컨테이너에서 표시할 수 있는 범위로만 좁힌다.
// 저장 값 자체는 바꾸지 않는다. (research.md 결정 3)
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

// 저장 여부 판단. 사용자가 직접 조절해 레이아웃이 안정된 경우에만 저장한다.
// 마운트 직후 값, 창 크기 변화로 제한된 값, 변화 없는 값은 저장하지 않으므로
// 좁은 화면을 한 번 열었다고 선호 폭이 사라지지 않는다. (research.md 결정 3, 결정 5)
export function shouldPersistPanelWidth({
  nextWidth,
  preferredWidth,
  hydrated,
  userInitiated,
}: {
  nextWidth: number | null | undefined;
  preferredWidth: number | undefined;
  hydrated: boolean;
  userInitiated: boolean;
}): boolean {
  if (!hydrated || !userInitiated) return false;
  const normalized = normalizePanelWidth(nextWidth);
  if (normalized === undefined) return false;
  return normalized !== normalizePanelWidth(preferredWidth);
}
