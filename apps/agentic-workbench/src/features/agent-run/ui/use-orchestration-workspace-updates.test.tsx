// @vitest-environment happy-dom
// 043 T038(화면 통합): 운영 orchestration 갱신 구독(`useOrchestrationWorkspaceUpdates` — `worktree-agent-run-area`가 쓰는
// 바로 그 hook)을 네트워크 경로로 렌더링하고, 화면에 그려진 작업 영역 revision으로 복구를 확인한다. 호출은 실제 루프백
// HTTP의 가짜 Workbench 서버(→ call client → HttpTransport), 이벤트는 가짜 hub 스트림(042 cursor 규칙)으로 온다.
// - Promise 거절: 수신자의 다시 읽기가 실패하면 이벤트 클라이언트가 이 수신자를 스냅샷(revision 알림)으로 재동기하고,
//   화면은 최신 revision을 그린다.
// - 수신자 교체: 화면 상태(revision 2)를 유지한 채 구독자만 내렸다 다시 올린다. 구독자가 없는 사이 도착한 revision 3은
//   대기열로 새 구독자에게 넘어가고, **운영 hook을 통해서만** 화면이 3이 된다(화면은 다시 마운트하지 않아 스스로 읽지 않는다).
// 운영 수신자는 async라 내부 예외는 모두 거절된 Promise가 된다 — 동기 예외 경로는 이 수신자에 없고, 이벤트 클라이언트
// 시험(T030 "treats a synchronous throw like a rejection")이 맡는다.
import { act, useEffect, useRef, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";

import { getOrchestrationWorkspace, type OrchestrationSession } from "@/entities/agent-orchestration";
import { compatTransport, setTransport } from "@/shared/api/transport";
import {
  startCompatSimulatingServer,
  type CompatSimulatingServer,
} from "@/shared/api/transport/testing/compat-simulating-server";

import { useOrchestrationWorkspaceUpdates } from "./use-orchestration-workspace-updates";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let revision = 1;
let failNextRefetch = 0;
const fetches: string[] = [];

function session(): OrchestrationSession {
  return {
    id: "ws1",
    revision,
    worktreePath: "/wt",
    mainNodeId: "main-agent-run",
    boundWindowLabel: "session-test",
    nodes: [],
    generations: [],
    tasks: [],
    reports: [],
  } as unknown as OrchestrationSession;
}

async function fakeInvoke(command: string) {
  if (command === "get_orchestration_workspace") {
    fetches.push(`get@${revision}`);
    if (failNextRefetch > 0) {
      failNextRefetch -= 1;
      throw "orchestration workspace fetch failed";
    }
    return session();
  }
  throw new Error(`unexpected command ${command}`);
}

/** 운영 hook을 쓰는 최소 화면: 처음 한 번 읽고, 이후는 hook이 갱신한다. `subscribed`가 거짓이면 구독자만 내린다. */
function OrchestrationRevision({ subscribed = true }: { subscribed?: boolean }) {
  const [current, setCurrent] = useState<OrchestrationSession | null>(null);
  const ref = useRef<OrchestrationSession | null>(null);
  ref.current = current;
  useEffect(() => {
    void getOrchestrationWorkspace().then((snapshot) => snapshot && setCurrent(snapshot));
  }, []);
  useOrchestrationWorkspaceUpdates(ref, setCurrent, subscribed ? "/wt" : null);
  return <div data-testid="revision">{current ? `revision:${current.revision}` : "none"}</div>;
}

let server: CompatSimulatingServer;

beforeAll(async () => {
  server = await startCompatSimulatingServer((command) => fakeInvoke(command));
  setTransport(server.transport);
});

afterAll(async () => {
  setTransport(compatTransport);
  await server.close();
});

function mount() {
  const container = document.createElement("div");
  document.body.append(container);
  const root: Root = createRoot(container);
  act(() => root.render(<OrchestrationRevision />));
  return { container, root };
}

const STREAM = "orchestration:test-binding";
const publish = (next: number) =>
  server.hub.publish(STREAM, { workspaceId: "ws1", revision: next, reason: "child" }, "orchestration.workspaceUpdated.v1");

describe("orchestration updates on the network path render recovered screen state", () => {
  it("recovers after a rejected refetch and after a receiver replacement", async () => {
    const first = mount();
    await vi.waitFor(() => expect(first.container.textContent).toBe("revision:1"));
    await vi.waitFor(() => expect(server.hub.sockets.some((socket) => socket.readyState === 1)).toBe(true));

    // Promise 거절: 다시 읽기가 한 번 실패한다 → 재동기(스냅샷 revision 알림) → 다시 읽기 → 화면 revision 2.
    revision = 2;
    failNextRefetch = 1;
    await act(async () => void publish(2));
    await vi.waitFor(() => expect(first.container.textContent).toBe("revision:2"), { timeout: 2_000 });
    expect(failNextRefetch).toBe(0);
    expect(fetches.filter((entry) => entry === "get@2").length).toBeGreaterThanOrEqual(2); // 실패한 읽기 + 복구 읽기

    // 수신자 교체: 화면 상태(revision 2)는 유지하고 구독자만 내린다 → 그 사이 revision 3 도착 → 구독자를 다시 올린다.
    const fetchesBefore = fetches.length;
    act(() => first.root.render(<OrchestrationRevision subscribed={false} />));
    revision = 3;
    await act(async () => void publish(3));
    expect(first.container.textContent).toBe("revision:2"); // 구독자가 없는 동안은 바뀌지 않는다
    act(() => first.root.render(<OrchestrationRevision subscribed />));
    // 대기열로 넘겨받은 revision 3 알림 → 운영 hook이 다시 읽어 화면이 3이 된다.
    await vi.waitFor(() => expect(first.container.textContent).toBe("revision:3"), { timeout: 2_000 });
    expect(fetches.slice(fetchesBefore)).toContain("get@3");

    // 교체 뒤에도 이어서 받는다.
    revision = 4;
    await act(async () => void publish(4));
    await vi.waitFor(() => expect(first.container.textContent).toBe("revision:4"), { timeout: 2_000 });
    act(() => first.root.unmount());
  }, 15_000);
});
