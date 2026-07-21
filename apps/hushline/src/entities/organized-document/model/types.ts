// 정리 실행 상태. 정리 문서의 저장 형태(payload)는 공유 API의 OrganizedDocumentInput이 단일 소스다.
export type OrganizeStatus = "idle" | "running" | "done" | "error" | "cancelled";
