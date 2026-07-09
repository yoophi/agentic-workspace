import { describe, expect, it } from "vitest";

import {
  availableCommandCandidatesFromSessionUpdate,
  clampHighlightedIndex,
  filterToolCommandCandidates,
  findPromptAutocompleteTrigger,
  insertionTextForPrefix,
  normalizeToolCommandCandidates,
  readAvailableCommandMetadata,
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
    expect(clampHighlightedIndex(5, 1)).toBe(0);
    expect(clampHighlightedIndex(-10, 1)).toBe(0);
    expect(clampHighlightedIndex(1, 3)).toBe(1);
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

  it("extracts available command candidates from session updates", () => {
    const extracted = availableCommandCandidatesFromSessionUpdate(
      {
        sessionUpdate: "available_commands_update",
        availableCommands: [
          {
            name: "mcp",
            description: "List configured Model Context Protocol (MCP) tools.",
            input: null,
          },
          {
            name: "$speckit-analyze",
            description: "Analyze spec artifacts.",
            input: null,
          },
          { name: " ", description: "ignored" },
        ],
      },
      { runId: "run-1", agentId: "codex", workingDirectory: "/repo" },
    );

    expect(extracted).toMatchObject([
      {
        name: "mcp",
        description: "List configured Model Context Protocol (MCP) tools.",
        insertText: "mcp",
        source: "appCommand",
        scope: { runId: "run-1", agentId: "codex", workingDirectory: "/repo" },
      },
      {
        name: "$speckit-analyze",
        description: "Analyze spec artifacts.",
        insertText: "$speckit-analyze",
        source: "extension",
      },
    ]);
  });

  it("reads available command metadata with descriptions and input hints", () => {
    const metadata = readAvailableCommandMetadata(
      {
        sessionUpdate: "available_commands_update",
        availableCommands: [
          {
            name: "mcp",
            description: "List configured Model Context Protocol (MCP) tools.",
            input: null,
          },
          {
            name: "review",
            description: "Review uncommitted changes.",
            input: { hint: "optional review instructions" },
          },
          {
            name: "$speckit-implement",
            description: "Execute tasks.",
            input: null,
          },
          {
            name: "/status",
            description: "Show status.",
            input: "optional filter",
          },
        ],
      },
      123,
    );

    expect(metadata).toMatchObject({
      sessionUpdate: "available_commands_update",
      updatedAt: 123,
      commands: [
        {
          name: "mcp",
          description: "List configured Model Context Protocol (MCP) tools.",
          inputHint: null,
          source: "appCommand",
        },
        {
          name: "review",
          description: "Review uncommitted changes.",
          inputHint: "optional review instructions",
          source: "appCommand",
        },
        {
          name: "$speckit-implement",
          description: "Execute tasks.",
          inputHint: null,
          source: "extension",
        },
        {
          name: "/status",
          description: "Show status.",
          inputHint: "optional filter",
          source: "appCommand",
        },
      ],
    });
  });

  it("extracts available command candidates from wrapped session/update payloads", () => {
    expect(
      availableCommandCandidatesFromSessionUpdate(
        {
          update: {
            sessionUpdate: "available_commands_update",
            availableCommands: [{ name: "status", description: "Display status." }],
          },
        },
        { runId: null, agentId: "codex", workingDirectory: "/repo" },
      ).map((item) => item.name),
    ).toEqual(["status"]);
  });

  it("reads available command metadata from params and message wrapper payloads", () => {
    expect(
      readAvailableCommandMetadata(
        {
          params: {
            update: {
              sessionUpdate: "available_commands_update",
              availableCommands: [{ name: "status", description: "Display status." }],
            },
          },
        },
        1,
      )?.commands.map((item) => item.name),
    ).toEqual(["status"]);
    expect(
      readAvailableCommandMetadata(
        {
          message: {
            params: {
              update: {
                sessionUpdate: "available_commands_update",
                availableCommands: [{ name: "$skill", description: "Run skill." }],
              },
            },
          },
        },
        1,
      )?.commands.map((item) => item.name),
    ).toEqual(["$skill"]);
  });

  it("ignores malformed command entries while preserving valid commands", () => {
    expect(
      readAvailableCommandMetadata(
        {
          sessionUpdate: "available_commands_update",
          availableCommands: [
            null,
            { description: "Missing name" },
            { name: " ", description: "Blank name" },
            { name: "valid", input: { hint: "optional text" } },
            { name: "bad-input", input: { schema: { type: "object" } } },
          ],
        },
        2,
      )?.commands,
    ).toMatchObject([
      { name: "valid", inputHint: "optional text" },
      { name: "bad-input", inputHint: null },
    ]);
  });

  it("returns empty metadata for missing or invalid availableCommands arrays", () => {
    expect(
      readAvailableCommandMetadata(
        { sessionUpdate: "available_commands_update" },
        3,
      ),
    ).toMatchObject({ commands: [], updatedAt: 3 });
    expect(
      readAvailableCommandMetadata(
        { sessionUpdate: "available_commands_update", availableCommands: "bad" },
        3,
      ),
    ).toMatchObject({ commands: [], updatedAt: 3 });
  });
});
