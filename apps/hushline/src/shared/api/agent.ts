// hushline이 소비하는 ACP agent run API. 실행 제어는 공유 패키지 `@yoophi/agent-client`를
// 재노출하고, 정리 문서 저장은 hushline 전용 command를 래핑한다.
import { invoke } from "@tauri-apps/api/core";
import {
  cancelAgentRun,
  listenRunEvents,
  sendPromptToRun,
  startAgentRun,
} from "@yoophi/agent-client";

// 정리·대화에 사용할 기본 agent. 실제 실행 명령은 백엔드 카탈로그(env)가 해석한다.
export const DEFAULT_AGENT_ID = "claude-code";

export type OrganizedDocumentInput = {
  sourceUrl?: string | null;
  sourceTranscriptPath?: string | null;
  style: string;
  title: string;
  content: string;
  createdAt: string;
};

export type ChatMessageInput = { role: string; text: string; createdAt: string };

export type ChatSessionInput = {
  documentPath?: string | null;
  sourceTranscriptPath?: string | null;
  title: string;
  messages: ChatMessageInput[];
  createdAt: string;
};

export const agent = {
  startAgentRun,
  sendPromptToRun,
  cancelAgentRun,
  listenRunEvents,
  saveOrganizedDocument: (dir: string, baseName: string, document: OrganizedDocumentInput) =>
    invoke<string>("save_organized_document", { dir, baseName, document }),
  saveChatSession: (dir: string, baseName: string, session: ChatSessionInput) =>
    invoke<string>("save_chat_session", { dir, baseName, session }),
};
