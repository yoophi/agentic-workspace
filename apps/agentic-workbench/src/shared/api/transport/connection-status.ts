// 네트워크 경로 연결 상태(043 T048, research R10). 부팅이 연결의 상태 변화를 알리고, 표시 컴포넌트가 구독한다. 호환 경로
// 창은 늘 `connected`로 둔다(표시 없음).
export type ConnectionStatus = "connecting" | "connected" | "reconnecting" | "disconnected";

let status: ConnectionStatus = "connected";
const listeners = new Set<() => void>();

export function setConnectionStatus(next: ConnectionStatus) {
  if (status === next) {
    return;
  }
  status = next;
  for (const listener of listeners) {
    listener();
  }
}

export function getConnectionStatus(): ConnectionStatus {
  return status;
}

export function subscribeConnectionStatus(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
