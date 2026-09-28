// 화면의 서버 소유 호출 경로(043 T024, research R3). 창이 뜰 때 한 번 정한 transport 하나로 모든 호출이 간다 —
// 호환 경로(Tauri command)와 네트워크 경로(Workbench HTTP)는 같은 command 이름·인자·결과·오류 문자열을 쓴다.
export interface InvokeOptions {
  /** 사용자 조작 하나에 묶인 멱등성 키(예: `exchange-delivery:<requestId>`). 호환 경로는 무시한다. */
  idempotencyKey?: string;
  /** 같은 epoch command의 저장된 성공을 재생한 응답인지 관찰한다. compat 경로에는 이 메타데이터가 없다. */
  onReply?: (reply: { replayed: boolean }) => void;
}

/** 이벤트 수신자. 네트워크 경로는 Promise를 settle까지 기다려 반영 완료로 친다(research R7). */
export type EventCallback<T> = (payload: T) => void | Promise<void>;

export interface Transport {
  readonly kind: "compat" | "http";
  invoke<T>(command: string, args?: Record<string, unknown>, options?: InvokeOptions): Promise<T>;
  /** 오늘과 같은 이벤트 이름·payload로 받는다. */
  listen<T>(event: string, callback: EventCallback<T>): Promise<() => void>;
}
