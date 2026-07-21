import { useState } from "react";
import { CircleAlert, LoaderCircle, MessageSquare, Play, Save, Send, StopCircle } from "lucide-react";

import type { TranscriptionResult } from "../../../entities/transcription";
import { Button, Input } from "../../../shared/ui";
import { useChatWithDocument } from "../model/use-chat-with-document";

export function ChatPanel({ result }: { result: TranscriptionResult }) {
  const { messages, status, error, savedPath, startSession, ask, end, save } =
    useChatWithDocument(result);
  const [draft, setDraft] = useState("");
  const [saveError, setSaveError] = useState<string | null>(null);

  const active = status !== "idle";
  const busy = status === "starting" || status === "thinking";

  function submit() {
    const q = draft.trim();
    if (!q) return;
    setDraft("");
    void ask(q);
  }

  async function onSave() {
    setSaveError(null);
    try {
      await save();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="border-t border-white/[.07] bg-white/[.02] p-3">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-1.5 text-[11px] font-medium text-[#b6bcaf]">
          <MessageSquare size={12} className="text-[#d8ff65]" />
          문서 기반 대화
        </div>
        {active && (
          <Button variant="ghost" size="sm" onClick={end} disabled={status === "ended"}>
            <StopCircle size={12} />
            종료
          </Button>
        )}
      </div>

      {!active ? (
        <Button size="sm" className="w-full" onClick={startSession}>
          <Play size={13} />
          대화 시작
        </Button>
      ) : (
        <>
          <div className="transcript-scroll max-h-56 space-y-2 overflow-y-auto rounded-md bg-black/20 p-2.5">
            {messages.map((message, index) => (
              <div
                key={index}
                className={`text-xs leading-5 whitespace-pre-wrap ${
                  message.role === "user" ? "text-[#d8ff65]" : "text-[#c7ccbf]"
                }`}
              >
                <span className="mr-1 font-mono text-[9px] uppercase tracking-wider text-[#697164]">
                  {message.role === "user" ? "나" : "agent"}
                </span>
                {message.text || (busy && index === messages.length - 1 ? "…" : "")}
              </div>
            ))}
          </div>

          <div className="mt-2 flex gap-2">
            <Input
              className="h-9 flex-1 text-xs"
              value={draft}
              disabled={status === "starting"}
              placeholder="문서에 대해 질문하세요"
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={(event) => event.key === "Enter" && submit()}
            />
            <Button size="sm" onClick={submit} disabled={busy || !draft.trim()}>
              {busy ? <LoaderCircle size={13} className="animate-spin" /> : <Send size={13} />}
            </Button>
          </div>

          <div className="mt-2 flex items-center gap-2">
            <Button variant="secondary" size="sm" onClick={onSave} disabled={!!savedPath}>
              <Save size={13} />
              {savedPath ? "저장됨" : "대화 저장"}
            </Button>
            {savedPath && <span className="truncate text-[10px] text-[#596056]">{savedPath}</span>}
          </div>

          {(error || saveError) && (
            <p className="mt-2 flex items-center gap-1 text-xs text-red-300">
              <CircleAlert size={12} />
              {error ?? saveError}
            </p>
          )}
        </>
      )}
    </div>
  );
}
