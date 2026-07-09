import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const AGENT_RUN_PANEL_SOURCE = readFileSync(
  new URL("./agent-run-panel.tsx", import.meta.url),
  "utf8",
);
const WORKTREE_AGENT_RUN_AREA_SOURCE = readFileSync(
  new URL("./worktree-agent-run-area.tsx", import.meta.url),
  "utf8",
);

describe("agent run settings entrypoints", () => {
  it("keeps the override failure action delegated to onOpenSettings", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("onClick={onOpenSettings}");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("isOverrideCommandFailure(error)");
  });

  it("passes onOpenSettings through without changing panel state", () => {
    expect(WORKTREE_AGENT_RUN_AREA_SOURCE).toContain("onOpenSettings={onOpenSettings}");
    expect(WORKTREE_AGENT_RUN_AREA_SOURCE).not.toContain("openSettingsWindow");
    expect(WORKTREE_AGENT_RUN_AREA_SOURCE).not.toContain("navigate(");
  });
});

describe("agent thread status indicator", () => {
  it("renders compact accessible labels for active, idle, and unknown thread status", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("function AgentThreadStatusBadge");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Agent active");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Agent idle");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Agent status unknown");
  });

  it("handles session_info_update without appending raw timeline items", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("isSessionInfoUpdateEvent(timelineEvent)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("readSessionInfoUpdateMetadata(timelineEvent)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("readAgentThreadStatus(timelineEvent)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("createSessionStartLifecycleStatusMessage(envelope.runId)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("createSessionIdleLifecycleStatusMessage({");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("appendSessionLifecycleStatusMessage(nextItems, envelope.runId, message)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("dispatchMcpWindowTitle(metadata.title)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("setSessionUpdatedAt(nextSessionUpdatedAt)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("return;");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain("function isIdleThreadStatusEvent");
  });

  it("renders session lifecycle status with low emphasis labels", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("if (status === \"sessionCreated\") return \"Session started\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("if (status === \"sessionIdle\") return \"Agent idle\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("return \"bg-muted text-muted-foreground\"");
  });

  it("renders session freshness in the run header without putting it in the timeline", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("formatSessionFreshnessLabel(sessionUpdatedAt)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("aria-label=\"Session updated at\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("<AgentThreadStatusBadge status={agentThreadStatus} />");
  });

  it("handles available_commands_update as metadata instead of raw timeline output", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("readAvailableCommandMetadata(");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("setAvailableCommandMetadata(nextAvailableCommandMetadata)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("setAvailableCommandCandidates(nextCandidates)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("formatAvailableCommandsSummary(");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("<AvailableCommandsPopover");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain("createSessionStartLifecycleStatusMessage(nextAvailableCommandMetadata");
  });

  it("renders available command details without raw input schema JSON", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("function AvailableCommandsPopover");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Available commands");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("Input: {command.inputHint}");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("max-h-80 min-w-0 overflow-y-auto");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("min-w-0 break-all font-mono");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("No commands available");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain("JSON.stringify(command");
  });
});

describe("prompt command autocomplete keyboard navigation", () => {
  it("routes autocomplete keyboard handling before prompt history navigation", () => {
    const autocompleteIndex = AGENT_RUN_PANEL_SOURCE.indexOf(
      "if (handleAutocompleteKeyDown(event))",
    );
    const historyIndex = AGENT_RUN_PANEL_SOURCE.indexOf(
      "handlePromptHistoryNavigation(event, \"previous\")",
    );

    expect(autocompleteIndex).toBeGreaterThan(-1);
    expect(historyIndex).toBeGreaterThan(-1);
    expect(autocompleteIndex).toBeLessThan(historyIndex);
  });

  it("handles ArrowUp and ArrowDown by clamping highlighted autocomplete candidates", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("function handleAutocompleteKeyDown");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("event.key === \"ArrowDown\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("event.key === \"ArrowUp\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("clampHighlightedIndex(current + 1, autocompleteCandidates.length)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("clampHighlightedIndex(current - 1, autocompleteCandidates.length)");
  });

  it("handles Enter, Tab, and Escape without adding multi-select state", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("event.key === \"Escape\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("event.key === \"Enter\" || event.key === \"Tab\"");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("selectAutocompleteCandidate(autocompleteCandidates[index])");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("setAutocompleteSuppression({");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain("selectedCommandCandidates");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain("selectedCommands");
  });

  it("keeps highlighted index clamped when candidate length changes", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("useEffect(() => {\n    setAutocompleteHighlightedIndex((current) =>");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("clampHighlightedIndex(current, autocompleteCandidates.length)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("}, [autocompleteCandidates.length])");
  });
});
