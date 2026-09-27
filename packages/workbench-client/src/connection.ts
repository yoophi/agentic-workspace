// 연결 수명(043 T022, research R5·R9·R10): 데스크톱 앱이 준 짧은 자격 증명으로 handshake하고, 수명의 80%에서 갱신하며,
// 끊김을 보고받으면 backoff(250ms → 10s, jitter)로 handshake를 다시 한다. 세대(`serverEpoch`)가 바뀌면 알린다.
// 자격 증명은 메모리에만 두고 기록하지 않는다.

export interface ConnectionInfo {
  baseUrl: string;
  token: string;
  expiresAt: string;
  /** 창 토큰이 묶인 incarnation(데스크톱). */
  incarnation?: string;
}

export interface Handshake {
  serverEpoch: string;
  selectedProtocolVersion: number;
  instanceId: string;
}

export type ConnectionState = "connecting" | "connected" | "reconnecting" | "disconnected";

export interface ConnectionOptions {
  /** 데스크톱: `get_workbench_connection`. */
  fetchConnection: () => Promise<ConnectionInfo>;
  fetch?: typeof fetch;
  random?: () => number;
  /** 연속 실패가 이만큼 쌓이면 `disconnected`로 보인다(재시도는 계속). */
  disconnectedAfter?: number;
}

export interface Connection {
  start(): Promise<Handshake>;
  state(): ConnectionState;
  epoch(): string | undefined;
  incarnation(): string | undefined;
  credentials(): { baseUrl: string; token: string };
  expiresAt(): number;
  refreshCredentials(): Promise<void>;
  reportLost(): void;
  whenConnected(): Promise<Handshake>;
  onState(listener: (state: ConnectionState) => void): () => void;
  onEpochChanged(listener: (epoch: string) => void): () => void;
  close(): void;
}

export const PROTOCOL_VERSIONS = [1];
const INITIAL_BACKOFF_MS = 250;
const MAX_BACKOFF_MS = 10_000;
const REFRESH_FRACTION = 0.8;

class HandshakeUnauthorized extends Error {}

export function createConnection(options: ConnectionOptions): Connection {
  const fetchImpl = options.fetch ?? fetch;
  const random = options.random ?? Math.random;
  const disconnectedAfter = options.disconnectedAfter ?? 5;
  let info: ConnectionInfo | undefined;
  let expiresAt = 0;
  let currentEpoch: string | undefined;
  let currentState: ConnectionState = "connecting";
  let lastHandshake: Handshake | undefined;
  let refreshTimer: ReturnType<typeof setTimeout> | undefined;
  let reconnecting = false;
  let closed = false;
  let waiters: Array<(handshake: Handshake) => void> = [];
  const stateListeners = new Set<(state: ConnectionState) => void>();
  const epochListeners = new Set<(epoch: string) => void>();

  function setState(next: ConnectionState) {
    if (currentState === next) {
      return;
    }
    currentState = next;
    for (const listener of stateListeners) {
      listener(next);
    }
  }

  function scheduleRefresh() {
    if (refreshTimer !== undefined) {
      clearTimeout(refreshTimer);
    }
    const lifetime = Math.max(0, expiresAt - Date.now());
    refreshTimer = setTimeout(() => {
      refreshTimer = undefined;
      void refreshCredentials().catch(() => {
        // 갱신 실패: 곧 만료될 토큰으로 버티지 않고 재연결 루프에 맡긴다(루프가 새 자격 증명을 받는다).
        reportLost();
      });
    }, lifetime * REFRESH_FRACTION);
  }

  /** 새 연결 정보를 받는다. 끝점이 바뀌었으면(서버 재기동 = 새 포트) 곧바로 handshake해 세대를 갱신한다 — 호출
   *  클라이언트가 재시도 전에 세대 경계를 확인할 수 있어야 한다. */
  async function refreshCredentials() {
    const next = await options.fetchConnection();
    const moved = info !== undefined && info.baseUrl !== next.baseUrl;
    info = next;
    expiresAt = Date.parse(next.expiresAt);
    if (!closed) {
      scheduleRefresh();
    }
    if (moved) {
      try {
        connected(await handshake());
      } catch (error) {
        reportLost();
        throw error;
      }
    }
  }

  async function handshake(): Promise<Handshake> {
    if (!info) {
      throw new Error("connection is not started");
    }
    const response = await fetchImpl(`${info.baseUrl}/v1/system/handshake`, {
      method: "POST",
      headers: { authorization: `Bearer ${info.token}`, "content-type": "application/json" },
      body: JSON.stringify({
        supportedProtocolVersions: PROTOCOL_VERSIONS,
        client: { name: "agentic-workbench", version: "043" },
      }),
    });
    if (response.status === 401) {
      throw new HandshakeUnauthorized("handshake unauthorized");
    }
    if (!response.ok) {
      throw new Error(`handshake failed with status ${response.status}`);
    }
    const body = (await response.json()) as Handshake;
    if (!PROTOCOL_VERSIONS.includes(body.selectedProtocolVersion)) {
      throw new Error(`unsupported protocol version ${body.selectedProtocolVersion}`);
    }
    return body;
  }

  function connected(result: Handshake) {
    const previous = currentEpoch;
    currentEpoch = result.serverEpoch;
    lastHandshake = result;
    setState("connected");
    const pending = waiters;
    waiters = [];
    for (const resolve of pending) {
      resolve(result);
    }
    if (previous !== undefined && previous !== result.serverEpoch) {
      for (const listener of epochListeners) {
        listener(result.serverEpoch);
      }
    }
  }

  /** 재연결 시도: 지금 끝점으로 handshake하고, 실패하면(401이든 연결 거부든) 연결 정보를 다시 받아(서버가 다른 포트로
   *  다시 떴을 수 있다) 그 끝점으로 한 번 더. */
  async function attempt(): Promise<Handshake> {
    try {
      return await handshake();
    } catch {
      const next = await options.fetchConnection();
      info = next;
      expiresAt = Date.parse(next.expiresAt);
      if (!closed) {
        scheduleRefresh();
      }
      return handshake();
    }
  }

  async function reconnectLoop() {
    let failures = 0;
    while (!closed) {
      try {
        connected(await attempt());
        reconnecting = false;
        return;
      } catch {
        failures += 1;
        if (failures >= disconnectedAfter) {
          setState("disconnected");
        }
        const base = Math.min(INITIAL_BACKOFF_MS * 2 ** (failures - 1), MAX_BACKOFF_MS);
        const delay = Math.min(MAX_BACKOFF_MS, base * (0.8 + 0.4 * random()));
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
    }
  }

  function reportLost() {
    if (closed || reconnecting || currentState === "connecting") {
      return;
    }
    reconnecting = true;
    setState("reconnecting");
    void reconnectLoop();
  }

  return {
    async start() {
      await refreshCredentials();
      const result = await handshake();
      connected(result);
      return result;
    },
    state: () => currentState,
    epoch: () => currentEpoch,
    incarnation: () => info?.incarnation,
    credentials() {
      if (!info) {
        throw new Error("connection is not started");
      }
      return { baseUrl: info.baseUrl, token: info.token };
    },
    expiresAt: () => expiresAt,
    refreshCredentials,
    reportLost,
    whenConnected() {
      if (currentState === "connected" && lastHandshake) {
        return Promise.resolve(lastHandshake);
      }
      return new Promise((resolve) => waiters.push(resolve));
    },
    onState(listener) {
      stateListeners.add(listener);
      listener(currentState);
      return () => stateListeners.delete(listener);
    },
    onEpochChanged(listener) {
      epochListeners.add(listener);
      return () => epochListeners.delete(listener);
    },
    close() {
      closed = true;
      if (refreshTimer !== undefined) {
        clearTimeout(refreshTimer);
      }
    },
  };
}
