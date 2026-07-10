import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const MINIMAP_SOURCE = readFileSync(
  new URL("./agent-run-minimap.tsx", import.meta.url),
  "utf8",
);

describe("agent run minimap UI contract", () => {
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
