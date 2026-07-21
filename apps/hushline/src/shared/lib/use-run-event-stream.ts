import { useCallback, useEffect, useRef } from "react";

import type { LifecycleStatus } from "@yoophi/agent-client";

import { agent } from "../api";

export type RunEventHandlers = {
  onMessage: (text: string) => void;
  onLifecycle: (status: LifecycleStatus) => void;
  onError: (message: string) => void;
};

/**
 * agent run 이벤트 구독의 공통 골격: runId 필터링, 구독/해제 수명, envelope 라우팅을 담당한다.
 * 각 기능은 3개의 핸들러만 넘기고, `begin()`으로 구독을 시작한 뒤 `setRunId()`로 대상 run을 고정한다.
 */
export function useRunEventStream(handlers: RunEventHandlers) {
  const runIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  const teardown = useCallback(() => {
    unlistenRef.current?.();
    unlistenRef.current = null;
  }, []);

  useEffect(() => teardown, [teardown]);

  const begin = useCallback(() => {
    teardown();
    runIdRef.current = null;
    unlistenRef.current = agent.listenRunEvents((envelope) => {
      if (runIdRef.current && envelope.runId !== runIdRef.current) return;
      const event = envelope.event;
      if (event.type === "agentMessage") handlersRef.current.onMessage(event.text);
      else if (event.type === "error") handlersRef.current.onError(event.message);
      else if (event.type === "lifecycle") handlersRef.current.onLifecycle(event.status);
    });
  }, [teardown]);

  const setRunId = useCallback((id: string) => {
    runIdRef.current = id;
  }, []);

  return { runIdRef, begin, setRunId, teardown };
}
