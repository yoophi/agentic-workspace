// 네트워크 경로 호출이 서버에서 끝났는지 알 수 없는 두 결과(043 호출 결과 규칙, `@yoophi/workbench-client` call-client):
// - notApplied: 연결이 끊겨 보내지 않았다(서버 상태는 그대로다).
// - unknown: 보낸 뒤 답을 받지 못했다(서버가 적용했는지 모른다 — 실제 상태는 복구된 이벤트로 다시 맞춘다).
// 화면은 이 두 경우 "서버가 거절했다"와 다르게 다룬다: 지금 보이는 상태를 버리지 않는다(Codex r8).
export const MESSAGE_NOT_APPLIED = "Workbench 서버에 연결되어 있지 않아 요청을 보내지 않았습니다.";
export const MESSAGE_RESULT_UNKNOWN = "Workbench 서버 연결이 끊겨 요청 결과를 알 수 없습니다. 상태를 다시 불러옵니다.";

/** 호출 오류가 서버의 답이 아니면 그 종류, 서버가 답한 오류(fault)나 다른 오류면 null. */
export function unsettledCall(error: unknown): "notApplied" | "unknown" | null {
  if (error === MESSAGE_NOT_APPLIED) {
    return "notApplied";
  }
  if (error === MESSAGE_RESULT_UNKNOWN) {
    return "unknown";
  }
  return null;
}
