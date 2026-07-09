import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { AgentToolCommandCandidate } from "@/entities/agent-run/model";

import { PromptCommandAutocomplete } from "./prompt-command-autocomplete";

const PROMPT_COMMAND_AUTOCOMPLETE_SOURCE = readFileSync(
  new URL("./prompt-command-autocomplete.tsx", import.meta.url),
  "utf8",
);

const candidates: AgentToolCommandCandidate[] = [
  {
    id: "session:set_window_title",
    name: "set_window_title",
    description: "Change the current Worktree Session window title.",
    insertText: "$set_window_title",
    source: "sessionTool",
    scope: { runId: "run-1", agentId: "codex", workingDirectory: "/repo" },
  },
  {
    id: "app:goal",
    name: "goal",
    description: "Manage the current goal.",
    insertText: "/goal",
    source: "appCommand",
    scope: { agentId: "codex", workingDirectory: "/repo" },
  },
];

describe("PromptCommandAutocomplete", () => {
  it("renders ready candidates with labels and descriptions", () => {
    const html = renderToStaticMarkup(
      <PromptCommandAutocomplete
        open
        status="ready"
        candidates={candidates}
        highlightedIndex={0}
        onHighlight={() => undefined}
        onSelect={() => undefined}
      />,
    );

    expect(html).toContain("set_window_title");
    expect(html).toContain("Change the current Worktree Session window title.");
    expect(html).toContain("role=\"listbox\"");
    expect(html).toContain("role=\"option\"");
    expect(html).toContain("aria-selected=\"true\"");
    expect(html).toContain("aria-activedescendant=\"prompt-command-option-0-session-set_window_title\"");
    expect(html).toContain("data-highlighted=\"true\"");
  });

  it("keeps highlighted options addressable for nearest scroll visibility", () => {
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("useRef(new Map<string, HTMLButtonElement>())");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("scrollIntoView({");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("block: \"nearest\"");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("inline: \"nearest\"");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("data-highlighted={selected ? \"true\" : undefined}");
  });

  it("constrains long candidate text inside suggestion items", () => {
    const html = renderToStaticMarkup(
      <PromptCommandAutocomplete
        open
        status="ready"
        candidates={[
          {
            ...candidates[0],
            id: "long",
            name: "very_long_command_name_without_any_word_boundaries_to_force_wrapping",
            description:
              "very_long_description_without_any_word_boundaries_to_force_wrapping_inside_the_autocomplete_container",
          },
        ]}
        highlightedIndex={0}
        onHighlight={() => undefined}
        onSelect={() => undefined}
      />,
    );

    expect(html).toContain("min-w-0");
    expect(html).toContain("break-all");
    expect(html).toContain("max-w-[35%]");
  });

  it("renders loading, empty, no-match, and error fallback states", () => {
    for (const [status, text] of [
      ["loading", "Loading commands..."],
      ["empty", "No commands available"],
      ["noMatch", "No matching commands"],
      ["error", "Commands unavailable"],
    ] as const) {
      const html = renderToStaticMarkup(
        <PromptCommandAutocomplete
          open
          status={status}
          candidates={[]}
          highlightedIndex={-1}
          onHighlight={() => undefined}
          onSelect={() => undefined}
        />,
      );
      expect(html).toContain(text);
      expect(html).not.toContain("role=\"option\"");
    }
  });

  it("keeps pointer selection on the same single-candidate selection contract", () => {
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("onMouseEnter={() => onHighlight(index)}");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("onPointerDown={(event) =>");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).toContain("onSelect(candidate)");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).not.toContain("selectedCandidates");
    expect(PROMPT_COMMAND_AUTOCOMPLETE_SOURCE).not.toContain("CommandDialog");
  });

  it("does not render when closed", () => {
    const html = renderToStaticMarkup(
      <PromptCommandAutocomplete
        open={false}
        status="ready"
        candidates={candidates}
        highlightedIndex={0}
        onHighlight={() => undefined}
        onSelect={() => undefined}
      />,
    );

    expect(html).toBe("");
  });
});
