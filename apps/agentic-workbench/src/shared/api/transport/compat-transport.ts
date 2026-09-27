// 호환 경로: 오늘의 Tauri command를 그대로 부른다(8단계에서 제거).
import { invoke } from "@tauri-apps/api/core";

import type { Transport } from "./transport";

export const compatTransport: Transport = {
  kind: "compat",
  invoke: (command, args) => invoke(command, args),
};
