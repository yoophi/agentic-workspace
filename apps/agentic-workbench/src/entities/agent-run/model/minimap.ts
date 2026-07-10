import type { TimelineItem } from "./types";

export const MINIMAP_SUMMARY_MAX_LENGTH = 96;
const MINIMAP_CONTENT_WEIGHT_UNIT = 80;
const MINIMAP_CONTENT_WEIGHT_MAX = 12;

export type MinimapEntryRole = "user" | "assistant";

export type MinimapEntry = {
  id: string;
  runId: string;
  role: MinimapEntryRole;
  summary: string;
  contentWeight: number;
  sourceOrder: number;
};

function normalizeSummary(body: string) {
  const normalized = body.replace(/\s+/g, " ").trim();
  if (normalized.length <= MINIMAP_SUMMARY_MAX_LENGTH) {
    return normalized;
  }
  return `${normalized.slice(0, MINIMAP_SUMMARY_MAX_LENGTH - 3).trimEnd()}...`;
}

function contentWeight(body: string) {
  const normalizedLength = body.replace(/\s+/g, " ").trim().length;
  return Math.min(
    MINIMAP_CONTENT_WEIGHT_MAX,
    Math.max(1, Math.ceil(normalizedLength / MINIMAP_CONTENT_WEIGHT_UNIT)),
  );
}

export function projectTimelineToMinimapEntries(items: TimelineItem[]): MinimapEntry[] {
  return items.flatMap((item, sourceOrder) => {
    if (item.group !== "user/message" && item.group !== "assistant/message") {
      return [];
    }
    return [
      {
        id: item.id,
        runId: item.runId,
        role: item.group === "user/message" ? "user" : "assistant",
        summary: normalizeSummary(item.body),
        contentWeight: contentWeight(item.body),
        sourceOrder,
      } satisfies MinimapEntry,
    ];
  });
}
