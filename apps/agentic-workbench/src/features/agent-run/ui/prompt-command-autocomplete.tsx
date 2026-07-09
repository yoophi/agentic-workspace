import { useEffect, useRef } from "react";

import type {
  AgentToolCommandCandidate,
  AgentToolCommandCandidateStatus,
} from "@/entities/agent-run/model";
import { cn } from "@/lib/utils";

type PromptCommandAutocompleteProps = {
  open: boolean;
  status: AgentToolCommandCandidateStatus | "noMatch";
  candidates: AgentToolCommandCandidate[];
  highlightedIndex: number;
  onHighlight: (index: number) => void;
  onSelect: (candidate: AgentToolCommandCandidate) => void;
  className?: string;
};

function statusMessage(status: PromptCommandAutocompleteProps["status"]) {
  switch (status) {
    case "loading":
      return "Loading commands...";
    case "empty":
      return "No commands available";
    case "noMatch":
      return "No matching commands";
    case "error":
      return "Commands unavailable";
    default:
      return null;
  }
}

export function PromptCommandAutocomplete({
  open,
  status,
  candidates,
  highlightedIndex,
  onHighlight,
  onSelect,
  className,
}: PromptCommandAutocompleteProps) {
  const optionRefs = useRef(new Map<string, HTMLButtonElement>());
  const highlightedCandidate = candidates[highlightedIndex] ?? null;
  const highlightedOptionId = highlightedCandidate
    ? promptCommandOptionId(highlightedCandidate, highlightedIndex)
    : undefined;

  useEffect(() => {
    if (!open || status !== "ready" || !highlightedOptionId) {
      return;
    }

    optionRefs.current.get(highlightedOptionId)?.scrollIntoView({
      block: "nearest",
      inline: "nearest",
    });
  }, [highlightedOptionId, open, status]);

  if (!open) {
    return null;
  }

  const message = status === "ready" ? null : statusMessage(status);

  return (
    <div
      role="listbox"
      aria-label="Prompt command suggestions"
      aria-activedescendant={highlightedOptionId}
      className={cn(
        "absolute left-4 right-4 top-3 z-20 max-h-56 min-w-0 overflow-auto rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
        className,
      )}
    >
      {message ? (
        <div className="px-3 py-2 text-sm text-muted-foreground">{message}</div>
      ) : (
        candidates.map((candidate, index) => {
          const selected = index === highlightedIndex;
          const optionId = promptCommandOptionId(candidate, index);
          return (
            <button
              key={candidate.id}
              id={optionId}
              ref={(element) => {
                if (element) {
                  optionRefs.current.set(optionId, element);
                  return;
                }
                optionRefs.current.delete(optionId);
              }}
              type="button"
              role="option"
              aria-selected={selected}
              data-highlighted={selected ? "true" : undefined}
              className={cn(
                "flex w-full min-w-0 cursor-default items-start gap-2 rounded-sm px-3 py-2 text-left text-sm outline-none",
                selected && "bg-muted text-foreground",
              )}
              onMouseEnter={() => onHighlight(index)}
              onPointerDown={(event) => {
                event.preventDefault();
                onHighlight(index);
                onSelect(candidate);
              }}
              onClick={(event) => {
                if (event.detail === 0) {
                  onSelect(candidate);
                }
              }}
            >
              <span className="min-w-0 flex-1">
                <span className="block break-all font-mono font-medium">
                  {candidate.name}
                </span>
                {candidate.description && (
                  <span className="block break-all text-xs text-muted-foreground">
                    {candidate.description}
                  </span>
                )}
              </span>
              <span className="max-w-[35%] shrink-0 truncate rounded border px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-muted-foreground">
                {candidate.source}
              </span>
            </button>
          );
        })
      )}
    </div>
  );
}

function promptCommandOptionId(candidate: AgentToolCommandCandidate, index: number) {
  return `prompt-command-option-${index}-${candidate.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
}
