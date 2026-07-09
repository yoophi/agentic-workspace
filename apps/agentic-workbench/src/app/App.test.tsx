import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const APP_SOURCE = readFileSync(new URL("./App.tsx", import.meta.url), "utf8");

describe("App settings entrypoints", () => {
  it("opens settings through the dedicated window command instead of route navigation", () => {
    expect(APP_SOURCE).toContain("openSettingsWindow");
    expect(APP_SOURCE).toContain("path=\"/settings-window\"");
    expect(APP_SOURCE).not.toContain("/settings?returnTo=");
    expect(APP_SOURCE).not.toContain("path=\"/settings\"");
  });

  it("passes the dedicated opener to worktree session routes", () => {
    expect(APP_SOURCE).toContain("onOpenSettings={openSettings}");
  });

  it("uses the shared MCP window title event helper for standalone session titles", () => {
    expect(APP_SOURCE).toContain("@/shared/lib/workspace-window-title");
    expect(APP_SOURCE).toContain("MCP_WINDOW_TITLE_EVENT");
    expect(APP_SOURCE).toContain("MCP_WINDOW_TITLE_FALLBACK_EVENT");
    expect(APP_SOURCE).toContain("normalizeAgentWindowTitle(title)");
    expect(APP_SOURCE).toContain("getCurrentWindow().setTitle(windowTitle)");
  });
});
