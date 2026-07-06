import { describe, expect, it } from "vitest";

import {
  clampHighlightedIndex,
  filterToolCommandCandidates,
  findPromptAutocompleteTrigger,
  insertionTextForPrefix,
  normalizeToolCommandCandidates,
  replacePromptAutocompleteTrigger,
} from "./prompt-autocomplete";
import type { AgentToolCommandCandidate } from "./types";

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

describe("prompt autocomplete helpers", () => {
  it("detects a dollar trigger at the cursor", () => {
    expect(findPromptAutocompleteTrigger("please $set", 11)).toEqual({
      prefix: "$",
      query: "set",
      start: 7,
      end: 11,
    });
  });

  it("detects a slash trigger after a line boundary", () => {
    expect(findPromptAutocompleteTrigger("first\n/go", 9)).toMatchObject({
      prefix: "/",
      query: "go",
      start: 6,
      end: 9,
    });
  });

  it("ignores selection ranges and ordinary words", () => {
    expect(findPromptAutocompleteTrigger("plain text", 5)).toBeNull();
    expect(findPromptAutocompleteTrigger("$tool", 1, 3)).toBeNull();
  });

  it("filters candidates by name, insert text, and description", () => {
    expect(filterToolCommandCandidates(candidates, "title").map((item) => item.name)).toEqual([
      "set_window_title",
    ]);
    expect(filterToolCommandCandidates(candidates, "manage").map((item) => item.name)).toEqual([
      "goal",
    ]);
  });

  it("clamps highlighted index around candidate boundaries", () => {
    expect(clampHighlightedIndex(-1, 2)).toBe(1);
    expect(clampHighlightedIndex(2, 2)).toBe(0);
    expect(clampHighlightedIndex(0, 0)).toBe(-1);
  });

  it("replaces only the active trigger token and restores cursor position", () => {
    const trigger = findPromptAutocompleteTrigger("run $set now", 8);
    expect(trigger).not.toBeNull();
    const result = replacePromptAutocompleteTrigger(
      { text: "run $set now", cursorStart: 8, cursorEnd: 8 },
      trigger!,
      candidates[0],
    );
    expect(result.text).toBe("run $set_window_title now");
    expect(result.cursorStart).toBe("run $set_window_title".length);
    expect(result.cursorEnd).toBe(result.cursorStart);
  });

  it("preserves the typed prefix when inserting candidates", () => {
    expect(insertionTextForPrefix(candidates[0], "/")).toBe("/set_window_title");
    expect(insertionTextForPrefix(candidates[1], "$")).toBe("$goal");
  });

  it("normalizes candidates and removes invalid or duplicate entries", () => {
    expect(
      normalizeToolCommandCandidates([
        candidates[0],
        { ...candidates[0] },
        { ...candidates[1], id: "blank-name", name: " ", insertText: "/goal" },
        { ...candidates[1], id: "blank-insert", insertText: " " },
      ]).map((item) => item.id),
    ).toEqual(["session:set_window_title"]);
  });
});
