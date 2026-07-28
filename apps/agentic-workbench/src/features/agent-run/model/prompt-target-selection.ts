import type { AgentRunPanelSlot } from "@/features/agent-run/model/agent-run-panel-slots";
import type { PromptTargetMode } from "@/entities/agent-orchestration";

export type PromptTargetSelection = {
  mode: PromptTargetMode;
  focusedPanelId: string;
  selectedPanelIds: string[];
};

export type PromptTargetSelectionResult =
  | { valid: true; panelIds: string[]; delegate: boolean }
  | { valid: false; panelIds: []; delegate: boolean; reason: "emptySelection" | "noRunnableTarget" };

export function selectPromptTargets(
  slots: AgentRunPanelSlot[],
  selection: PromptTargetSelection,
): PromptTargetSelectionResult {
  if (selection.mode === "coordinator") {
    const main = slots.find((slot) => slot.kind === "main" && slot.closeState === "open");
    return main
      ? { valid: true, panelIds: [main.id], delegate: true }
      : { valid: false, panelIds: [], delegate: true, reason: "noRunnableTarget" };
  }
  const runnable = slots.filter((slot) => slot.closeState === "open");
  const requested =
    selection.mode === "focused"
      ? [selection.focusedPanelId]
      : selection.mode === "all"
        ? runnable.map((slot) => slot.id)
        : selection.selectedPanelIds;
  if (selection.mode === "selected" && requested.length === 0) {
    return { valid: false, panelIds: [], delegate: false, reason: "emptySelection" };
  }
  const panelIds = [...new Set(requested)].filter((panelId) =>
    runnable.some((slot) => slot.id === panelId),
  );
  return panelIds.length
    ? { valid: true, panelIds, delegate: false }
    : { valid: false, panelIds: [], delegate: false, reason: "noRunnableTarget" };
}

