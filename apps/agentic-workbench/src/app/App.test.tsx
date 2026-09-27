import { readFileSync } from "node:fs";

import {
  MCP_WINDOW_TITLE_EVENT,
  MCP_WINDOW_TITLE_FALLBACK_EVENT,
} from "@/shared/lib/workspace-window-title";

const COMPAT_TRANSPORT_SOURCE = readFileSync(
  new URL("../shared/api/transport/compat-transport.ts", import.meta.url),
  "utf8",
);
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
    // 043: 제목 삽입(fallback) 수신은 호환 transport가 맡고, App은 창 transport로 같은 이벤트 이름을 받는다.
    expect(APP_SOURCE).toContain('listen<McpWindowTitleEvent>(MCP_WINDOW_TITLE_EVENT');
    expect(COMPAT_TRANSPORT_SOURCE).toContain(`"${MCP_WINDOW_TITLE_EVENT}": { tauri: true, fallback: "${MCP_WINDOW_TITLE_FALLBACK_EVENT}" }`);
    expect(APP_SOURCE).toContain("normalizeAgentWindowTitle(title)");
    expect(APP_SOURCE).toContain("getCurrentWindow().setTitle(windowTitle)");
  });
});
