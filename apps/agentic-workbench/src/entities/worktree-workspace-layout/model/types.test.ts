import { describe, expect, it } from "vitest";

import {
  emptyWorktreeWorkspaceLayout,
  withOuterPanelWidth,
  withPanelWidth,
  type WorktreeWorkspaceLayout,
} from "@/entities/worktree-workspace-layout/model/types";

const saved: WorktreeWorkspaceLayout = {
  workingDirectory: "/repo/tree-a",
  outerPanelWidthPx: 640,
  panelWidthsPx: { git: 520, markdown: 700 },
};

describe("emptyWorktreeWorkspaceLayout", () => {
  it("저장된 폭이 없는 Worktree의 기본 레코드를 만든다", () => {
    expect(emptyWorktreeWorkspaceLayout("/repo/tree-a")).toEqual({
      workingDirectory: "/repo/tree-a",
      panelWidthsPx: {},
    });
  });
});

describe("withOuterPanelWidth", () => {
  it("바깥 B 폭만 교체하고 내부 B 폭은 유지한다", () => {
    expect(withOuterPanelWidth(saved, 800)).toEqual({
      workingDirectory: "/repo/tree-a",
      outerPanelWidthPx: 800,
      panelWidthsPx: { git: 520, markdown: 700 },
    });
  });

  it("원본을 변경하지 않는다", () => {
    withOuterPanelWidth(saved, 800);
    expect(saved.outerPanelWidthPx).toBe(640);
  });
});

describe("withPanelWidth", () => {
  it("지정한 패널 종류의 내부 B 폭만 교체한다", () => {
    expect(withPanelWidth(saved, "git", 560)).toEqual({
      workingDirectory: "/repo/tree-a",
      outerPanelWidthPx: 640,
      panelWidthsPx: { git: 560, markdown: 700 },
    });
  });

  it("다른 패널 종류의 내부 B 폭을 덮어쓰지 않는다", () => {
    const next = withPanelWidth(saved, "files", 480);
    expect(next.panelWidthsPx).toEqual({ git: 520, markdown: 700, files: 480 });
  });

  it("바깥 B 폭을 지우지 않는다", () => {
    expect(withPanelWidth(saved, "speckit", 620).outerPanelWidthPx).toBe(640);
  });

  it("내부·바깥 저장을 연달아 적용해도 서로를 잃지 않는다", () => {
    const next = withPanelWidth(withOuterPanelWidth(saved, 900), "speckit", 620);
    expect(next).toEqual({
      workingDirectory: "/repo/tree-a",
      outerPanelWidthPx: 900,
      panelWidthsPx: { git: 520, markdown: 700, speckit: 620 },
    });
  });
});
