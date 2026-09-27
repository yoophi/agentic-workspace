// 창 부팅 때 경로를 한 번 정한다(043 T026, research R3·R4, contracts §3): 연결 정보 → handshake → 네트워크 전달 선언이 모두
// 성공하면 네트워크 경로, 하나라도 실패하면 처음부터 호환 경로(진단 기록 `[workbench-client] using compat path: <이유>`).
// 이후 전환은 없다 — 네트워크 창은 끊겨도 재연결만 하고, 호출·이벤트가 한 경로를 쓴다.
import { invoke as invokeDesktop } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createConnection,
  createEventClient,
  createWorkbenchClient,
  type Connection,
  type ConnectionInfo,
} from "@yoophi/workbench-client";

import { compatTransport, setTransport } from "@/shared/api/transport";
import { setConnectionStatus } from "@/shared/api/transport/connection-status";
import { createHttpTransport } from "@/shared/api/transport/http-transport";
import { createNetworkEvents } from "@/shared/api/transport/network-events";

export interface BootstrapDeps {
  getConnection: () => Promise<ConnectionInfo>;
  ensureWindowBench: (open: boolean, hint?: string | null) => Promise<string | null>;
  declareNetworkDelivery: (incarnation: string) => Promise<void>;
  windowLabel: () => string;
  fetch?: typeof fetch;
  log?: (line: string) => void;
  /** 이벤트 소켓(시험은 가짜 소켓을 넣는다). */
  openSocket?: Parameters<typeof createEventClient>[0]["openSocket"];
  /** 서버 세대가 바뀌었을 때(서버 재기동). 기본: 창을 다시 불러와 처음부터 부팅한다 — 작업대를 새로 열고 구독·화면
   *  상태를 모두 새로 맞춘다(research R9 전체 재동기). 응답을 잃은 변경은 호출 클라이언트가 다시 보내지 않는다. */
  onEpochChanged?: (epoch: string) => void;
}

export interface BootstrapResult {
  kind: "http" | "compat";
  connection?: Connection;
}

export const desktopBootstrapDeps: BootstrapDeps = {
  getConnection: () => invokeDesktop<ConnectionInfo>("get_workbench_connection"),
  ensureWindowBench: (open, hint) => invokeDesktop<string | null>("ensure_window_bench", { open, hint: hint ?? null }),
  declareNetworkDelivery: (incarnation) => invokeDesktop<void>("declare_network_delivery", { incarnation }),
  windowLabel: () => getCurrentWindow().label,
};

function reason(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export async function bootstrapTransport(deps: BootstrapDeps = desktopBootstrapDeps): Promise<BootstrapResult> {
  const log = deps.log ?? ((line: string) => console.info(line));
  const connection = createConnection({ fetchConnection: deps.getConnection, fetch: deps.fetch });
  try {
    await connection.start();
    const incarnation = connection.incarnation();
    if (!incarnation) {
      throw new Error("connection is not bound to a window incarnation");
    }
    await deps.declareNetworkDelivery(incarnation);
  } catch (error) {
    connection.close();
    setTransport(compatTransport);
    log(`[workbench-client] using compat path: ${reason(error)}`);
    return { kind: "compat" };
  }
  const client = createWorkbenchClient({ connection, fetch: deps.fetch });
  let resynced = false;
  const onEpochChanged = (epoch: string) => {
    if (resynced) {
      return;
    }
    resynced = true;
    log(`[workbench-client] server epoch changed to ${epoch}: resynchronizing the window`);
    (deps.onEpochChanged ?? (() => window.location.reload()))(epoch);
  };
  const eventClient = createEventClient({
    connection,
    fetch: deps.fetch,
    openSocket: deps.openSocket,
    onEpochChanged,
    onStreamError: (streamId, error) => log(`[workbench-client] stream ${streamId}: ${error}`),
  });
  connection.onEpochChanged(onEpochChanged);
  connection.onState(setConnectionStatus);
  const events = createNetworkEvents({ events: eventClient, client });
  setTransport(
    createHttpTransport({ client, ensureWindowBench: deps.ensureWindowBench, windowLabel: deps.windowLabel(), events }),
  );
  return { kind: "http", connection };
}
