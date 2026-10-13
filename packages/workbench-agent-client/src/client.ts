import {
  createConnection,
  createEventClient,
  createWorkbenchClient,
  faultToString,
  type CallOutcome,
  type ConnectionState,
  type EventEnvelope,
  type OperationMap,
} from "@yoophi/workbench-client";
import { applyRunEvent, newSession, readRunEvent } from "./conversation";
import {
  AgentChatError,
  type AgentChatClient,
  type AgentChatOptions,
  type AgentChatSnapshot,
  type ChatSession,
  type CommandOutcome,
  type SessionOptions,
} from "./types";

type Replay = OperationMap["run.replay"]["output"];
function asVoid<T>(outcome: CallOutcome<T>): CommandOutcome {
  return outcome.kind === "ok" ? { ...outcome, output: undefined } : outcome;
}
function wasNotApplied(outcome: CommandOutcome) {
  return (
    outcome.kind === "notApplied" ||
    (outcome.kind === "fault" && outcome.fault.outcome === "notApplied")
  );
}
function outcomeMessage(outcome: CommandOutcome): string {
  if (outcome.kind === "fault") return faultToString(outcome.fault);
  if (outcome.kind === "unknown")
    return "Command result is uncertain; the message was not resent.";
  if (outcome.kind === "notApplied")
    return "Disconnected; the command was not sent.";
  return "";
}

export function createAgentChatClient(
  options: AgentChatOptions,
): AgentChatClient {
  if (!options.workspace.trim())
    throw new AgentChatError("notConnected", "A workspace is required.");
  const sessions = new Map<string, ChatSession>();
  const subscriptions = new Map<string, () => void>();
  const listeners = new Set<(snapshot: AgentChatSnapshot) => void>();
  const pendingCommands = new Set<string>();
  const closeFlights = new Map<string, Promise<CommandOutcome>>();
  const creationFlights = new Map<string, Promise<void>>();
  let disposed = false;
  let benchId: string | undefined;
  let currentEpoch: string | undefined;
  let connectionState: ConnectionState = "connecting";
  let snapshot: AgentChatSnapshot = {
    connection: connectionState,
    sessions: [],
    disposed,
  };
  let connectFlight: Promise<CallOutcome<{ benchId: string }>> | undefined;
  let rejectDisposed!: () => void;
  const disposedPromise = new Promise<never>((_, reject) => {
    rejectDisposed = () =>
      reject(new AgentChatError("disposed", "Chat client is disposed."));
  });
  // The rejection is also observed when disposal happens with no pending command.
  void disposedPromise.catch(() => undefined);
  const active = () => {
    if (disposed)
      throw new AgentChatError("disposed", "Chat client is disposed.");
  };
  const awaitActive = <T>(promise: Promise<T>) =>
    Promise.race([promise, disposedPromise]);

  function notify() {
    snapshot = {
      connection: connectionState,
      benchId,
      sessions: [...sessions.values()],
      disposed,
    };
    for (const listener of listeners) {
      try {
        listener(snapshot);
      } catch {
        /* UI listener failure cannot roll back committed state/cursor. */
      }
    }
  }
  function put(session: ChatSession) {
    sessions.set(session.id, session);
    notify();
  }
  function get(id: string) {
    const session = sessions.get(id);
    if (!session)
      throw new AgentChatError("sessionMissing", "Chat session was not found.");
    return session;
  }
  function owner() {
    active();
    if (currentEpoch !== connection.epoch())
      throw new AgentChatError(
        "epochChanged",
        "Server identity changed; reconnect the workspace.",
      );
    if (!benchId)
      throw new AgentChatError("notConnected", "Connect the workspace first.");
    return benchId;
  }
  function changedEpoch(epoch: string) {
    if (disposed || epoch === currentEpoch) return;
    if (currentEpoch !== undefined) {
      benchId = undefined;
      for (const unsubscribe of subscriptions.values()) unsubscribe();
      subscriptions.clear();
      for (const session of sessions.values())
        sessions.set(session.id, {
          ...session,
          status: "lost",
          permissions: [],
          error: "Server restarted; the previous session is unavailable.",
        });
    }
    currentEpoch = epoch;
    // WS may observe a restart before an HTTP request does. Refresh the shared connection before opening a new bench.
    if (connection.epoch() !== epoch) connection.reportLost();
    notify();
  }
  const connection = createConnection({
    fetchConnection: options.connectionProvider,
    fetch: options.fetch,
  });
  const calls = createWorkbenchClient({ connection, fetch: options.fetch });
  const events = createEventClient({
    connection,
    fetch: options.fetch,
    openSocket: options.openSocket,
    onEpochChanged: changedEpoch,
    onStreamError: (streamId, error) => {
      if (disposed) return;
      const id = streamId.slice("run:".length);
      const session = sessions.get(id);
      if (session && session.status !== "lost" && session.status !== "closed")
        put({ ...session, streamError: error });
    },
  });
  const unlistenState = connection.onState((state) => {
    if (!disposed) {
      connectionState = state;
      notify();
    }
  });
  const unlistenEpoch = connection.onEpochChanged(changedEpoch);

  function checkEpoch(epoch: string | undefined) {
    active();
    if (currentEpoch !== epoch)
      throw new AgentChatError(
        "epochChanged",
        "Server restarted during the operation.",
      );
  }
  function ingest(id: string, sequence: number, body: unknown) {
    if (disposed || get(id).status === "lost") return;
    const session = get(id);
    const next = applyRunEvent(session, sequence, readRunEvent(body));
    if (next !== session) put(next);
  }
  function subscribeRun(id: string, bench: string, epoch: string | undefined) {
    const load = async (): Promise<Replay> => {
      checkEpoch(epoch);
      const result = await awaitActive(
        calls.call("run.replay", {
          benchId: bench,
          runId: id,
          afterSequence: 0,
        }),
      );
      checkEpoch(epoch);
      if (result.kind !== "ok") throw new Error(outcomeMessage(asVoid(result)));
      return result.output;
    };
    subscriptions.set(
      id,
      events.subscribe(
        `run:${id}`,
        {
          onEvent: (event: EventEnvelope) => {
            if (disposed || currentEpoch !== epoch || get(id).status === "lost")
              return;
            if (event.schema !== "run.event.v1")
              throw new Error("Unsupported run event schema");
            if (event.streamId !== `run:${id}` || event.epoch !== epoch)
              throw new Error("Run event identity does not match this session");
            ingest(id, event.sequence, event.body);
          },
          onReset: (raw, context) => {
            if (disposed || currentEpoch !== epoch || get(id).status === "lost")
              return;
            const replay = raw as Replay;
            if (replay.runId !== id)
              throw new Error("Replay identity does not match this session");
            for (const item of replay.events) {
              ingest(id, item.sequence, item.event);
              context.markApplied(item.sequence);
            }
            const session = get(id);
            put({
              ...session,
              streamError: undefined,
              lastSequence: Math.max(session.lastSequence, replay.lastSequence),
              historyIncomplete:
                session.historyIncomplete || replay.gapDetected,
              ...(replay.terminal && session.status !== "failed"
                ? { status: "closed", permissions: [] }
                : {}),
            });
            context.markApplied(replay.lastSequence);
          },
        },
        {
          resyncOnStart: true,
          snapshot: {
            load,
            passes: (event, replay) =>
              event.sequence > (replay as Replay).lastSequence,
            position: (replay) => (replay as Replay).lastSequence,
          },
        },
      ),
    );
  }

  /** An uncertain create may still be applying. Wait for server-owned run evidence before cancelling it. */
  async function waitForRunEvidence(id: string): Promise<boolean> {
    if (get(id).events.length > 0) return true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let listener: ((state: AgentChatSnapshot) => void) | undefined;
    const evidence = new Promise<boolean>((resolve) => {
      listener = () => {
        const session = get(id);
        if (session.events.length > 0 || session.status === "lost")
          resolve(session.status !== "lost");
      };
      listeners.add(listener);
      timer = setTimeout(() => resolve(false), 10_000);
      listener(snapshot);
    });
    try {
      return await awaitActive(evidence);
    } finally {
      if (timer !== undefined) clearTimeout(timer);
      if (listener) listeners.delete(listener);
    }
  }

  const api: AgentChatClient = {
    connect() {
      active();
      if (benchId && connectionState === "connected")
        return Promise.resolve({
          kind: "ok",
          output: { benchId },
          revision: undefined,
        });
      if (connectFlight) return connectFlight;
      connectFlight = (async () => {
        const handshake =
          currentEpoch === undefined
            ? await awaitActive(connection.start())
            : await awaitActive(connection.whenConnected());
        changedEpoch(handshake.serverEpoch);
        active();
        if (benchId)
          return {
            kind: "ok",
            output: { benchId },
            revision: undefined,
          } as const;
        const epoch = currentEpoch;
        const result = await awaitActive(
          calls.call("bench.open", { workingDirectory: options.workspace }),
        );
        active();
        if (epoch !== currentEpoch)
          return { kind: "unknown", reason: "epochChanged" } as const;
        if (result.kind === "ok") {
          benchId = result.output.benchId;
          notify();
        }
        return result;
      })().finally(() => {
        connectFlight = undefined;
      });
      return connectFlight;
    },
    async listAgents() {
      active();
      const result = await awaitActive(calls.call("agent.list", {}));
      active();
      return result;
    },
    async createSession(settings: SessionOptions) {
      const bench = owner();
      const epoch = currentEpoch;
      const id = settings.runId ?? crypto.randomUUID();
      if (!id.trim() || !settings.agentId.trim())
        throw new AgentChatError(
          "invalidMessage",
          "Agent and session id are required.",
        );
      if (sessions.has(id))
        throw new AgentChatError(
          "duplicateSession",
          "This session id is already in use.",
        );
      const {
        label = `${settings.agentId} chat`,
        runId: _runId,
        ...request
      } = settings;
      let releaseCreation!: () => void;
      creationFlights.set(
        id,
        new Promise<void>((resolve) => {
          releaseCreation = resolve;
        }),
      );
      put(newSession(id, settings.agentId, label));
      try {
        const called = await awaitActive(
          calls.call("run.createSession", {
            benchId: bench,
            request: {
              ...request,
              cwd: options.workspace,
              goal: label,
              runId: id,
            },
          }),
        );
        const result =
          called.kind === "ok"
            ? { ...called, output: { id: called.output.id } }
            : called;
        active();
        // Preserve the original command outcome; no transport fallback and no extra attempt.
        const session = get(id);
        if (currentEpoch !== epoch) {
          put({ ...session, creationOutcome: result });
          return result;
        }
        put({
          ...session,
          creationOutcome: result,
          ...(result.kind !== "ok"
            ? {
                status: wasNotApplied(asVoid(result)) ? "failed" : "uncertain",
                error: outcomeMessage(asVoid(result)),
              }
            : {}),
        });
        if (result.kind === "ok" || !wasNotApplied(asVoid(result)))
          subscribeRun(id, bench, epoch);
        return result;
      } finally {
        releaseCreation();
        creationFlights.delete(id);
      }
    },
    async send(id, text) {
      active();
      const session = get(id);
      if (
        session.status !== "ready" ||
        session.streamError ||
        pendingCommands.has(id)
      )
        throw new AgentChatError(
          "sessionNotReady",
          "Wait for the session to be ready before sending.",
        );
      const bench = owner();
      if (!text.trim())
        throw new AgentChatError("invalidMessage", "A message is required.");
      const messageId = crypto.randomUUID();
      const epoch = currentEpoch;
      pendingCommands.add(id);
      put({
        ...session,
        status: "busy",
        messages: [
          ...session.messages,
          { id: messageId, role: "user", text, delivery: { kind: "pending" } },
        ],
      });
      try {
        const result = asVoid(
          await awaitActive(
            calls.call("run.sendPrompt", {
              benchId: bench,
              runId: id,
              prompt: text,
            }),
          ),
        );
        active();
        const current = get(id);
        put({
          ...current,
          messages: current.messages.map((message) =>
            message.id === messageId
              ? { ...message, delivery: result }
              : message,
          ),
          ...(currentEpoch === epoch &&
          result.kind !== "ok" &&
          current.status !== "closed" &&
          current.status !== "failed"
            ? {
                status: wasNotApplied(result) ? "ready" : "uncertain",
                error: outcomeMessage(result),
              }
            : {}),
        });
        return result;
      } finally {
        pendingCommands.delete(id);
      }
    },
    async respondPermission(id, permissionId, optionId) {
      const bench = owner();
      const session = get(id);
      const permission = session.permissions.find(
        (item) => item.id === permissionId,
      );
      if (
        !permission ||
        session.status === "closing" ||
        session.status === "closed" ||
        session.status === "lost" ||
        permission.responding ||
        !permission.options.some((option) => option.optionId === optionId)
      ) {
        throw new AgentChatError(
          "invalidPermission",
          "Choose an option from this session's pending permission request.",
        );
      }
      const epoch = currentEpoch;
      put({
        ...session,
        permissions: session.permissions.map((item) =>
          item.id === permissionId ? { ...item, responding: true } : item,
        ),
      });
      const result = asVoid(
        await awaitActive(
          calls.call("run.respondPermission", {
            benchId: bench,
            runId: id,
            permissionId,
            optionId,
          }),
        ),
      );
      active();
      if (currentEpoch === epoch) {
        const current = get(id);
        put({
          ...current,
          permissions:
            result.kind === "ok"
              ? current.permissions.filter((item) => item.id !== permissionId)
              : current.permissions.map((item) =>
                  item.id === permissionId
                    ? {
                        ...item,
                        responding: !wasNotApplied(result),
                        outcome: result,
                      }
                    : item,
                ),
        });
      }
      return result;
    },
    closeSession(id) {
      active();
      const existing = closeFlights.get(id);
      if (existing) return existing;
      const session = get(id);
      if (session.status === "lost") {
        put({ ...session, status: "closed", permissions: [] });
        return Promise.resolve({
          kind: "ok",
          output: undefined,
          revision: undefined,
        });
      }
      if (session.status === "closed")
        return Promise.resolve({
          kind: "ok",
          output: undefined,
          revision: undefined,
        });
      const bench = owner();
      const epoch = currentEpoch;
      put({ ...session, status: "closing" });
      const flight = (async () => {
        const creation = creationFlights.get(id);
        if (creation) await awaitActive(creation);
        active();
        if (currentEpoch !== epoch)
          return { kind: "unknown", reason: "epochChanged" } as const;
        const created = get(id);
        if (
          created.creationOutcome &&
          wasNotApplied(asVoid(created.creationOutcome))
        ) {
          put({ ...created, status: "closed", permissions: [] });
          return {
            kind: "ok",
            output: undefined,
            revision: undefined,
          } as const;
        }
        if (
          created.creationOutcome?.kind !== "ok" &&
          !(await waitForRunEvidence(id))
        ) {
          const result = {
            kind: "unknown",
            reason: currentEpoch === epoch ? "lost" : "epochChanged",
          } as const;
          if (currentEpoch === epoch)
            put({
              ...get(id),
              status: "uncertain",
              closeOutcome: result,
              error:
                "Session creation is still uncertain; cancellation was not sent.",
            });
          return result;
        }
        if (currentEpoch !== epoch)
          return { kind: "unknown", reason: "epochChanged" } as const;
        if (get(id).status === "closed")
          return {
            kind: "ok",
            output: undefined,
            revision: undefined,
          } as const;
        const result = asVoid(
          await awaitActive(
            calls.call("run.cancel", { benchId: bench, runId: id }),
          ),
        );
        active();
        if (currentEpoch === epoch) {
          const current = get(id);
          if (result.kind === "ok") {
            put({
              ...current,
              status: "closed",
              permissions: [],
              closeOutcome: result,
            });
            subscriptions.get(id)?.();
            subscriptions.delete(id);
          } else {
            put({
              ...current,
              closeOutcome: result,
              ...(current.status === "closing"
                ? {
                    status: wasNotApplied(result)
                      ? session.status
                      : "uncertain",
                    error: outcomeMessage(result),
                  }
                : {}),
            });
          }
        }
        return result;
      })().finally(() => closeFlights.delete(id));
      closeFlights.set(id, flight);
      return flight;
    },
    subscribe(listener) {
      active();
      listeners.add(listener);
      try {
        listener(snapshot);
      } catch {
        /* Same isolation as subsequent notifications. */
      }
      return () => {
        listeners.delete(listener);
      };
    },
    getSnapshot: () => snapshot,
    dispose() {
      if (disposed) return;
      disposed = true;
      rejectDisposed();
      unlistenState();
      unlistenEpoch();
      for (const unsubscribe of subscriptions.values()) unsubscribe();
      subscriptions.clear();
      events.close();
      connection.close();
      notify();
      listeners.clear();
    },
  };
  return api;
}
