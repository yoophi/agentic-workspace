// @vitest-environment happy-dom

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";

import type { MinimapEntry } from "@/entities/agent-run/model";
import { EMPTY_TIMELINE_LAYOUT_SNAPSHOT } from "@/features/agent-run/model/agent-run-minimap";

import { AgentRunMinimap } from "./agent-run-minimap";

const MINIMAP_SOURCE = readFileSync(
  resolve(process.cwd(), "src/features/agent-run/ui/agent-run-minimap.tsx"),
  "utf8",
);

(
  globalThis as typeof globalThis & {
    IS_REACT_ACT_ENVIRONMENT: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const mountedRoots: Array<ReturnType<typeof createRoot>> = [];

afterEach(async () => {
  document.documentElement.removeAttribute("data-font-size-step");
  await act(async () => {
    mountedRoots.splice(0).forEach((root) => root.unmount());
  });
  document.body.replaceChildren();
});

async function renderMinimap(fontSizeStep: "0" | "2", entries: MinimapEntry[] = []) {
  document.documentElement.dataset.fontSizeStep = fontSizeStep;
  const container = document.createElement("div");
  document.body.append(container);
  const root = createRoot(container);
  mountedRoots.push(root);

  await act(async () => {
    root.render(
      <AgentRunMinimap
        entries={entries}
        layoutSnapshot={EMPTY_TIMELINE_LAYOUT_SNAPSHOT}
        onSeek={() => undefined}
      />,
    );
  });

  return container;
}

function findElementWithExactText(container: HTMLElement, text: string) {
  return Array.from(container.querySelectorAll<HTMLElement>("div")).find(
    (element) => element.childElementCount === 0 && element.textContent === text,
  );
}

describe("agent run minimap UI contract", () => {
  it.each(["0", "2"] as const)(
    "keeps its title and empty state at 10px for font size step %s",
    async (fontSizeStep) => {
      const container = await renderMinimap(fontSizeStep);

      expect(findElementWithExactText(container, "대화 미니맵")?.style.fontSize).toBe("10px");
      expect(findElementWithExactText(container, "대화 없음")?.style.fontSize).toBe("10px");
    },
  );

  it.each(["0", "2"] as const)(
    "keeps entry summaries at 9px for font size step %s",
    async (fontSizeStep) => {
      const container = await renderMinimap(fontSizeStep, [
        {
          id: "user-1",
          runId: "run-1",
          role: "user",
          summary: "Inspect the worktree",
          contentWeight: 1,
          sourceOrder: 0,
        },
      ]);

      expect(
        (container.querySelector("[data-minimap-entry-role='user']") as HTMLElement | null)?.style
          .fontSize,
      ).toBe("9px");
    },
  );

  it("renders semantic conversation entries without duplicating rich timeline content", () => {
    expect(MINIMAP_SOURCE).toContain('data-minimap-entry-role={entry.role}');
    expect(MINIMAP_SOURCE).toContain('entry.role === "user"');
    expect(MINIMAP_SOURCE).toContain('title={entry.summary');
    expect(MINIMAP_SOURCE).not.toContain("StreamingMarkdown");
    expect(MINIMAP_SOURCE).not.toContain("AgentRunMermaidDiagram");
  });

  it("exposes one accessible vertical viewport slider", () => {
    expect(MINIMAP_SOURCE).toContain('role="slider"');
    expect(MINIMAP_SOURCE).toContain('aria-orientation="vertical"');
    expect(MINIMAP_SOURCE).toContain('aria-valuenow={Math.round(indicator.startRatio * 100)}');
    expect(MINIMAP_SOURCE).toContain('aria-disabled={isDisabled}');
    expect(MINIMAP_SOURCE).toContain('tabIndex={isDisabled ? -1 : 0}');
  });

  it("captures pointer drag and schedules bounded seek updates", () => {
    expect(MINIMAP_SOURCE).toContain("setPointerCapture(event.pointerId)");
    expect(MINIMAP_SOURCE).toContain("releasePointerCapture(event.pointerId)");
    expect(MINIMAP_SOURCE).toContain("requestAnimationFrame");
    expect(MINIMAP_SOURCE).toContain("pointerSeekRatio({");
    expect(MINIMAP_SOURCE).toContain('"pointer"');
  });

  it("delegates Arrow, Page, Home, and End behavior to pure keyboard math", () => {
    expect(MINIMAP_SOURCE).toContain("keyboardSeekRatio({");
    expect(MINIMAP_SOURCE).toContain("event.preventDefault()");
    expect(MINIMAP_SOURCE).toContain('onSeek(targetRatio, "keyboard")');
  });

  it("keeps empty, disabled, and narrow rail states bounded", () => {
    expect(MINIMAP_SOURCE).toContain("대화 없음");
    expect(MINIMAP_SOURCE).toContain('const isDisabled = indicator.disabled || entries.length === 0;');
    expect(MINIMAP_SOURCE).toContain('tabIndex={isDisabled ? -1 : 0}');
    expect(MINIMAP_SOURCE).toContain("w-28 shrink-0");
    expect(MINIMAP_SOURCE).toContain("min-h-0 flex-1 overflow-hidden");
    expect(MINIMAP_SOURCE).toContain("left-0.5 right-0.5 overflow-hidden");
  });
});
