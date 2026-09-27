import type { RuntimeEventSnapshot } from "@/entities/agent-orchestration";

export type RuntimeHydrationStatus =
  | "idle"
  | "loading"
  | "ready"
  | "gap"
  | "runtimeLost";

export type AgentRunControllerState = {
  nodeId: string;
  runId: string;
  lastSequence: number;
  terminal: boolean;
  hydrationStatus: RuntimeHydrationStatus;
  events: unknown[];
  /**
   * 재수화 전·중(`idle`·`loading`)에 도착한 live 이벤트. snapshot을 적용한 뒤 순번 순으로 비운다(039 research R7):
   * replay 응답보다 먼저 온 live가 `lastSequence`를 올려 snapshot 전체를 버리게 되는 경합을 막는다.
   */
  pendingLive: SequencedRuntimeEvent[];
};

export type SequencedRuntimeEvent = RuntimeEventSnapshot["events"][number];

/** 버퍼 상한: 서버의 run당 보관 한도와 같다. 넘치면 오래된 것부터 버리고 drain 때 빈틈으로 `gap`이 된다. */
export const PENDING_LIVE_LIMIT = 512;

export function createAgentRunControllerState(
  runId: string,
  nodeId = "",
): AgentRunControllerState {
  return {
    nodeId,
    runId,
    lastSequence: 0,
    terminal: false,
    hydrationStatus: "idle",
    events: [],
    pendingLive: [],
  };
}

function isHydrating(state: AgentRunControllerState) {
  return state.hydrationStatus === "idle" || state.hydrationStatus === "loading";
}

/** 한 건 적용. 중복(`<= lastSequence`)은 무시하고, 빈틈(`> lastSequence + 1`)이면 적용한 뒤 `gap`으로 표시한다. */
function applyOne(
  state: AgentRunControllerState,
  event: SequencedRuntimeEvent,
): AgentRunControllerState {
  if (event.sequence <= state.lastSequence) return state;
  const skipped = event.sequence > state.lastSequence + 1;
  const hydrationStatus: RuntimeHydrationStatus =
    state.hydrationStatus === "runtimeLost"
      ? "runtimeLost"
      : skipped || state.hydrationStatus === "gap"
        ? "gap"
        : "ready";
  return {
    ...state,
    lastSequence: event.sequence,
    terminal: event.terminal,
    hydrationStatus,
    events: [...state.events, event.event],
  };
}

/** 버퍼를 순번 순으로 적용하고 비운다. */
export function drainPendingLive(state: AgentRunControllerState): AgentRunControllerState {
  const pending = [...state.pendingLive].sort((left, right) => left.sequence - right.sequence);
  let next: AgentRunControllerState = { ...state, pendingLive: [] };
  for (const event of pending) next = applyOne(next, event);
  return next;
}

export function applyRuntimeSnapshot(
  state: AgentRunControllerState,
  snapshot: RuntimeEventSnapshot,
): AgentRunControllerState {
  if (snapshot.runId !== state.runId) return state;
  let next: AgentRunControllerState = state;
  if (snapshot.lastSequence >= state.lastSequence) {
    const events = snapshot.events
      .filter((event) => event.sequence > state.lastSequence)
      .sort((left, right) => left.sequence - right.sequence);
    next = {
      ...state,
      lastSequence: Math.max(state.lastSequence, snapshot.lastSequence),
      terminal: snapshot.terminal,
      hydrationStatus: snapshot.gapDetected ? "gap" : "ready",
      events: [...state.events, ...events.map((event) => event.event)],
    };
  } else if (isHydrating(state)) {
    next = { ...state, hydrationStatus: "ready" };
  }
  return drainPendingLive(next);
}

export function applyLiveRuntimeEvent(
  state: AgentRunControllerState,
  event: SequencedRuntimeEvent,
): AgentRunControllerState {
  if (event.runId !== state.runId) return state;
  if (isHydrating(state)) {
    if (state.pendingLive.some((pending) => pending.sequence === event.sequence)) return state;
    const pendingLive = [...state.pendingLive, event];
    while (pendingLive.length > PENDING_LIVE_LIMIT) pendingLive.shift();
    return { ...state, pendingLive };
  }
  return applyOne(state, event);
}

/** 재수화 실패: 버퍼의 이벤트는 적용하되 상태는 `runtimeLost`(유실 표시가 우선). */
export function failRuntimeHydration(state: AgentRunControllerState): AgentRunControllerState {
  return drainPendingLive({ ...state, hydrationStatus: "runtimeLost" });
}

export class AgentRunController {
  private state: AgentRunControllerState;
  private readonly listeners = new Set<(state: AgentRunControllerState) => void>();

  constructor(nodeId: string, runId: string) {
    this.state = createAgentRunControllerState(runId, nodeId);
  }

  get snapshot() {
    return this.state;
  }

  markLoading() {
    this.update({ ...this.state, hydrationStatus: "loading" });
  }

  markRuntimeLost() {
    this.update(failRuntimeHydration(this.state));
  }

  applySnapshot(snapshot: RuntimeEventSnapshot) {
    this.update(applyRuntimeSnapshot(this.state, snapshot));
  }

  applyLive(event: SequencedRuntimeEvent) {
    this.update(applyLiveRuntimeEvent(this.state, event));
  }

  subscribe(listener: (state: AgentRunControllerState) => void) {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private update(state: AgentRunControllerState) {
    if (state === this.state) return;
    this.state = state;
    for (const listener of this.listeners) listener(state);
  }
}

export class AgentRunControllerRegistry {
  private readonly controllers = new Map<string, AgentRunController>();

  getOrCreate(nodeId: string, runId: string) {
    const existing = this.controllers.get(runId);
    if (existing) return existing;
    const controller = new AgentRunController(nodeId, runId);
    this.controllers.set(runId, controller);
    return controller;
  }

  get(runId: string | null | undefined) {
    return runId ? this.controllers.get(runId) ?? null : null;
  }

  remove(runId: string) {
    this.controllers.delete(runId);
  }

  get size() {
    return this.controllers.size;
  }
}
