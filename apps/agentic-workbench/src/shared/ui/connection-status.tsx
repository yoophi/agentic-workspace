// 연결 상태 표시(043 T048): 네트워크 경로 창이 Workbench 서버와 끊겼을 때만 보인다. 연결돼 있으면 아무것도 그리지 않는다
// (기존 화면 배치·문구는 그대로). 재연결은 자동으로 계속한다.
import { useSyncExternalStore } from "react";

import {
  getConnectionStatus,
  subscribeConnectionStatus,
  type ConnectionStatus as Status,
} from "@/shared/api/transport/connection-status";

export const CONNECTION_STATUS_MESSAGES: Partial<Record<Status, string>> = {
  reconnecting: "Workbench 서버에 다시 연결하는 중입니다…",
  disconnected: "Workbench 서버에 연결할 수 없습니다. 계속 다시 시도합니다.",
};

export function ConnectionStatusView({ status }: { status: Status }) {
  const message = CONNECTION_STATUS_MESSAGES[status];
  if (!message) {
    return null;
  }
  return (
    <div
      role="status"
      aria-live="polite"
      data-connection-status={status}
      className="pointer-events-none fixed bottom-3 left-1/2 z-50 -translate-x-1/2 rounded-md border bg-background/95 px-3 py-1.5 text-xs text-muted-foreground shadow-sm"
    >
      {message}
    </div>
  );
}

export function ConnectionStatus() {
  const status = useSyncExternalStore(subscribeConnectionStatus, getConnectionStatus, getConnectionStatus);
  return <ConnectionStatusView status={status} />;
}
