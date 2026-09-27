// 이벤트 클라이언트(043 T033·T044·T045, research R7·R8, contracts §5).
//
// - 스트림 하나당 WebSocket 하나. 표(`POST /v1/event-tickets`)의 cursor는 붙은 수신자들의 **반영 완료** 순번 최솟값이다.
// - 수신자 `onEvent`는 void 또는 Promise. Promise는 settle까지 기다리고, 수신자마다 하나씩 차례로 처리한다. 이행 = 반영 완료
//   (그 수신자의 cursor 전진). 거절·동기 예외 = 그 수신자만 실패 → 스트림 스냅샷으로 재동기(`onReset`) 뒤 스냅샷 이후만 이어
//   받는다. 다른 수신자는 막지 않는다. 이미 넘긴 순번은 그 수신자에게 다시 넘기지 않는다.
// - 수신자가 0명인 동안 도착한 프레임은 대기열에 둔다(상한 초과 → 소켓을 닫고 다음 수신자가 붙을 때 cursor에서 다시 구독).
// - 보관 범위 gap(`retentionExceeded`·`unknownStream`): gap의 `lastSequence`로 새 표를 열어 live를 먼저 확보하고, 그동안
//   오는 이벤트는 버퍼에 담았다가, hello 뒤 스냅샷을 불러 수신자를 재설정하고 버퍼를 스냅샷 기준으로 걸러 넘긴다. 새 gap이
//   오면 처음부터(최대 `maxRecoveryAttempts`). `hello`만으로는 복구 성공으로 보지 않는다(042 hub는 등록 실패에도 hello를 보낸다).
// - `epochChanged`: 알린 뒤(`onEpochChanged`) 새 세대의 처음부터 같은 절차. `evicted`: 스냅샷으로 재설정하고 끝낸다.
// - `subscriberLagged`·`shutdown`·연결 끊김: 같은 cursor로 다시 연결한다(backoff 250ms → 10s, jitter).
import type { EventEnvelope, EventFrame, GapNotice, StreamCursor } from "./operation-map";

export interface EventConnectionPort {
  credentials(): { baseUrl: string; token: string };
  epoch(): string | undefined;
  reportLost(): void;
  whenConnected(): Promise<{ serverEpoch: string }>;
  refreshCredentials(): Promise<void>;
}

export interface SocketLike {
  readyState: number;
  onopen: ((event: unknown) => void) | null;
  onmessage: ((event: { data: string }) => void) | null;
  onclose: ((event: { code: number; reason: string }) => void) | null;
  onerror: ((event: unknown) => void) | null;
  close(): void;
}

export interface StreamListener {
  onEvent(event: EventEnvelope): void | Promise<void>;
  /** 스냅샷으로 상태를 다시 맞춘다(수신자 실패·보관 gap·세대 변경). `delivered`는 이 수신자가 반영을 마친 순번 —
   *  run처럼 스냅샷이 이벤트 목록이면 그 뒤만 다시 반영하면 된다. */
  onReset?(snapshot: unknown, context: { delivered: number }): void | Promise<void>;
}

export interface SnapshotSource {
  load(): Promise<unknown>;
  /** 스냅샷 뒤에 넘길 이벤트인지(run: 순번, orchestration: revision, 교환: 모두 — requestId upsert). */
  passes(event: EventEnvelope, snapshot: unknown): boolean;
}

export interface SubscribeOptions {
  after?: number;
  snapshot?: SnapshotSource;
  /** 알림 스트림(worktree 등): 보관이 없어 끊긴 동안의 알림은 사라진다 — 다시 연결될 때마다 수신자를 스냅샷으로
   *  재설정한다(재조회 신호). 첫 연결에서는 하지 않는다. */
  resyncOnReconnect?: boolean;
  /** 첫 연결(hello)에서도 스냅샷으로 재설정한다 — 구독 전에 일어난 일을 맞춘다(교환 재조정 등). */
  resyncOnStart?: boolean;
}

export interface EventClientOptions {
  connection: EventConnectionPort;
  fetch?: typeof fetch;
  openSocket?: (url: string) => SocketLike;
  graceMs?: number;
  backlogLimit?: number;
  maxRecoveryAttempts?: number;
  random?: () => number;
  onEpochChanged?: (epoch: string) => void;
  onStreamError?: (streamId: string, error: string) => void;
}

export interface EventClient {
  subscribe(streamId: string, listener: StreamListener, options?: SubscribeOptions): () => void;
  close(): void;
  /** 시험용: 스트림의 재연결 cursor(수신자 반영 완료 최솟값). */
  debugCursor(streamId: string): number | undefined;
  /** 진단용: 열린 소켓을 모두 닫는다 — 연결 끊김과 같은 재연결 경로를 탄다. */
  debugDropSockets(): number;
}

const INITIAL_BACKOFF_MS = 250;
const MAX_BACKOFF_MS = 10_000;

interface ListenerState {
  listener: StreamListener;
  delivered: number;
  lastQueued: number;
  queue: EventEnvelope[];
  busy: boolean;
  resetting: boolean;
  removed: boolean;
}

interface Recovery {
  after: number;
  buffer: EventEnvelope[];
  terminal: boolean;
  helloSeen: boolean;
}

export function createEventClient(options: EventClientOptions): EventClient {
  const fetchImpl = options.fetch ?? fetch;
  const openSocket = options.openSocket ?? ((url: string) => new WebSocket(url) as unknown as SocketLike);
  const graceMs = options.graceMs ?? 500;
  const backlogLimit = options.backlogLimit ?? 1_024;
  const maxRecoveryAttempts = options.maxRecoveryAttempts ?? 3;
  const random = options.random ?? Math.random;
  const { connection } = options;
  const streams = new Map<string, Stream>();
  let closed = false;

  async function issueTicket(cursors: StreamCursor[]): Promise<string> {
    const attempt = async () => {
      const { baseUrl, token } = connection.credentials();
      return fetchImpl(`${baseUrl}/v1/event-tickets`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
        body: JSON.stringify({ cursors }),
      });
    };
    let response = await attempt();
    if (response.status === 401) {
      await connection.refreshCredentials();
      response = await attempt();
    }
    if (!response.ok) {
      throw new Error(`event ticket failed with status ${response.status}`);
    }
    const body = (await response.json()) as { ticket: string };
    return body.ticket;
  }

  function socketUrl(ticket: string) {
    const { baseUrl } = connection.credentials();
    return `${baseUrl.replace(/^http/, "ws")}/v1/events?ticket=${encodeURIComponent(ticket)}`;
  }

  class Stream {
    epoch: string;
    cursorWhenEmpty: number;
    highest: number;
    listeners = new Set<ListenerState>();
    backlog: EventEnvelope[] = [];
    suspended = false;
    socket: SocketLike | undefined;
    opening = false;
    failures = 0;
    recovery: Recovery | undefined;
    recoveryAttempts = 0;
    terminal = false;
    snapshot: SnapshotSource | undefined;
    resyncOnReconnect = false;
    resyncOnStart = false;
    connections = 0;
    graceTimer: ReturnType<typeof setTimeout> | undefined;
    reconnectTimer: ReturnType<typeof setTimeout> | undefined;

    constructor(readonly id: string, after: number, snapshot?: SnapshotSource) {
      this.epoch = connection.epoch() ?? "";
      this.cursorWhenEmpty = after;
      this.highest = after;
      this.snapshot = snapshot;
    }

    cursor(): number {
      if (this.listeners.size === 0) {
        return this.cursorWhenEmpty;
      }
      let min = Number.POSITIVE_INFINITY;
      for (const state of this.listeners) {
        min = Math.min(min, state.delivered);
      }
      return min;
    }

    add(listener: StreamListener, after: number | undefined, snapshot?: SnapshotSource) {
      if (snapshot && !this.snapshot) {
        this.snapshot = snapshot;
      }
      if (this.graceTimer !== undefined) {
        clearTimeout(this.graceTimer);
        this.graceTimer = undefined;
      }
      const joinAt = after ?? (this.listeners.size === 0 ? this.cursorWhenEmpty : this.highest);
      const state: ListenerState = {
        listener,
        delivered: joinAt,
        lastQueued: joinAt,
        queue: [],
        busy: false,
        resetting: false,
        removed: false,
      };
      const firstListener = this.listeners.size === 0;
      this.listeners.add(state);
      if (firstListener && this.backlog.length > 0) {
        this.distribute(this.backlog.splice(0));
      }
      if (this.suspended) {
        this.suspended = false;
        this.connect();
      } else if (!this.socket && !this.opening) {
        this.connect();
      } else if (state.delivered < this.highest && !this.recovery) {
        // 이미 지난 순번부터 받겠다는 수신자: cursor(최솟값)에서 다시 연결한다(다른 수신자는 lastQueued로 중복을 건너뛴다).
        this.reconnectNow();
      }
      return () => this.remove(state);
    }

    remove(state: ListenerState) {
      if (state.removed) {
        return;
      }
      state.removed = true;
      this.listeners.delete(state);
      if (this.listeners.size > 0) {
        return;
      }
      // 마지막 수신자: 반영하지 못한 대기분을 대기열로 돌려 다음 수신자가 이어 받게 한다.
      this.cursorWhenEmpty = state.delivered;
      this.backlog = state.queue.splice(0).concat(this.backlog).filter((event) => event.sequence > state.delivered);
      this.graceTimer = setTimeout(() => {
        this.graceTimer = undefined;
        if (this.listeners.size === 0) {
          this.dispose();
        }
      }, graceMs);
    }

    dispose() {
      this.terminal = true;
      this.closeSocket();
      if (this.reconnectTimer !== undefined) {
        clearTimeout(this.reconnectTimer);
      }
      streams.delete(this.id);
    }

    closeSocket() {
      const socket = this.socket;
      this.socket = undefined;
      if (socket) {
        socket.onclose = null;
        socket.onmessage = null;
        socket.close();
      }
    }

    reconnectNow() {
      this.closeSocket();
      this.connect();
    }

    connect() {
      if (closed || this.terminal || this.opening) {
        return;
      }
      this.opening = true;
      const cursor = this.recovery ? this.recovery.after : this.cursor();
      const epoch = this.epoch;
      void issueTicket([{ streamId: this.id, epoch, afterSequence: cursor }])
        .then((ticket) => {
          this.opening = false;
          if (closed || this.terminal) {
            return;
          }
          const socket = openSocket(socketUrl(ticket));
          this.socket = socket;
          socket.onmessage = (message) => this.onFrame(socket, JSON.parse(message.data) as EventFrame);
          socket.onclose = () => {
            if (this.socket === socket) {
              this.socket = undefined;
              this.scheduleReconnect();
            }
          };
          socket.onerror = () => undefined;
        })
        .catch(async () => {
          this.opening = false;
          connection.reportLost();
          await connection.whenConnected();
          this.scheduleReconnect();
        });
    }

    scheduleReconnect() {
      if (closed || this.terminal || this.suspended || this.listeners.size === 0 && !this.recovery) {
        return;
      }
      this.failures += 1;
      if (this.failures === 1) {
        queueMicrotask(() => this.connect());
        return;
      }
      const base = Math.min(INITIAL_BACKOFF_MS * 2 ** (this.failures - 2), MAX_BACKOFF_MS);
      const delay = Math.min(MAX_BACKOFF_MS, base * (0.8 + 0.4 * random()));
      this.reconnectTimer = setTimeout(() => {
        this.reconnectTimer = undefined;
        this.connect();
      }, delay);
    }

    onFrame(socket: SocketLike, frame: EventFrame) {
      if (this.socket !== socket) {
        return;
      }
      switch (frame.type) {
        case "hello":
          this.failures = 0;
          this.connections += 1;
          if (
            !this.recovery &&
            ((this.resyncOnReconnect && this.connections > 1) || (this.resyncOnStart && this.connections === 1))
          ) {
            for (const state of this.listeners) {
              void this.resetListener(state);
            }
          }
          if (this.recovery && !this.recovery.helloSeen) {
            this.recovery.helloSeen = true;
            void this.completeRecovery(this.recovery);
          }
          return;
        case "event":
          this.onEvent(frame.event);
          return;
        case "gap":
          this.onGap(frame);
          return;
        case "fault":
          options.onStreamError?.(this.id, frame.fault.message);
          this.terminal = true;
          this.closeSocket();
          return;
        default:
          return;
      }
    }

    onEvent(event: EventEnvelope) {
      if (event.sequence > this.highest) {
        this.highest = event.sequence;
      }
      if (this.recovery) {
        this.recovery.buffer.push(event);
        return;
      }
      if (this.listeners.size === 0) {
        const last = this.backlog[this.backlog.length - 1]?.sequence ?? this.cursorWhenEmpty;
        if (event.sequence > last) {
          this.backlog.push(event);
        }
        if (this.backlog.length > backlogLimit) {
          this.backlog = [];
          this.suspended = true;
          this.closeSocket();
        }
        return;
      }
      this.distribute([event]);
    }

    distribute(events: EventEnvelope[]) {
      for (const state of this.listeners) {
        for (const event of events) {
          if (event.sequence > state.lastQueued) {
            state.queue.push(event);
            state.lastQueued = event.sequence;
          }
        }
        void this.pump(state);
      }
    }

    async pump(state: ListenerState) {
      if (state.busy || state.resetting || state.removed) {
        return;
      }
      state.busy = true;
      try {
        while (state.queue.length > 0 && !state.resetting && !state.removed) {
          const event = state.queue[0];
          try {
            await state.listener.onEvent(event);
          } catch {
            state.busy = false;
            void this.resetListener(state);
            return;
          }
          state.queue.shift();
          state.delivered = Math.max(state.delivered, event.sequence);
        }
      } finally {
        state.busy = false;
      }
    }

    /** 수신자 하나의 재동기: 스냅샷을 불러 `onReset` → 스냅샷이 덮은 순번 뒤만 이어서. */
    async resetListener(state: ListenerState, attempt = 0): Promise<void> {
      state.resetting = true;
      const coveredUpTo = this.highest;
      try {
        if (!this.snapshot) {
          // 스냅샷이 없는 스트림(알림): 실패한 이벤트는 건너뛴다 — 다음 알림이 다시 읽게 한다.
          state.queue.shift();
        } else {
          const data = await this.snapshot.load();
          await state.listener.onReset?.(data, { delivered: state.delivered });
          const snapshot = this.snapshot;
          state.queue = state.queue.filter((event) => snapshot.passes(event, data));
          state.delivered = Math.max(state.delivered, coveredUpTo);
        }
      } catch (error) {
        if (attempt + 1 >= maxRecoveryAttempts) {
          options.onStreamError?.(this.id, `listener resync failed: ${String(error)}`);
        }
        const delay = Math.min(MAX_BACKOFF_MS, INITIAL_BACKOFF_MS * 2 ** attempt);
        setTimeout(() => void this.resetListener(state, attempt + 1), delay);
        return;
      }
      state.resetting = false;
      void this.pump(state);
    }

    onGap(gap: GapNotice & { type: "gap" }) {
      switch (gap.reason) {
        case "subscriberLagged":
        case "shutdown":
          this.reconnectNow();
          return;
        case "epochChanged":
          this.epoch = gap.epoch;
          options.onEpochChanged?.(gap.epoch);
          this.startRecovery(0, false);
          return;
        case "evicted":
          this.startRecovery(this.highest, true);
          return;
        default:
          this.epoch = gap.epoch;
          this.startRecovery(gap.lastSequence ?? 0, false);
      }
    }

    startRecovery(after: number, terminal: boolean) {
      this.recoveryAttempts += 1;
      if (this.recoveryAttempts > maxRecoveryAttempts) {
        this.recovery = undefined;
        this.terminal = true;
        this.closeSocket();
        options.onStreamError?.(this.id, "stream recovery failed repeatedly");
        return;
      }
      this.recovery = { after, buffer: [], terminal, helloSeen: false };
      this.closeSocket();
      if (terminal) {
        void this.completeRecovery(this.recovery);
        return;
      }
      this.connect();
    }

    async completeRecovery(recovery: Recovery) {
      let data: unknown;
      try {
        data = this.snapshot ? await this.snapshot.load() : undefined;
      } catch (error) {
        if (this.recovery === recovery) {
          options.onStreamError?.(this.id, `snapshot failed: ${String(error)}`);
          this.startRecovery(recovery.after, recovery.terminal);
        }
        return;
      }
      if (this.recovery !== recovery) {
        return; // 그 사이 새 gap으로 절차가 다시 시작됐다
      }
      for (const state of this.listeners) {
        state.resetting = true;
      }
      await Promise.all(
        [...this.listeners].map(async (state) => {
          try {
            await state.listener.onReset?.(data, { delivered: state.delivered });
          } catch {
            // 재설정 실패: 그 수신자만 다시 재동기(아래 pump 전에 resetListener가 이어 받는다).
            state.queue = [];
            state.delivered = state.lastQueued = recovery.after;
            state.resetting = false;
            void this.resetListener(state);
            return;
          }
          state.queue = [];
          state.delivered = state.lastQueued = Math.max(recovery.after, 0);
          state.resetting = false;
        }),
      );
      if (this.recovery !== recovery) {
        return;
      }
      this.recovery = undefined;
      this.recoveryAttempts = 0;
      const snapshot = this.snapshot;
      const pending = recovery.buffer.filter((event) => (snapshot ? snapshot.passes(event, data) : true));
      if (recovery.terminal) {
        this.terminal = true;
        this.distribute(pending);
        return;
      }
      this.cursorWhenEmpty = recovery.after;
      this.distribute(pending);
    }
  }

  return {
    subscribe(streamId, listener, subscribeOptions = {}) {
      let stream = streams.get(streamId);
      if (!stream || stream.terminal) {
        stream = new Stream(streamId, subscribeOptions.after ?? 0, subscribeOptions.snapshot);
        streams.set(streamId, stream);
      }
      if (subscribeOptions.resyncOnReconnect) {
        stream.resyncOnReconnect = true;
      }
      if (subscribeOptions.resyncOnStart) {
        stream.resyncOnStart = true;
      }
      return stream.add(listener, subscribeOptions.after, subscribeOptions.snapshot);
    },
    close() {
      closed = true;
      for (const stream of [...streams.values()]) {
        stream.dispose();
      }
    },
    debugCursor(streamId) {
      return streams.get(streamId)?.cursor();
    },
    debugDropSockets() {
      let dropped = 0;
      for (const stream of streams.values()) {
        if (stream.socket) {
          stream.socket.close(); // onclose 처리기가 남아 있어 재연결한다
          dropped += 1;
        }
      }
      return dropped;
    },
  };
}
