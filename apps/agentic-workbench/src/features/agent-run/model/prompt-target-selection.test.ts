import { describe, expect, it } from "vitest";

import {
  addExtraPanel,
  createInitialAgentRunAreaState,
} from "./agent-run-panel-slots";
import { selectPromptTargets } from "./prompt-target-selection";

const slots = addExtraPanel(createInitialAgentRunAreaState()).slots;

describe("selectPromptTargets", () => {
  it("calculates focused, selected, all and coordinator targets", () => {
    expect(selectPromptTargets(slots, {
      mode: "focused",
      focusedPanelId: "extra-agent-run-1",
      selectedPanelIds: [],
    })).toMatchObject({ panelIds: ["extra-agent-run-1"], delegate: false });
    expect(selectPromptTargets(slots, {
      mode: "selected",
      focusedPanelId: "main-agent-run",
      selectedPanelIds: ["main-agent-run", "extra-agent-run-1"],
    })).toMatchObject({ panelIds: ["main-agent-run", "extra-agent-run-1"] });
    expect(selectPromptTargets(slots, {
      mode: "all",
      focusedPanelId: "main-agent-run",
      selectedPanelIds: [],
    })).toMatchObject({ panelIds: ["main-agent-run", "extra-agent-run-1"] });
    expect(selectPromptTargets(slots, {
      mode: "coordinator",
      focusedPanelId: "extra-agent-run-1",
      selectedPanelIds: [],
    })).toMatchObject({ panelIds: ["main-agent-run"], delegate: true });
  });

  it("rejects an empty selected target set", () => {
    expect(selectPromptTargets(slots, {
      mode: "selected",
      focusedPanelId: "main-agent-run",
      selectedPanelIds: [],
    })).toMatchObject({ valid: false, reason: "emptySelection" });
  });
});

