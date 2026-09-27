// 043 T028(SC-001): 화면 코드는 서버 소유 command를 Tauri `invoke`로 직접 부르지 않는다 — 모두 창 transport(`@/shared/api/
// transport`)를 거친다. Tauri `invoke`로 직접 부를 수 있는 것은 데스크톱 표현·부팅 command 목록뿐이다(명시적 허용 목록).
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

import { COMMANDS } from "./command-table";

const SRC = join(__dirname, "../../..");

/** 데스크톱 표현·앱 내부 command(인벤토리 D 표). */
const DESKTOP_COMMANDS = new Set([
  "get_appearance_preferences",
  "set_font_size_step",
  "adjust_font_size_step",
  "get_worktree_workspace_layout",
  "save_worktree_workspace_layout",
  "open_settings_window",
  "open_worktree_window",
  "open_external_url",
  "get_workbench_connection",
  "ensure_window_bench",
  "declare_network_delivery",
  "withdraw_network_delivery",
  "start_worktree_watcher",
  "stop_worktree_watcher",
]);

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      return name === "node_modules" ? [] : sourceFiles(path);
    }
    return /\.(ts|tsx)$/.test(name) && !/\.test\.|\.test-d\.|\.stories\.|test-harness/.test(name) ? [path] : [];
  });
}

function tauriInvokeCalls(source: string): string[] {
  const imported = /import\s*\{([^}]*)\}\s*from\s*"@tauri-apps\/api\/core"/.exec(source);
  if (!imported) {
    return [];
  }
  const names = imported[1]
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part.startsWith("invoke"))
    .map((part) => (part.includes(" as ") ? part.split(" as ")[1].trim() : part));
  return names.flatMap((name) =>
    [...source.matchAll(new RegExp(`\\b${name}(?:<[^>]*>)?\\(\\s*"([a-z_]+)"`, "g"))].map((match) => match[1]),
  );
}

describe("no direct Tauri invoke of server-owned commands", () => {
  const files = sourceFiles(SRC);

  it("scans the app sources", () => {
    expect(files.length).toBeGreaterThan(50);
  });

  it("only calls desktop-presentation commands through Tauri invoke", () => {
    const offenders = files.flatMap((file) =>
      tauriInvokeCalls(readFileSync(file, "utf8"))
        .filter((command) => !DESKTOP_COMMANDS.has(command))
        .map((command) => `${relative(SRC, file)}: ${command}`),
    );
    expect(offenders).toEqual([]);
  });

  it("keeps the desktop list and the server-owned table disjoint", () => {
    expect(Object.keys(COMMANDS).filter((command) => DESKTOP_COMMANDS.has(command))).toEqual([]);
  });

  it("detects a direct server-owned call (self-check of the scanner)", () => {
    const sample = 'import { invoke } from "@tauri-apps/api/core";\ninvoke<Project[]>("list_projects");';
    expect(tauriInvokeCalls(sample)).toEqual(["list_projects"]);
    const aliased = 'import { invoke as invokeDesktop } from "@tauri-apps/api/core";\ninvokeDesktop("start_agent_run", {});';
    expect(tauriInvokeCalls(aliased)).toEqual(["start_agent_run"]);
  });
});
