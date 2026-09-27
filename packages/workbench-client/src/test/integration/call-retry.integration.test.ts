// 043 T042(SC-004b): 응답을 잃은 변경의 재시도를 실제 042 서버(시험 host, prompt 효과 뒤 응답 전 지연 `HOST_PROMPT_SETTLE_MS`)
// 에서 확인한다. 요청을 끊는 시점은 효과 표식(run replay에 그 prompt의 AgentMessage가 나타남)으로 잡는다 — 시간 추정이 아니다.
// - 같은 세대: 클라이언트는 같은 멱등성 키로 한 번 재시도해 저장된 결과를 받고, 효과는 한 번뿐이다.
// - 새 세대(서버 재기동, 같은 데이터·다른 포트): 클라이언트는 새 끝점을 찾아 연결하지만 그 변경을 새 서버로 보내지 않는다.
import { afterEach, describe, expect, it, vi } from "vitest";

import { createWorkbenchClient } from "../../call-client";
import type { OperationId } from "../../operation-map";

import { connectTo, startHost, type Host, type HostInfo } from "./host";

let cleanup: Array<() => Promise<void> | void> = [];
afterEach(async () => {
  for (const step of cleanup.reverse()) {
    await step();
  }
  cleanup = [];
});

async function raw(host: HostInfo, operation: string, input: unknown, key?: string) {
  const response = await fetch(`${host.baseUrl}/v1/calls`, {
    method: "POST",
    headers: { authorization: `Bearer ${host.tokens.windowA}`, "content-type": "application/json" },
    body: JSON.stringify({ protocolVersion: 1, operation, requestId: crypto.randomUUID(), input, ...(key ? { idempotencyKey: key } : {}) }),
  });
  const body = (await response.json()) as { output?: unknown };
  return body.output as Record<string, unknown>;
}

/** run replay에서 본문에 `text`가 든 AgentMessage 수(효과 표식). */
async function effects(host: HostInfo, bench: string, text: string) {
  const replay = (await raw(host, "run.replay", { benchId: bench, runId: "r1", afterSequence: 0 })) as {
    events: Array<{ event: { type?: string; text?: string } }>;
  };
  return replay.events.filter((item) => item.event.type === "agentMessage" && item.event.text === text).length;
}

type Sent = { url: string; operation?: string; key?: string };

/** 첫 run.sendPrompt를 효과 표식이 보인 뒤 끊는 fetch. 보낸 요청을 기록한다. */
function interruptingFetch(onEffect: () => Promise<boolean>, afterEffect?: () => Promise<void>) {
  const sent: Sent[] = [];
  let interrupted = false;
  const wrapped = (async (url: string | URL | Request, init?: RequestInit) => {
    const body = init?.body ? (JSON.parse(String(init.body)) as { operation?: string; idempotencyKey?: string }) : {};
    sent.push({ url: String(url), operation: body.operation, key: body.idempotencyKey });
    if (body.operation !== "run.sendPrompt" || interrupted) {
      return fetch(url, init);
    }
    interrupted = true;
    const controller = new AbortController();
    const pending = fetch(url, { ...init, signal: controller.signal });
    // 서버가 죽으면 끊기 전에 먼저 실패한다 — 호출부에 돌려주기 전이라 처리 표시를 미리 붙인다(호출부는 같은 Promise로 받는다).
    pending.catch(() => undefined);
    await vi.waitFor(async () => expect(await onEffect()).toBe(true), { timeout: 10_000, interval: 20 });
    await afterEffect?.();
    controller.abort();
    return pending;
  }) as typeof fetch;
  return { wrapped, sent };
}

describe("real 042 server: lost mutation responses", () => {
  it("retries once with the same key in the same epoch and applies the effect once", async () => {
    const host = await startHost({ HOST_PROMPT_SETTLE_MS: "1500" });
    cleanup.push(() => host.stop());
    const bench = (await raw(host, "bench.open", { workingDirectory: host.workDir }, crypto.randomUUID())).benchId as string;
    await raw(host, "run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId: "r1" } }, crypto.randomUUID());
    const target = { current: host as HostInfo };
    const connection = await connectTo(target);
    cleanup.push(() => connection.close());
    const { wrapped, sent } = interruptingFetch(async () => (await effects(host, bench, "retry-me")) === 1);
    const client = createWorkbenchClient({ connection, fetch: wrapped });

    const outcome = await client.call("run.sendPrompt" as OperationId, { benchId: bench, runId: "r1", prompt: "retry-me" } as never);
    expect(outcome.kind).toBe("ok");
    const prompts = sent.filter((item) => item.operation === "run.sendPrompt");
    expect(prompts).toHaveLength(2);
    expect(prompts[1].key).toBe(prompts[0].key);
    expect(await effects(host, bench, "retry-me")).toBe(1);
  });

  it("finds the restarted server on a new port but never resends the lost mutation to the new epoch", async () => {
    const first = await startHost({ HOST_PROMPT_SETTLE_MS: "3000" });
    let second: Host | undefined;
    cleanup.push(async () => {
      await second?.stop();
      if (first.process.exitCode === null) {
        await first.stop();
      }
    });
    const bench = (await raw(first, "bench.open", { workingDirectory: first.workDir }, crypto.randomUUID())).benchId as string;
    await raw(first, "run.start", { benchId: bench, request: { goal: "g", agentId: "codex", runId: "r1" } }, crypto.randomUUID());
    const target = { current: first as HostInfo };
    const connection = await connectTo(target);
    cleanup.push(() => connection.close());
    const { wrapped, sent } = interruptingFetch(
      async () => (await effects(first, bench, "lost-on-restart")) === 1,
      async () => {
        // 효과 뒤·응답 전에 서버가 죽고, 같은 데이터로 새 세대가 다른 포트에 뜬다.
        first.process.kill("SIGKILL");
        await new Promise((done) => first.process.once("exit", done));
        second = await startHost({ HOST_DATA_DIR: first.dataDir });
        target.current = second;
      },
    );
    const client = createWorkbenchClient({ connection, fetch: wrapped });

    const outcome = await client.call("run.sendPrompt" as OperationId, { benchId: bench, runId: "r1", prompt: "lost-on-restart" } as never);
    expect(outcome).toEqual({ kind: "unknown", reason: "epochChanged" });
    expect(second).toBeDefined();
    expect(second?.epoch).not.toBe(first.epoch);
    expect(connection.epoch()).toBe(second?.epoch);
    expect(connection.credentials().baseUrl).toBe(second?.baseUrl);
    const toNewServer = sent.filter((item) => item.operation === "run.sendPrompt" && item.url.startsWith(second?.baseUrl ?? "-"));
    expect(toNewServer).toHaveLength(0);
  });
});
