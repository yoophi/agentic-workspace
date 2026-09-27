// 네트워크 경로(043 T024): command 표(`command-table.ts`)로 Workbench operation을 부르고, 결과·오류를 호환 경로와 같은
// 모양으로 돌려준다. 작업대 id는 데스크톱 앱이 창 주체로 연 것을 `ensure_window_bench`로 받는다.
import { faultToString, type WorkbenchClient } from "@yoophi/workbench-client";

import { COMMANDS } from "./command-table";
import { collectRunIds, type NetworkEvents } from "./network-events";
import type { InvokeOptions, Transport } from "./transport";

export const MESSAGE_NOT_APPLIED = "Workbench 서버에 연결되어 있지 않아 요청을 보내지 않았습니다.";
export const MESSAGE_RESULT_UNKNOWN = "Workbench 서버 연결이 끊겨 요청 결과를 알 수 없습니다. 상태를 다시 불러옵니다.";

export interface HttpTransportOptions {
  client: WorkbenchClient;
  /** `open`이면 없을 때 연다. 아니면 있을 때만(없으면 null). */
  ensureWindowBench: (open: boolean, hint?: string | null) => Promise<string | null>;
  windowLabel: string;
  /** 네트워크 경로 이벤트. 호출 결과에서 알게 된 작업대·run·orchestration 묶임을 알린다. */
  events?: NetworkEvents;
}

export function createHttpTransport(options: HttpTransportOptions): Transport {
  const { client, windowLabel, events } = options;

  async function ensureWindowBench(open: boolean, hint?: string | null) {
    const benchId = await options.ensureWindowBench(open, hint);
    if (benchId) {
      events?.noteBench(benchId);
    }
    return benchId;
  }

  function observe(args: Record<string, unknown>, output: unknown) {
    if (!events) {
      return;
    }
    const streamId = (output as { eventStreamId?: unknown } | null)?.eventStreamId;
    if (typeof streamId === "string") {
      events.noteOrchestrationStream(streamId);
    }
    const runIds = collectRunIds(args);
    collectRunIds(output, runIds);
    const started = (output as { id?: unknown; agentId?: unknown } | null) ?? null;
    if (started && typeof started.id === "string" && typeof started.agentId === "string") {
      runIds.add(started.id); // run.start 결과(AgentRun)
    }
    events.noteRuns(runIds);
  }

  async function run(command: string, args: Record<string, unknown>, invokeOptions: InvokeOptions) {
    // 네트워크 경로의 worktree 감시는 `worktree:<path>` 구독이다(구독이 서버 감시를 시작한다, 039).
    if (command === "start_worktree_watcher" && events) {
      events.watchWorktree(String(args.workingDirectory));
      return null;
    }
    if (command === "stop_worktree_watcher" && events) {
      events.unwatchWorktree();
      return null;
    }
    const spec = COMMANDS[command];
    if (!spec) {
      throw new Error(`${command} is not a server-owned command`);
    }
    let benchId: string | undefined;
    if (spec.bench.kind === "ensure") {
      benchId = (await ensureWindowBench(true, spec.bench.hint?.(args) ?? null)) ?? undefined;
    } else if (spec.bench.kind === "lookup") {
      const found = await ensureWindowBench(false);
      if (!found) {
        const missing = spec.bench.missing;
        if ("result" in missing) {
          return missing.result(args);
        }
        throw missing.error(args);
      }
      benchId = found;
    }
    const outcome = await client.call(
      spec.operation,
      spec.input(args, benchId) as never,
      invokeOptions.idempotencyKey ? { idempotencyKey: invokeOptions.idempotencyKey } : undefined,
    );
    switch (outcome.kind) {
      case "ok":
        observe(args, outcome.output);
        return spec.output ? spec.output(outcome.output, { windowLabel, args }) : outcome.output;
      case "fault":
        if (spec.onFault) {
          return spec.onFault(args);
        }
        throw faultToString(outcome.fault, spec.flavor);
      case "notApplied":
        if (spec.onFault) {
          return spec.onFault(args);
        }
        throw MESSAGE_NOT_APPLIED;
      case "unknown":
        if (spec.onFault) {
          return spec.onFault(args);
        }
        throw MESSAGE_RESULT_UNKNOWN;
    }
  }

  return {
    kind: "http",
    invoke: (command, args = {}, invokeOptions = {}) => run(command, args, invokeOptions) as Promise<never>,
    listen: (event, callback) => {
      if (!events) {
        return Promise.reject(new Error("network events are not configured"));
      }
      return events.listen(event, callback as (payload: unknown) => void | Promise<void>);
    },
  };
}
