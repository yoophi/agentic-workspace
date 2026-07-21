import { useCallback, useEffect, useRef, useState } from "react";

import type { TranscriptionResult } from "../../../entities/transcription";
import { agent } from "../../../shared/api";

const DEFAULT_AGENT_ID = "claude-code";

export type ChatRole = "user" | "assistant";
export type ChatMessage = { role: ChatRole; text: string };
export type ChatStatus = "idle" | "starting" | "ready" | "thinking" | "error" | "ended";

function dirOf(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx > 0 ? path.slice(0, idx) : path;
}

function fileOf(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx >= 0 ? path.slice(idx + 1) : path;
}

function baseNoExt(path: string): string {
  const file = fileOf(path);
  const dot = file.lastIndexOf(".");
  return dot > 0 ? file.slice(0, dot) : file;
}

/**
 * 저장된 자막/문서를 대상으로 한 세션 내 다회 대화. 하나의 장수(long-lived) run을 유지하고
 * `sendPromptToRun`으로 후속 질문을 이어 보낸다. agentMessage 스트림은 현재 assistant 턴에 누적된다.
 */
export function useChatWithDocument(result: TranscriptionResult) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [status, setStatus] = useState<ChatStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const runIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);

  const teardown = useCallback(() => {
    unlistenRef.current?.();
    unlistenRef.current = null;
  }, []);

  useEffect(() => teardown, [teardown]);

  const appendToAssistant = useCallback((text: string) => {
    setMessages((prev) => {
      const next = [...prev];
      const last = next[next.length - 1];
      if (last && last.role === "assistant") {
        next[next.length - 1] = { ...last, text: last.text + text };
      } else {
        next.push({ role: "assistant", text });
      }
      return next;
    });
  }, []);

  const subscribe = useCallback(() => {
    teardown();
    unlistenRef.current = agent.listenRunEvents((envelope) => {
      if (runIdRef.current && envelope.runId !== runIdRef.current) return;
      const event = envelope.event;
      switch (event.type) {
        case "agentMessage":
          appendToAssistant(event.text);
          break;
        case "error":
          setError(event.message);
          setStatus("error");
          break;
        case "lifecycle":
          if (event.status === "promptCompleted" || event.status === "completed") {
            setStatus((prev) => (prev === "ended" || prev === "error" ? prev : "ready"));
          } else if (event.status === "cancelled") {
            setStatus("ended");
          }
          break;
        default:
          break;
      }
    });
  }, [appendToAssistant, teardown]);

  const startSession = useCallback(async () => {
    if (status !== "idle" && status !== "error" && status !== "ended") return;
    setMessages([{ role: "assistant", text: "" }]);
    setError(null);
    setSavedPath(null);
    setStatus("starting");
    subscribe();

    const dir = dirOf(result.transcript_path);
    const file = fileOf(result.transcript_path);
    const goal = `작업 폴더의 자막 파일 "${file}"을(를) 읽고 내용을 파악해줘. 이후 이 내용에 대한 질문에 답할 것이다. 준비되면 한 문장으로만 확인해줘.`;
    try {
      const run = await agent.startAgentRun({
        goal,
        agentId: DEFAULT_AGENT_ID,
        cwd: dir,
        permissionMode: "readOnly",
      });
      runIdRef.current = run.id;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus("error");
      teardown();
    }
  }, [result, status, subscribe, teardown]);

  const ask = useCallback(
    async (question: string) => {
      const trimmed = question.trim();
      if (!trimmed || !runIdRef.current || status === "thinking" || status === "starting") return;
      setMessages((prev) => [...prev, { role: "user", text: trimmed }, { role: "assistant", text: "" }]);
      setStatus("thinking");
      try {
        await agent.sendPromptToRun(runIdRef.current, trimmed);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setStatus("error");
      }
    },
    [status],
  );

  const end = useCallback(async () => {
    if (runIdRef.current) await agent.cancelAgentRun(runIdRef.current);
    setStatus("ended");
  }, []);

  const save = useCallback(async () => {
    const meaningful = messages.filter((m) => m.text.trim());
    if (!meaningful.length) throw new Error("저장할 대화가 없습니다");
    const dir = dirOf(result.transcript_path);
    const now = new Date().toISOString();
    const path = await agent.saveChatSession(dir, `${baseNoExt(result.transcript_path)}-chat`, {
      sourceTranscriptPath: result.transcript_path,
      title: `${result.title} · 대화`,
      messages: meaningful.map((m) => ({ role: m.role, text: m.text, createdAt: now })),
      createdAt: now,
    });
    setSavedPath(path);
    return path;
  }, [messages, result]);

  return { messages, status, error, savedPath, startSession, ask, end, save };
}
