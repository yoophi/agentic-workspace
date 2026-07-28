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
};

export type SequencedRuntimeEvent = RuntimeEventSnapshot["events"][number];

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
  };
}

export function applyRuntimeSnapshot(
  state: AgentRunControllerState,
  snapshot: RuntimeEventSnapshot,
): AgentRunControllerState {
  if (snapshot.runId !== state.runId || snapshot.lastSequence < state.lastSequence) {
    return state;
  }
  const events = snapshot.events
    .filter((event) => event.sequence > state.lastSequence)
    .sort((left, right) => left.sequence - right.sequence);
  return {
    ...state,
    lastSequence: Math.max(state.lastSequence, snapshot.lastSequence),
    terminal: snapshot.terminal,
    hydrationStatus: snapshot.gapDetected ? "gap" : "ready",
    events: [...state.events, ...events.map((event) => event.event)],
  };
}

export function applyLiveRuntimeEvent(
  state: AgentRunControllerState,
  event: SequencedRuntimeEvent,
): AgentRunControllerState {
  if (event.runId !== state.runId || event.sequence <= state.lastSequence) return state;
  return {
    ...state,
    lastSequence: event.sequence,
    terminal: event.terminal,
    hydrationStatus:
      state.hydrationStatus === "gap" ? "gap" : "ready",
    events: [...state.events, event.event],
  };
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
    this.update({ ...this.state, hydrationStatus: "runtimeLost" });
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
