import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import type { AgentToolCommandCandidate } from "@/entities/agent-run/model";

import { PromptCommandAutocomplete } from "./prompt-command-autocomplete";

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
    expect(html).toContain("aria-selected=\"true\"");
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
    }
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
