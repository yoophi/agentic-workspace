// 호환 경로: 오늘의 Tauri command를 그대로 부르고, 이벤트도 오늘 방식 그대로 받는다(8단계에서 제거). 이벤트마다 받는
// 방식이 다르다 — run은 창 삽입만, 교환·orchestration은 Tauri 이벤트 + 창 삽입, 제목은 Tauri 이벤트 + 제목 삽입,
// worktree는 Tauri 이벤트만.
import { invoke } from "@tauri-apps/api/core";
import { listen as listenTauri } from "@tauri-apps/api/event";

import type { EventCallback, Transport } from "./transport";

type Mode = { tauri: boolean; fallback: string | null };

const MODES: Record<string, Mode> = {
  "agent-run-event": { tauri: false, fallback: "agent-run-event-fallback" },
  "workspace://mcp-window-title": { tauri: true, fallback: "mcp-window-title-fallback" },
  "workspace://worktree-changed": { tauri: true, fallback: null },
};

function modeOf(event: string): Mode {
  return MODES[event] ?? { tauri: true, fallback: `${event}-fallback` };
}

function listen<T>(event: string, callback: EventCallback<T>): Promise<() => void> {
  const mode = modeOf(event);
  let disposed = false;
  const handleFallback = (dispatched: Event) => {
    if (!disposed) {
      void callback((dispatched as CustomEvent<T>).detail);
    }
  };
  if (mode.fallback) {
    window.addEventListener(mode.fallback, handleFallback);
  }
  const removeFallback = () => {
    disposed = true;
    if (mode.fallback) {
      window.removeEventListener(mode.fallback, handleFallback);
    }
  };
  if (!mode.tauri) {
    return Promise.resolve(removeFallback);
  }
  return listenTauri<T>(event, (received) => {
    if (!disposed) {
      void callback(received.payload);
    }
  }).then((unlistenTauri) => () => {
    removeFallback();
    unlistenTauri();
  });
}

export const compatTransport: Transport = {
  kind: "compat",
  invoke: (command, args) => invoke(command, args),
  listen,
};
