// 자막을 사용자가 지정한 방식으로 재구성한 정리 문서(백엔드 camelCase와 일치).
export type OrganizedDocument = {
  sourceUrl?: string | null;
  sourceTranscriptPath?: string | null;
  style: string;
  title: string;
  content: string;
  createdAt: string;
};

export type OrganizeStatus = "idle" | "running" | "done" | "error" | "cancelled";
