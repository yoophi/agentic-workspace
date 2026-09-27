// orchestration 작업 영역 갱신 구독(043 T038로 `worktree-agent-run-area`에서 옮김 — 동작은 그대로). 갱신 알림의 revision이
// 화면 상태보다 앞서면 작업 영역을 다시 읽는다. 수신자는 async다: 다시 읽기가 실패하면 Promise가 거절되고, 네트워크
// 경로의 이벤트 클라이언트는 이 수신자를 스트림 스냅샷(revision 알림)으로 재동기한다(research R7·R8).
import { useEffect, type MutableRefObject } from "react";

import {
  getOrchestrationWorkspace,
  listenOrchestrationWorkspaceUpdated,
  type OrchestrationSession,
} from "@/entities/agent-orchestration";

export function useOrchestrationWorkspaceUpdates(
  sessionRef: MutableRefObject<OrchestrationSession | null>,
  onSnapshot: (session: OrchestrationSession) => void,
  /** 구독 대상. 바뀌면 구독자를 교체하고, `null`이면 구독하지 않는다(운영은 Worktree 경로를 넘긴다). */
  resetKey: unknown,
) {
  useEffect(() => {
    if (resetKey === null) {
      return;
    }
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listenOrchestrationWorkspaceUpdated(async (event) => {
      const current = sessionRef.current;
      if (
        disposed ||
        (current && event.workspaceId !== current.id) ||
        (current && event.revision <= current.revision)
      ) {
        return;
      }
      const snapshot = await getOrchestrationWorkspace();
      if (!disposed && snapshot) {
        onSnapshot(snapshot);
      }
    }).then((dispose) => {
      if (disposed) {
        dispose();
      } else {
        unlisten = dispose;
      }
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
    // onSnapshot은 상태 setter처럼 안정적인 함수를 넘긴다. 구독은 대상(resetKey)이 바뀔 때만 다시 연다.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resetKey]);
}
