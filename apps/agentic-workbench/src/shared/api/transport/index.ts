// 창의 transport(043 T024·T026). 부팅이 한 번 정하고(`app/bootstrap-transport.ts`), 저장소는 이 `invoke`만 쓴다.
// 정하기 전 기본값은 호환 경로다 — 네트워크 경로로 부팅하지 않은 창과 시험은 오늘과 같다.
import { compatTransport } from "./compat-transport";
import type { EventCallback, InvokeOptions, Transport } from "./transport";

let current: Transport = compatTransport;

export function setTransport(transport: Transport) {
  current = transport;
}

export function getTransport(): Transport {
  return current;
}

export function invoke<T>(command: string, args?: Record<string, unknown>, options?: InvokeOptions): Promise<T> {
  return current.invoke<T>(command, args, options);
}

export function listen<T>(event: string, callback: EventCallback<T>): Promise<() => void> {
  return current.listen<T>(event, callback);
}

export type { EventCallback, InvokeOptions, Transport } from "./transport";
export { compatTransport } from "./compat-transport";
export { MESSAGE_NOT_APPLIED, MESSAGE_RESULT_UNKNOWN, unsettledCall } from "./call-outcome";
