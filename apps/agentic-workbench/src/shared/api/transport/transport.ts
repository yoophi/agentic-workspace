// 화면의 서버 소유 호출 경로(043 T024, research R3). 창이 뜰 때 한 번 정한 transport 하나로 모든 호출이 간다 —
// 호환 경로(Tauri command)와 네트워크 경로(Workbench HTTP)는 같은 command 이름·인자·결과·오류 문자열을 쓴다.
export interface InvokeOptions {
  /** 사용자 조작 하나에 묶인 멱등성 키(예: `exchange-delivery:<requestId>`). 호환 경로는 무시한다. */
  idempotencyKey?: string;
}

export interface Transport {
  readonly kind: "compat" | "http";
  invoke<T>(command: string, args?: Record<string, unknown>, options?: InvokeOptions): Promise<T>;
}
