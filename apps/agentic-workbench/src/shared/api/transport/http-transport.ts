// 네트워크 경로(043 T024): command 표(`command-table.ts`)로 Workbench operation을 부르고, 결과·오류를 호환 경로와 같은
// 모양으로 돌려준다. 작업대 id는 데스크톱 앱이 창 주체로 연 것을 `ensure_window_bench`로 받는다.
import { faultToString, type WorkbenchClient } from "@yoophi/workbench-client";

import { COMMANDS } from "./command-table";
import type { InvokeOptions, Transport } from "./transport";

export const MESSAGE_NOT_APPLIED = "Workbench 서버에 연결되어 있지 않아 요청을 보내지 않았습니다.";
export const MESSAGE_RESULT_UNKNOWN = "Workbench 서버 연결이 끊겨 요청 결과를 알 수 없습니다. 상태를 다시 불러옵니다.";

export interface HttpTransportOptions {
  client: WorkbenchClient;
  /** `open`이면 없을 때 연다. 아니면 있을 때만(없으면 null). */
  ensureWindowBench: (open: boolean, hint?: string | null) => Promise<string | null>;
  windowLabel: string;
}

export function createHttpTransport(options: HttpTransportOptions): Transport {
  const { client, ensureWindowBench, windowLabel } = options;

  async function run(command: string, args: Record<string, unknown>, invokeOptions: InvokeOptions) {
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
  };
}
