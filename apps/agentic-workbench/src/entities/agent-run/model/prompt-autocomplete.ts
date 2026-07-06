import type { AgentToolCommandCandidate } from "./types";

export type PromptAutocompleteTrigger = {
  prefix: "$" | "/";
  query: string;
  start: number;
  end: number;
};

export type PromptDraftSelection = {
  text: string;
  cursorStart: number;
  cursorEnd: number;
};

export type AvailableCommandCandidateScope = {
  runId?: string | null;
  agentId?: string | null;
  workingDirectory?: string | null;
};

const triggerPrefixes = new Set(["$", "/"]);

function isTokenBoundary(char: string | undefined) {
  return !char || /\s/.test(char);
}

export function findPromptAutocompleteTrigger(
  text: string,
  cursorStart: number,
  cursorEnd = cursorStart,
): PromptAutocompleteTrigger | null {
  if (cursorStart !== cursorEnd || cursorStart < 0 || cursorStart > text.length) {
    return null;
  }

  let start = cursorStart;
  while (start > 0 && !isTokenBoundary(text[start - 1])) {
    start -= 1;
  }

  const prefix = text[start];
  if (!triggerPrefixes.has(prefix)) {
    return null;
  }

  return {
    prefix: prefix as "$" | "/",
    query: text.slice(start + 1, cursorStart),
    start,
    end: cursorStart,
  };
}

export function normalizeToolCommandCandidates(
  candidates: AgentToolCommandCandidate[],
): AgentToolCommandCandidate[] {
  const seen = new Set<string>();
  return candidates
    .map((candidate) => ({
      ...candidate,
      id: candidate.id.trim(),
      name: candidate.name.trim(),
      insertText: candidate.insertText.trim(),
      description: candidate.description?.trim() || null,
    }))
    .filter((candidate) => candidate.name.length > 0 && candidate.insertText.length > 0)
    .filter((candidate) => {
      const key = candidate.id || `${candidate.source}:${candidate.name}:${candidate.insertText}`;
      if (seen.has(key)) {
        return false;
      }
      seen.add(key);
      return true;
    });
}

export function availableCommandCandidatesFromSessionUpdate(
  updatePayload: unknown,
  scope: AvailableCommandCandidateScope,
): AgentToolCommandCandidate[] {
  const update = unwrapSessionUpdate(updatePayload);
  if (!update || readString(update.sessionUpdate) !== "available_commands_update") {
    return [];
  }

  const availableCommands = Array.isArray(update.availableCommands)
    ? update.availableCommands
    : [];

  return normalizeToolCommandCandidates(
    availableCommands.flatMap((rawCommand, index) => {
      if (!rawCommand || typeof rawCommand !== "object") {
        return [];
      }

      const command = rawCommand as Record<string, unknown>;
      const name = readString(command.name)?.trim() ?? "";
      if (!name) {
        return [];
      }

      const description = readString(command.description)?.trim() || null;
      const source = name.startsWith("$") ? "extension" : "appCommand";

      return [
        {
          id: `available-command:${name}:${index}`,
          name,
          description,
          insertText: name,
          source,
          scope,
        },
      ];
    }),
  );
}

export function isAvailableCommandsSessionUpdate(updatePayload: unknown) {
  const update = unwrapSessionUpdate(updatePayload);
  return readString(update?.sessionUpdate) === "available_commands_update";
}

export function filterToolCommandCandidates(
  candidates: AgentToolCommandCandidate[],
  query: string,
) {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const normalized = normalizeToolCommandCandidates(candidates);
  if (!normalizedQuery) {
    return normalized;
  }
  return normalized.filter((candidate) => {
    const haystack = [
      candidate.name,
      candidate.insertText,
      candidate.description ?? "",
      candidate.source,
    ]
      .join(" ")
      .toLocaleLowerCase();
    return haystack.includes(normalizedQuery);
  });
}

export function clampHighlightedIndex(index: number, candidateCount: number) {
  if (candidateCount <= 0) {
    return -1;
  }
  if (index < 0) {
    return candidateCount - 1;
  }
  if (index >= candidateCount) {
    return 0;
  }
  return index;
}

export function insertionTextForPrefix(
  candidate: AgentToolCommandCandidate,
  prefix: "$" | "/",
) {
  const trimmed = candidate.insertText.trim();
  if (trimmed.startsWith("$") || trimmed.startsWith("/")) {
    return `${prefix}${trimmed.slice(1)}`;
  }
  return `${prefix}${trimmed}`;
}

export function replacePromptAutocompleteTrigger(
  draft: PromptDraftSelection,
  trigger: PromptAutocompleteTrigger,
  candidate: AgentToolCommandCandidate,
) {
  const insertText = insertionTextForPrefix(candidate, trigger.prefix);
  const nextText = `${draft.text.slice(0, trigger.start)}${insertText}${draft.text.slice(
    trigger.end,
  )}`;
  const nextCursor = trigger.start + insertText.length;
  return {
    text: nextText,
    cursorStart: nextCursor,
    cursorEnd: nextCursor,
  };
}

function unwrapSessionUpdate(payload: unknown): Record<string, unknown> | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }

  const value = payload as Record<string, unknown>;
  if (value.update && typeof value.update === "object") {
    return value.update as Record<string, unknown>;
  }
  return value;
}

function readString(value: unknown) {
  return typeof value === "string" ? value : null;
}
