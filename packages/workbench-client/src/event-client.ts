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
  /** 재연결 cursor 기여분: 이 순번까지는 이벤트로 반영했거나, 적용 중인 스냅샷이 덮는다(그 앞을 다시 받을 필요 없음). */
  delivered: number;
  /** 수신자가 실제로 반영을 마친 순번(재설정 context로 넘긴다 — run은 그 뒤 스냅샷 이벤트를 다시 반영한다). 끝난
   *  onReset은 세대가 지났어도 반영한 것이므로 올린다. */
  applied: number;
  lastQueued: number;
  queue: EventEnvelope[];
  busy: boolean;
  resetting: boolean;
  removed: boolean;
  /** 재동기마다 늘린다. 재동기 전에 시작된 onEvent가 늦게 끝나면 새 대기열·cursor를 건드리지 않는다. */
  generation: number;
  /** 마지막 재동기 스냅샷 기준: 스냅샷이 이미 반영한 이벤트는 넘기지 않고 cursor만 올린다(다음 재동기까지). */
  covered?: (event: EventEnvelope) => boolean;
  /** 이 수신자의 스냅샷 **적용**(onReset·상태 변경) 사슬. 적재는 사슬 밖에서 하므로 끝나지 않는 옛 적재가 새 재동기를
   *  막지 않는다. 적용 차례가 왔을 때 더 새 재동기가 시작돼 있으면 콜백·상태 변경 없이 끝난다. */
  resync: Promise<void>;
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
    /** 표를 받는 중에 더 앞선 cursor가 필요해졌다(수신자 합류): 받은 표를 버리고 새 cursor로 다시 연다. */
    reopen = false;
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
        applied: joinAt,
        lastQueued: joinAt,
        queue: [],
        busy: false,
        resetting: false,
        removed: false,
        generation: 0,
        resync: Promise.resolve(),
      };
      if (this.recovery) {
        // gap 복구 중 합류: 복구가 끝나면 복구 스냅샷으로 재설정한 뒤 버퍼·live를 이어 받는다.
        state.resetting = true;
      }
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
      } else if (state.lastQueued < this.highest && !this.recovery && this.opening) {
        this.reopen = true;
      } else if (state.lastQueued < this.highest && !this.recovery) {
        // 대기열로 받지 못한 지난 순번부터 받겠다는 수신자: cursor(최솟값)에서 다시 연결한다(다른 수신자는 lastQueued로
        // 중복을 건너뛴다). 대기열을 넘겨받아 최고 순번까지 받은 수신자는 다시 연결하지 않는다.
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
          if (this.reopen) {
            this.reopen = false;
            this.connect();
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
            this.snapshot &&
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
        // 복구 중 재연결은 기준점부터 다시 재생한다: 순번이 늘어나는 이벤트만 버퍼에 둔다.
        const buffer = this.recovery.buffer;
        if (event.sequence > (buffer[buffer.length - 1]?.sequence ?? this.recovery.after)) {
          buffer.push(event);
        }
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
          if (state.covered?.(event)) {
            state.queue.shift();
            state.delivered = Math.max(state.delivered, event.sequence);
            state.applied = Math.max(state.applied, event.sequence);
            continue;
          }
          const generation = state.generation;
          let failed = false;
          try {
            await state.listener.onEvent(event);
          } catch {
            failed = true;
          }
          if (generation !== state.generation) {
            continue; // 그 사이 재동기됐다: 대기열·cursor는 재동기가 정했다
          }
          if (failed) {
            state.busy = false;
            void this.resetListener(state);
            return;
          }
          state.queue.shift();
          state.delivered = Math.max(state.delivered, event.sequence);
          state.applied = Math.max(state.applied, event.sequence);
        }
      } finally {
        state.busy = false;
      }
    }

    /** 수신자 하나의 재동기: 스냅샷을 불러 `onReset` → 스냅샷이 덮은 순번 뒤만 이어서. */
    /** 스냅샷 적용을 수신자의 사슬에 잇는다. 앞선 적용이 끝난 뒤 이 세대가 아직 최신일 때만 `work`를 돈다. */
    serialize(state: ListenerState, generation: number, work: () => Promise<void>): Promise<void> {
      const run = state.resync.then(async () => {
        if (this.isStale(state, generation)) {
          return;
        }
        await work();
      });
      state.resync = run.catch(() => undefined);
      return run;
    }

    isStale(state: ListenerState, generation: number) {
      return state.removed || generation !== state.generation;
    }

    /** 수신자 하나의 재동기: 스냅샷을 불러 `onReset` → 스냅샷이 덮은 순번 뒤만 이어서. 더 새 재동기가 시작되면 이 작업은
     *  스냅샷을 적용하지 않고 끝난다(늦게 온 옛 스냅샷이 새 상태를 덮지 않는다). */
    async resetListener(state: ListenerState, attempt = 0): Promise<void> {
      if (state.removed || closed || this.terminal) {
        return;
      }
      state.resetting = true;
      state.generation += 1;
      const generation = state.generation;
      try {
        const snapshot = this.snapshot;
        const coveredUpTo = this.highest;
        // 적재는 사슬 밖: 옛 재동기의 적재가 끝나지 않아도 이 재동기는 제 스냅샷을 불러 적용한다.
        const data = snapshot ? await snapshot.load() : undefined;
        if (this.isStale(state, generation)) {
          return;
        }
        await this.serialize(state, generation, async () => {
          if (!snapshot) {
            // 스냅샷이 없는 스트림(알림): 실패한 이벤트는 건너뛴다 — 다음 알림이 다시 읽게 한다.
            state.queue.shift();
          } else {
            await state.listener.onReset?.(data, { delivered: state.applied });
            state.applied = Math.max(state.applied, coveredUpTo);
            if (this.isStale(state, generation)) {
              return;
            }
            state.covered = (event) => !snapshot.passes(event, data);
            state.queue = state.queue.filter((event) => snapshot.passes(event, data));
            state.delivered = Math.max(state.delivered, coveredUpTo);
          }
          state.resetting = false;
          void this.pump(state);
        });
      } catch (error) {
        if (this.isStale(state, generation) || closed || this.terminal) {
          return;
        }
        if (attempt + 1 >= maxRecoveryAttempts) {
          options.onStreamError?.(this.id, `listener resync failed: ${String(error)}`);
        }
        const delay = Math.min(MAX_BACKOFF_MS, INITIAL_BACKOFF_MS * 2 ** attempt);
        setTimeout(() => void this.resetListener(state, attempt + 1), delay);
      }
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
          this.forgetEpochSequences();
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

    /** 세대가 바뀌면 옛 세대의 순번은 새 세대에서 뜻이 없다: 반영 cursor·대기열·최고 순번을 모두 0으로 되돌린다. 뒤이은
     *  gap·재시도·재설정도 이 값에서 시작하므로 옛 cursor를 다시 쓰지 않는다. 진행 중인 전달은 세대를 올려 무효로 한다. */
    forgetEpochSequences() {
      this.highest = 0;
      this.cursorWhenEmpty = 0;
      this.backlog = [];
      for (const state of this.listeners) {
        state.generation += 1;
        state.delivered = 0;
        state.applied = 0;
        state.lastQueued = 0;
        state.queue = [];
        state.covered = undefined;
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
      const snapshot = this.snapshot;
      const pending = recovery.buffer.filter((event) => (snapshot ? snapshot.passes(event, data) : true));
      // 복구 완료는 어떤 수신자의 onReset도 기다리지 않는다: 수신자마다 버퍼를 대기열에 먼저 받고(그 뒤 live는 이어 붙는다),
      // 스냅샷 적용은 수신자별 사슬에서 돈다. 한 수신자의 재설정이 끝나지 않거나 그 수신자가 떠나도 다른 수신자는 막히지 않는다.
      this.recovery = undefined;
      this.recoveryAttempts = 0;
      // gap 기준점까지는 스트림에 있었다: 그보다 앞선 cursor로 합류하는 수신자는 다시 구독해 자기 스냅샷을 받는다.
      this.highest = Math.max(this.highest, recovery.after);
      if (recovery.terminal) {
        this.terminal = true;
      } else {
        this.cursorWhenEmpty = recovery.after;
      }
      for (const state of this.listeners) {
        this.applyRecoverySnapshot(state, recovery, data, pending);
      }
    }

    applyRecoverySnapshot(state: ListenerState, recovery: Recovery, data: unknown, pending: EventEnvelope[]) {
      const snapshot = this.snapshot;
      const after = Math.max(recovery.after, 0);
      state.resetting = true;
      state.generation += 1;
      const generation = state.generation;
      state.covered = undefined;
      // 재연결 cursor는 곧바로 기준점: 복구 스냅샷이 그 앞을 덮으므로, 재설정 중 재연결해도 같은 gap 복구를 다시 일으키지
      // 않는다. 재설정 context는 실제 반영 순번(`applied`)을 적용 차례에 읽는다. 대기열 중복 방지는 기준점·버퍼 기준이다.
      state.delivered = after;
      state.queue = [];
      state.lastQueued = after;
      for (const event of pending) {
        if (event.sequence > state.lastQueued) {
          state.queue.push(event);
          state.lastQueued = event.sequence;
        }
      }
      void this.serialize(state, generation, async () => {
        await state.listener.onReset?.(data, { delivered: state.applied });
        state.applied = Math.max(state.applied, after);
        if (this.isStale(state, generation)) {
          return;
        }
        state.covered = snapshot ? (event) => !snapshot.passes(event, data) : undefined;
        state.resetting = false;
        void this.pump(state);
      }).catch(() => {
        if (this.isStale(state, generation)) {
          return;
        }
        // 재설정 실패: 그 수신자만 다시 재동기한다(대기열은 그 재동기가 스냅샷 기준으로 거른다).
        state.resetting = false;
        void this.resetListener(state);
      });
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
