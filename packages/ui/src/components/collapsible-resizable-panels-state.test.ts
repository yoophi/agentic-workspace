import { describe, expect, it } from "vitest";

import {
  canCollapsePanel,
  clampPanelSize,
  collapsePanel,
  createPanelPairState,
  expandPanel,
  getPanelRestoreSize,
  updatePanelLayout,
} from "./collapsible-resizable-panels-state";

describe("collapsible resizable panel state", () => {
  describe("resizing", () => {
    it("starts with both panels open and resizing enabled", () => {
      const state = createPanelPairState({ leftSize: 40, rightSize: 60 });

      expect(state).toMatchObject({
        collapsedPanel: null,
        resizeEnabled: true,
        left: { side: "left", collapsed: false, lastExpandedSize: 40 },
        right: { side: "right", collapsed: false, lastExpandedSize: 60 },
      });
    });

    it("captures the completed layout for each panel", () => {
      const state = createPanelPairState({ leftSize: 40, rightSize: 60 });
      const resized = updatePanelLayout(state, 55, 45);

      expect(resized.left.lastExpandedSize).toBe(55);
      expect(resized.right.lastExpandedSize).toBe(45);
      expect(resized.resizeEnabled).toBe(true);
    });

    it("clamps panel sizes to their configured boundaries", () => {
      const state = createPanelPairState({
        leftSize: 40,
        rightSize: 60,
        leftMinSize: 25,
        leftMaxSize: 70,
        rightMinSize: 20,
        rightMaxSize: 75,
      });

      const resized = updatePanelLayout(state, 10, 90);

      expect(resized.left.lastExpandedSize).toBe(25);
      expect(resized.right.lastExpandedSize).toBe(75);
      expect(clampPanelSize(45, 20, 70)).toBe(45);
      expect(clampPanelSize(5, 20, 70)).toBe(20);
      expect(clampPanelSize(90, 20, 70)).toBe(70);
    });

    it("ignores completed resize events while a panel is collapsed", () => {
      const state = createPanelPairState({ leftSize: 40, rightSize: 60 });
      const collapsed = {
        ...state,
        collapsedPanel: "left" as const,
        left: { ...state.left, collapsed: true },
        resizeEnabled: false,
      };

      expect(updatePanelLayout(collapsed, 5, 95)).toEqual(collapsed);
    });
  });

  describe("collapsing and restoring", () => {
    it("collapses either panel while the other panel is open", () => {
      const state = createPanelPairState({ leftSize: 40, rightSize: 60 });
      const leftCollapsed = collapsePanel(state, "left", 38);

      expect(leftCollapsed).toMatchObject({
        collapsedPanel: "left",
        resizeEnabled: false,
        left: { collapsed: true, lastExpandedSize: 38 },
        right: { collapsed: false, lastExpandedSize: 60 },
      });
      expect(canCollapsePanel(state, "left")).toBe(true);
      expect(canCollapsePanel(state, "right")).toBe(true);
    });

    it("rejects collapsing the last open panel", () => {
      const state = createPanelPairState({ leftSize: 40, rightSize: 60 });
      const leftCollapsed = collapsePanel(state, "left", 40);

      expect(canCollapsePanel(leftCollapsed, "right")).toBe(false);
      expect(collapsePanel(leftCollapsed, "right", 60)).toBe(leftCollapsed);
      expect(leftCollapsed.left.collapsed && leftCollapsed.right.collapsed).toBe(false);
    });

    it("expands a collapsed panel and restores resizing", () => {
      const state = createPanelPairState({ leftSize: 40, rightSize: 60 });
      const leftCollapsed = collapsePanel(state, "left", 37);
      const expanded = expandPanel(leftCollapsed, "left");

      expect(expanded).toMatchObject({
        collapsedPanel: null,
        resizeEnabled: true,
        left: { collapsed: false, lastExpandedSize: 37 },
        right: { collapsed: false, lastExpandedSize: 60 },
      });
    });

    it("preserves each panel's saved width independently", () => {
      const initial = createPanelPairState({ leftSize: 40, rightSize: 60 });
      const leftRestored = expandPanel(collapsePanel(initial, "left", 36), "left");
      const resized = updatePanelLayout(leftRestored, 45, 55);
      const rightCollapsed = collapsePanel(resized, "right", 54);

      expect(rightCollapsed.left.lastExpandedSize).toBe(45);
      expect(rightCollapsed.right.lastExpandedSize).toBe(54);
      expect(getPanelRestoreSize(rightCollapsed, "right")).toBe(54);
    });

    it("clamps a saved width when the available boundary shrinks", () => {
      const state = collapsePanel(
        createPanelPairState({ leftSize: 40, rightSize: 60 }),
        "right",
        60,
      );

      expect(getPanelRestoreSize(state, "right", 20, 45)).toBe(45);
    });
  });
});
