import { describe, expect, it } from "vitest";

import {
  clampPanelWidth,
  normalizePanelWidth,
  shouldPersistPanelWidth,
  toggleWorkspacePanel,
  workspacePanelIds,
} from "@/features/worktree-workspace/model/workspace-layout";

describe("toggleWorkspacePanel", () => {
  it("선택된 패널을 다시 선택하면 선택 없음으로 전환한다", () => {
    expect(toggleWorkspacePanel("git", "git")).toBeNull();
  });

  it("다른 패널을 선택하면 이전 선택을 교체한다", () => {
    expect(toggleWorkspacePanel("git", "markdown")).toBe("markdown");
  });

  it("선택 없음 상태에서 선택하면 그 패널만 선택한다", () => {
    expect(toggleWorkspacePanel(null, "speckit")).toBe("speckit");
  });

  it("네 패널 모두 토글 가능하다", () => {
    for (const panel of workspacePanelIds) {
      expect(toggleWorkspacePanel(null, panel)).toBe(panel);
      expect(toggleWorkspacePanel(panel, panel)).toBeNull();
    }
  });
});

describe("normalizePanelWidth", () => {
  it("양의 유한 값만 정수로 유지한다", () => {
    expect(normalizePanelWidth(420.4)).toBe(420);
    expect(normalizePanelWidth(420.6)).toBe(421);
  });

  it("0, 음수, 비유한 값, 누락 값은 저장 대상이 아니다", () => {
    expect(normalizePanelWidth(0)).toBeUndefined();
    expect(normalizePanelWidth(-10)).toBeUndefined();
    expect(normalizePanelWidth(Number.NaN)).toBeUndefined();
    expect(normalizePanelWidth(Number.POSITIVE_INFINITY)).toBeUndefined();
    expect(normalizePanelWidth(null)).toBeUndefined();
    expect(normalizePanelWidth(undefined)).toBeUndefined();
  });
});

describe("clampPanelWidth", () => {
  it("컨테이너가 충분하면 선호 폭을 그대로 쓴다", () => {
    expect(
      clampPanelWidth({
        preferredWidth: 600,
        containerWidth: 1600,
        minimumA: 360,
        minimumB: 480,
      }),
    ).toBe(600);
  });

  it("A의 최소 폭을 침범하면 표시 폭만 줄인다", () => {
    expect(
      clampPanelWidth({
        preferredWidth: 900,
        containerWidth: 1000,
        minimumA: 360,
        minimumB: 480,
      }),
    ).toBe(640);
  });

  it("B의 최소 폭보다 작아지지 않는다", () => {
    expect(
      clampPanelWidth({
        preferredWidth: 500,
        containerWidth: 700,
        minimumA: 360,
        minimumB: 480,
      }),
    ).toBe(480);
  });

  it("컨테이너 폭을 아직 모르면 선호 폭을 그대로 통과시킨다", () => {
    expect(
      clampPanelWidth({
        preferredWidth: 900,
        containerWidth: undefined,
        minimumA: 360,
        minimumB: 480,
      }),
    ).toBe(900);
  });

  it("선호 폭이 없으면 결과도 없다", () => {
    expect(
      clampPanelWidth({
        preferredWidth: undefined,
        containerWidth: 1600,
        minimumA: 360,
        minimumB: 480,
      }),
    ).toBeUndefined();
  });
});

describe("shouldPersistPanelWidth", () => {
  const base = { nextWidth: 700, preferredWidth: 900, hydrated: true, userInitiated: true };

  it("사용자가 직접 조절해 값이 바뀌면 저장한다", () => {
    expect(shouldPersistPanelWidth(base)).toBe(true);
  });

  it("hydrate 전에는 저장하지 않는다", () => {
    expect(shouldPersistPanelWidth({ ...base, hydrated: false })).toBe(false);
  });

  it("사용자 조작이 아니면 저장하지 않는다 — 표시 제한이 선호 폭을 덮어쓰지 못한다", () => {
    // 넓은 화면에서 900px을 저장한 뒤 좁은 창에서 640px로 제한된 상황.
    expect(
      shouldPersistPanelWidth({
        nextWidth: 640,
        preferredWidth: 900,
        hydrated: true,
        userInitiated: false,
      }),
    ).toBe(false);
  });

  it("값이 그대로면 저장하지 않는다", () => {
    expect(shouldPersistPanelWidth({ ...base, nextWidth: 900 })).toBe(false);
    expect(shouldPersistPanelWidth({ ...base, nextWidth: 900.2 })).toBe(false);
  });

  it("저장할 수 없는 폭은 저장하지 않는다", () => {
    expect(shouldPersistPanelWidth({ ...base, nextWidth: 0 })).toBe(false);
    expect(shouldPersistPanelWidth({ ...base, nextWidth: Number.NaN })).toBe(false);
  });

  it("선호 폭이 없던 Worktree는 첫 조절을 저장한다", () => {
    expect(shouldPersistPanelWidth({ ...base, preferredWidth: undefined })).toBe(true);
  });
});
