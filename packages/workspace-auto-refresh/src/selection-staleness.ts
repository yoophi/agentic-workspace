export type StaleSelectionKind = "file" | "commit" | "diff-file" | "markdown-document";

export type StaleSelectionReason =
  | "deleted"
  | "renamed"
  | "branch-changed"
  | "history-rewritten"
  | "unreadable"
  | "unknown";

export type StaleSelection = {
  kind: StaleSelectionKind;
  id: string;
  reason: StaleSelectionReason;
  detectedAt: number;
};

export type SelectionPresenceInput = {
  selectedId: string | null | undefined;
  availableIds: Iterable<string>;
};

function normalizeId(id: string) {
  return id.trim();
}

function nowOrValue(now?: number) {
  return now ?? Date.now();
}

export function isSelectionStillPresent({
  selectedId,
  availableIds,
}: SelectionPresenceInput) {
  const normalizedSelectedId = selectedId ? normalizeId(selectedId) : "";

  if (!normalizedSelectedId) {
    return true;
  }

  for (const availableId of availableIds) {
    if (normalizeId(availableId) === normalizedSelectedId) {
      return true;
    }
  }

  return false;
}

export function createStaleSelection({
  kind,
  id,
  reason,
  now,
}: {
  kind: StaleSelectionKind;
  id: string;
  reason: StaleSelectionReason;
  now?: number;
}): StaleSelection {
  return {
    kind,
    id: normalizeId(id),
    reason,
    detectedAt: nowOrValue(now),
  };
}

export function findStaleFileSelection({
  selectedPath,
  availablePaths,
  reason = "deleted",
  now,
}: {
  selectedPath: string | null | undefined;
  availablePaths: Iterable<string>;
  reason?: StaleSelectionReason;
  now?: number;
}) {
  if (isSelectionStillPresent({ selectedId: selectedPath, availableIds: availablePaths })) {
    return null;
  }

  return createStaleSelection({
    kind: "file",
    id: selectedPath ?? "",
    reason,
    now,
  });
}

export function findStaleCommitSelection({
  selectedCommitHash,
  availableCommitHashes,
  reason = "history-rewritten",
  now,
}: {
  selectedCommitHash: string | null | undefined;
  availableCommitHashes: Iterable<string>;
  reason?: StaleSelectionReason;
  now?: number;
}) {
  if (
    isSelectionStillPresent({
      selectedId: selectedCommitHash,
      availableIds: availableCommitHashes,
    })
  ) {
    return null;
  }

  return createStaleSelection({
    kind: "commit",
    id: selectedCommitHash ?? "",
    reason,
    now,
  });
}

export function findStaleMarkdownDocument({
  absolutePath,
  readable,
  now,
}: {
  absolutePath: string | null | undefined;
  readable: boolean;
  now?: number;
}) {
  if (!absolutePath || readable) {
    return null;
  }

  return createStaleSelection({
    kind: "markdown-document",
    id: absolutePath,
    reason: "unreadable",
    now,
  });
}
