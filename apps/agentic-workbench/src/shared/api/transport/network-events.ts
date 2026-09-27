// 네트워크 경로의 이벤트(043 T034, research R7·R8). 화면은 오늘과 같은 이벤트 이름·payload로 `listen`하고, 이 계층이 창이
// 알게 된 작업대·run·orchestration 묶임·worktree를 Workbench 스트림 구독으로 바꾼다(호환 경로의 창 삽입 대신 — 창은
// `declare_network_delivery`로 삽입을 끈다). payload는 core의 데스크톱 전달과 같은 모양이다:
// run `{runId, event, sequence, epoch, streamId, eventId}`, 교환·orchestration `{...body, sequence, epoch, streamId, eventId}`,
// 제목 `{title}`, worktree `{...body, workingDirectory}`.
// 재동기 스냅샷: run `run.replay`(순번 기준), 교환 `exchange.list`(모두 넘김 — 화면 원장이 requestId로 맞춤), orchestration
// `orchestration.get`(revision 기준, 재설정은 revision 알림으로 — 화면이 다시 읽는다).
import type { EventClient, EventEnvelope, SnapshotSource, WorkbenchClient } from "@yoophi/workbench-client";
import { WORKTREE_CHANGED_EVENT } from "@yoophi/workspace-auto-refresh";

export const RUN_EVENT = "agent-run-event";
export const EXCHANGE_REQUESTED_EVENT = "agent-exchange-requested";
export const EXCHANGE_STATUS_EVENT = "agent-exchange-status";
export const ORCHESTRATION_WORKSPACE_UPDATED_EVENT = "orchestration-workspace-updated";
export const ORCHESTRATION_COMMAND_UPDATED_EVENT = "orchestration-command-updated";
export const ORCHESTRATION_NOTIFICATION_UPDATED_EVENT = "orchestration-coordinator-notification-updated";
export const TITLE_EVENT = "workspace://mcp-window-title";
export { WORKTREE_CHANGED_EVENT };

const SCHEMA_EXCHANGE_REQUESTED = "exchange.requested.v1";
const SCHEMA_EXCHANGE_STATUS = "exchange.status.v1";
const SCHEMA_TITLE = "bench.titleRequested.v1";

type Callback = (payload: unknown) => void | Promise<void>;
type Body = Record<string, unknown>;

function positioned(event: EventEnvelope) {
  return {
    ...((event.body ?? {}) as Body),
    sequence: event.sequence,
    epoch: event.epoch,
    streamId: event.streamId,
    eventId: event.eventId,
  };
}

function runPayload(runId: string, event: EventEnvelope) {
  return {
    runId,
    event: event.body,
    sequence: event.sequence,
    epoch: event.epoch,
    streamId: event.streamId,
    eventId: event.eventId,
  };
}

/** orchestration 사유 → 상세 이벤트(AW `tauri_desktop_bridge::orchestration_detail_event`와 같은 규칙). */
export function orchestrationDetailEvent(reason: string): string | null {
  if (reason.includes("command") || reason.includes("Command")) {
    return ORCHESTRATION_COMMAND_UPDATED_EVENT;
  }
  if (reason.includes("notification") || reason.includes("Notification")) {
    return ORCHESTRATION_NOTIFICATION_UPDATED_EVENT;
  }
  return null;
}

interface Registration {
  name: string;
  callback: Callback;
  /** 스트림 id → 구독 해제 */
  subscriptions: Map<string, () => void>;
}

export interface NetworkEventsOptions {
  events: EventClient;
  client: WorkbenchClient;
}

export function createNetworkEvents({ events, client }: NetworkEventsOptions) {
  const registrations = new Set<Registration>();
  let benchId: string | undefined;
  let orchestrationStream: string | undefined;
  let worktreePath: string | undefined;
  const runs = new Set<string>();

  async function call(operation: string, input: unknown) {
    const outcome = await client.call(operation as never, input as never);
    if (outcome.kind !== "ok") {
      throw new Error(outcome.kind === "fault" ? outcome.fault.message : outcome.kind);
    }
    return outcome.output;
  }

  function runSnapshot(runId: string): SnapshotSource {
    return {
      load: () => call("run.replay", { benchId, runId, afterSequence: 0 }),
      passes: (event, data) => event.sequence > ((data as { lastSequence?: number }).lastSequence ?? 0),
    };
  }

  // 교환 병합(R8·T040): 스냅샷에 같은 requestId가 있으면 스냅샷보다 늦은 상태만 넘긴다(updatedAt). 요청 이벤트는
  // 스냅샷에서 아직 확인 전(`accepted`)일 때만 — 이미 종결된 교환의 옛 요청을 다시 라우팅하지 않는다.
  const exchangeSnapshot: SnapshotSource = {
    load: () => call("exchange.list", { benchId }),
    passes: (event, data) => exchangePasses(event, data),
  };

  const orchestrationSnapshot: SnapshotSource = {
    load: () => call("orchestration.get", { benchId }),
    passes: (event, data) => {
      const revision = (data as { revision?: number } | null)?.revision;
      const next = (event.body as { revision?: number } | null)?.revision;
      return revision === undefined || next === undefined || next > revision;
    },
  };

  /** 등록 하나가 들어야 할 스트림과 그 수신자. */
  function wanted(registration: Registration): Map<string, Parameters<EventClient["subscribe"]>> {
    const map = new Map<string, Parameters<EventClient["subscribe"]>>();
    const { name, callback } = registration;
    if (name === RUN_EVENT && benchId) {
      for (const runId of runs) {
        const streamId = `run:${runId}`;
        map.set(streamId, [
          streamId,
          {
            onEvent: (event) => callback(runPayload(runId, event)),
            onReset: async (data, { delivered }) => {
              const replay = data as { events?: Array<{ sequence: number; event: unknown }>; epoch?: string };
              for (const item of replay.events ?? []) {
                if (item.sequence > delivered) {
                  await callback({ runId, event: item.event, sequence: item.sequence, streamId });
                }
              }
            },
          },
          { snapshot: runSnapshot(runId) },
        ]);
      }
    }
    if ((name === EXCHANGE_REQUESTED_EVENT || name === EXCHANGE_STATUS_EVENT) && benchId) {
      const schema = name === EXCHANGE_REQUESTED_EVENT ? SCHEMA_EXCHANGE_REQUESTED : SCHEMA_EXCHANGE_STATUS;
      const streamId = `exchange:${benchId}`;
      map.set(streamId, [
        streamId,
        {
          onEvent: (event) => (event.schema === schema ? callback(positioned(event)) : undefined),
          onReset: async (data) => {
            const list = (data as Array<Body & { status: string }>) ?? [];
            for (const item of list) {
              if (name === EXCHANGE_STATUS_EVENT) {
                await callback(item);
              } else if (item.status === "accepted") {
                // 요청 이벤트를 잃었을 수 있다: 확인 전 교환을 요청으로 다시 넘긴다(화면 원장이 한 번만 라우팅·확인).
                const { requestId, source, target, message, delivery, createdAt } = item;
                await callback({ requestId, source, target, message, delivery, createdAt });
              }
            }
          },
        },
        { snapshot: exchangeSnapshot },
      ]);
    }
    if (
      (name === ORCHESTRATION_WORKSPACE_UPDATED_EVENT ||
        name === ORCHESTRATION_COMMAND_UPDATED_EVENT ||
        name === ORCHESTRATION_NOTIFICATION_UPDATED_EVENT) &&
      orchestrationStream
    ) {
      const streamId = orchestrationStream;
      map.set(streamId, [
        streamId,
        {
          onEvent: (event) => {
            const payload = positioned(event);
            if (name === ORCHESTRATION_WORKSPACE_UPDATED_EVENT) {
              return callback(payload);
            }
            const detail = orchestrationDetailEvent(String((payload as Body).reason ?? ""));
            return detail === name ? callback(payload) : undefined;
          },
          onReset: async (data) => {
            const session = data as { id?: string; revision?: number } | null;
            if (session?.id && name === ORCHESTRATION_WORKSPACE_UPDATED_EVENT) {
              // 재설정 = 현재 revision 알림: 화면은 revision이 앞서면 작업 영역을 다시 읽는다.
              await callback({ workspaceId: session.id, revision: session.revision, reason: "resync" });
            }
          },
        },
        { snapshot: orchestrationSnapshot },
      ]);
    }
    if (name === TITLE_EVENT && benchId) {
      const streamId = `bench:${benchId}`;
      map.set(streamId, [
        streamId,
        {
          onEvent: (event) =>
            event.schema === SCHEMA_TITLE ? callback({ title: (event.body as Body).title }) : undefined,
        },
        {},
      ]);
    }
    if (name === WORKTREE_CHANGED_EVENT && worktreePath) {
      const path = worktreePath;
      const streamId = `worktree:${path}`;
      map.set(streamId, [
        streamId,
        {
          onEvent: (event) => callback({ ...((event.body ?? {}) as Body), workingDirectory: path }),
          // 알림 스트림 재조회(R8): 끊긴 동안 놓친 변경이 있을 수 있어 다시 연결되면 전체 다시 읽기 신호를 보낸다
          // (`kind: "git"`은 화면에서 파일 목록·변경·Git 이력을 모두 무효화한다).
          onReset: () => callback(worktreeResync(path)),
        },
        { snapshot: WORKTREE_SNAPSHOT, resyncOnReconnect: true },
      ]);
    }
    return map;
  }

  /** 등록마다 원하는 스트림과 실제 구독을 맞춘다(새로 알게 된 run·작업대·묶임). */
  function sync(registration: Registration) {
    const target = wanted(registration);
    for (const [streamId, unsubscribe] of registration.subscriptions) {
      if (!target.has(streamId)) {
        unsubscribe();
        registration.subscriptions.delete(streamId);
      }
    }
    for (const [streamId, args] of target) {
      if (!registration.subscriptions.has(streamId)) {
        registration.subscriptions.set(streamId, events.subscribe(...args));
      }
    }
  }

  function syncAll() {
    for (const registration of registrations) {
      sync(registration);
    }
  }

  return {
    listen(name: string, callback: Callback): Promise<() => void> {
      const registration: Registration = { name, callback, subscriptions: new Map() };
      registrations.add(registration);
      sync(registration);
      return Promise.resolve(() => {
        registrations.delete(registration);
        for (const unsubscribe of registration.subscriptions.values()) {
          unsubscribe();
        }
        registration.subscriptions.clear();
      });
    },
    noteBench(id: string) {
      if (benchId !== id) {
        benchId = id;
        syncAll();
      }
    },
    noteRuns(ids: Iterable<string>) {
      let changed = false;
      for (const id of ids) {
        if (!runs.has(id)) {
          runs.add(id);
          changed = true;
        }
      }
      if (changed) {
        syncAll();
      }
    },
    noteOrchestrationStream(streamId: string) {
      if (orchestrationStream !== streamId) {
        orchestrationStream = streamId;
        syncAll();
      }
    },
    watchWorktree(path: string) {
      worktreePath = path;
      syncAll();
    },
    unwatchWorktree() {
      worktreePath = undefined;
      syncAll();
    },
    /** 세대가 바뀌면 작업대·run·묶임 정보가 무효다(새로 열린 작업대로 다시 알게 된다). */
    resetForEpoch() {
      benchId = undefined;
      orchestrationStream = undefined;
      runs.clear();
      syncAll();
    },
  };
}

export type NetworkEvents = ReturnType<typeof createNetworkEvents>;

/** worktree 재조회 신호(화면의 전체 무효화 경로를 탄다). */
export function worktreeResync(path: string) {
  return { workingDirectory: path, kind: "git", reason: "resync" };
}

/** worktree에는 읽어 올 서버 상태가 없다 — 재설정 자체가 재조회 신호다. */
const WORKTREE_SNAPSHOT: SnapshotSource = { load: async () => null, passes: () => true };

export function exchangePasses(event: EventEnvelope, data: unknown): boolean {
  const body = (event.body ?? {}) as { requestId?: string; updatedAt?: string };
  const list = (Array.isArray(data) ? data : []) as Array<{ requestId: string; status: string; updatedAt: string }>;
  const current = list.find((item) => item.requestId === body.requestId);
  if (!current) {
    return true;
  }
  if (event.schema === SCHEMA_EXCHANGE_REQUESTED) {
    return current.status === "accepted";
  }
  return Date.parse(body.updatedAt ?? "") > Date.parse(current.updatedAt);
}

/** 결과·인자에서 run id를 모은다(run 시작 결과 `id`, `runId`·`currentRunId` 등 `*RunId` 필드). */
export function collectRunIds(value: unknown, into = new Set<string>(), depth = 0): Set<string> {
  if (depth > 8 || value === null || typeof value !== "object") {
    return into;
  }
  if (Array.isArray(value)) {
    for (const item of value) {
      collectRunIds(item, into, depth + 1);
    }
    return into;
  }
  for (const [key, item] of Object.entries(value as Body)) {
    if (typeof item === "string" && item && /(^runId$|RunId$)/.test(key)) {
      into.add(item);
    } else if (typeof item === "object") {
      collectRunIds(item, into, depth + 1);
    }
  }
  return into;
}
