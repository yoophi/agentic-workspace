// hushline이 소비하는 ACP agent run API. 실행 제어는 공유 패키지 `@yoophi/agent-client`를
// 재노출하고, 정리 문서 저장은 hushline 전용 command를 래핑한다.
import { invoke } from "@tauri-apps/api/core";
import {
  cancelAgentRun,
  listenRunEvents,
  sendPromptToRun,
  startAgentRun,
} from "@yoophi/agent-client";

export type OrganizedDocumentInput = {
  sourceUrl?: string | null;
  sourceTranscriptPath?: string | null;
  style: string;
  title: string;
  content: string;
  createdAt: string;
};

export const agent = {
  startAgentRun,
  sendPromptToRun,
  cancelAgentRun,
  listenRunEvents,
  saveOrganizedDocument: (dir: string, baseName: string, document: OrganizedDocumentInput) =>
    invoke<string>("save_organized_document", { dir, baseName, document }),
};
