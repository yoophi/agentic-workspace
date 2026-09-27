// 043 T049(US4, SC-004a): 실제 042 서버에서 창 B의 자격 증명으로 창 A의 작업대를 조작·구독하면 서버 판정으로 거절된다 —
// TS 클라이언트의 호출(fault)과 이벤트 구독(fault 프레임 → 스트림 오류) 경로 모두. 창 A는 계속 쓸 수 있다.
// 토큰 폐기(창 Destroyed)는 core 시험 `http_window_tokens`(T005)가 운영 발급기로 확인한다.
import { afterEach, describe, expect, it, vi } from "vitest";

import { createWorkbenchClient } from "../../call-client";
import { createEventClient } from "../../event-client";
import type { OperationId } from "../../operation-map";

import { connectTo, startHost, type HostInfo } from "./host";

let cleanup: Array<() => Promise<void> | void> = [];
afterEach(async () => {
  for (const step of cleanup.reverse()) {
    await step();
  }
  cleanup = [];
});

describe("real 042 server: window isolation through the TS client", () => {
  it("rejects another window's calls and subscriptions on a bench it does not own", async () => {
    const host = await startHost();
    cleanup.push(() => host.stop());
    const target = { current: host as HostInfo };
    const a = await connectTo(target, "windowA");
    const b = await connectTo(target, "windowB");
    cleanup.push(() => a.close(), () => b.close());
    const clientA = createWorkbenchClient({ connection: a });
    const clientB = createWorkbenchClient({ connection: b });
    const opened = await clientA.call("bench.open" as OperationId, { workingDirectory: host.workDir } as never);
    if (opened.kind !== "ok") {
      throw new Error(JSON.stringify(opened));
    }
    const bench = (opened.output as { benchId: string }).benchId;

    for (const [operation, input] of [
      ["exchange.list", { benchId: bench }],
      ["orchestration.get", { benchId: bench }],
      ["run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId: "rb" } }],
      ["bench.close", { benchId: bench }],
    ] as const) {
      const outcome = await clientB.call(operation as OperationId, input as never);
      expect(outcome.kind, operation).toBe("fault");
      expect(outcome.kind === "fault" && outcome.fault.code, operation).toBe("forbidden");
    }

    const errors: string[] = [];
    const eventsB = createEventClient({ connection: b, onStreamError: (_stream, error) => errors.push(error) });
    cleanup.push(() => eventsB.close());
    eventsB.subscribe(`exchange:${bench}`, { onEvent: () => undefined });
    await vi.waitFor(() => expect(errors).toEqual(["bench belongs to another principal."]), { timeout: 10_000 });

    const own = await clientA.call("exchange.list" as OperationId, { benchId: bench } as never);
    expect(own.kind).toBe("ok");
  });
});
