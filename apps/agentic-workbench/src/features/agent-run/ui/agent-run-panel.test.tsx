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
  it("keeps draft prompt injection separate from send and queue behavior", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain('externalPromptRequest.delivery === "draft"');
    expect(AGENT_RUN_PANEL_SOURCE).toContain("setPrompt(nextPrompt)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain('externalPromptRequest.delivery === "queue"');
    expect(AGENT_RUN_PANEL_SOURCE).toContain('enqueuePrompt(nextPrompt, "external-request")');
  });
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

describe("agent run start controls", () => {
  it("allows starting a run without prompt text when agent and session selection are ready", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("const canStartRun = Boolean(");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("selectedAgentId &&\n      !isRunning");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain(
      "selectedAgentId &&\n      prompt.trim() &&\n      !isRunning",
    );
    expect(AGENT_RUN_PANEL_SOURCE).toContain("if (!selectedAgentId) {");
    expect(AGENT_RUN_PANEL_SOURCE).not.toContain("if (!selectedAgentId || !goal) {");
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

describe("agent run history minimap integration", () => {
  it("projects the full conversation independently from the active event filter", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain(
      "const minimapEntries = useMemo(() => projectTimelineToMinimapEntries(items), [items]);",
    );
    expect(AGENT_RUN_PANEL_SOURCE).toContain("items={visibleItems}");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("entries={minimapEntries}");
  });

  it("shares the virtual timeline layout and uses clamped scroller requests", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("onLayoutChange={handleTimelineLayoutChange}");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("scrollTopForTimelineRatio(");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("scrollElement.scrollHeight - scrollElement.clientHeight");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("timelineOffsetInScroller");
  });

  it("switches filtered history to All before applying a pending seek", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain('if (filter !== "all")');
    expect(AGENT_RUN_PANEL_SOURCE).toContain("createPendingSeek(");
    expect(AGENT_RUN_PANEL_SOURCE).toContain('setFilter("all")');
    expect(AGENT_RUN_PANEL_SOURCE).toContain("applyPendingSeek(");
  });

  it("keeps one observer and one bottom-stick owner in the virtual timeline", () => {
    expect(AGENT_RUN_PANEL_SOURCE.match(/new ResizeObserver\(updateViewport\)/g)).toHaveLength(1);
    expect(AGENT_RUN_PANEL_SOURCE).toContain("stickToBottomRef.current = distanceFromBottom < 48");
  });

  it("publishes direct-scroll and measured-height revisions back to the minimap", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain('scrollElement.addEventListener("scroll", updateViewport');
    expect(AGENT_RUN_PANEL_SOURCE).toContain("setScrollTop(visibleTop)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain(
      "setViewportHeight(Math.max(visibleHeight, timelineViewportCapacity))",
    );
    expect(AGENT_RUN_PANEL_SOURCE).toContain("revisionRef.current += 1");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("measured: Object.prototype.hasOwnProperty.call");
  });

  it("keeps past positions stable and follows streaming only near the bottom", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("if (!scrollElement || !stickToBottomRef.current)");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("scrollElement.scrollTop = scrollElement.scrollHeight");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("}, [items.length, scrollParentRef, totalHeight]);");
  });

  it("owns visible state per mounted panel and exposes an accessible icon toggle", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain(
      "const [isMinimapVisible, setIsMinimapVisible] = useState(initialMinimapVisible);",
    );
    expect(AGENT_RUN_PANEL_SOURCE).toContain('aria-pressed={isMinimapVisible}');
    expect(AGENT_RUN_PANEL_SOURCE).toContain('isMinimapVisible ? "대화 미니맵 숨기기"');
    expect(AGENT_RUN_PANEL_SOURCE).toContain("<PanelRightCloseIcon />");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("<PanelRightOpenIcon />");
  });

  it("removes only the rail and keeps the same history scroller mounted", () => {
    const scrollerIndex = AGENT_RUN_PANEL_SOURCE.indexOf("ref={timelineScrollRef}");
    const conditionalIndex = AGENT_RUN_PANEL_SOURCE.indexOf("{isMinimapVisible && (");
    expect(scrollerIndex).toBeGreaterThan(-1);
    expect(conditionalIndex).toBeGreaterThan(scrollerIndex);
    expect(AGENT_RUN_PANEL_SOURCE).toContain('className="h-full min-w-0 flex-1 overflow-auto"');
  });

  it("restores the logical history position after toggle-driven remeasurement", () => {
    expect(AGENT_RUN_PANEL_SOURCE).toContain("hiddenMinimapRatioRef");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("pendingMinimapResizeSeekRef");
    expect(AGENT_RUN_PANEL_SOURCE).toContain("createViewportIndicator(timelineLayout).startRatio");
    expect(AGENT_RUN_PANEL_SOURCE).toContain(
      "timelineLayout.revision <= pending.requestedRevision",
    );
    expect(AGENT_RUN_PANEL_SOURCE).toContain(
      "performMinimapSeek(pending.targetRatio, timelineLayout)",
    );
  });
});
