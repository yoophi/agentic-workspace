import { useState } from "react";
import { Check, CircleAlert, LoaderCircle, Save, Sparkles, StopCircle } from "lucide-react";

import type { TranscriptionResult } from "../../../entities/transcription";
import { Button, Input } from "../../../shared/ui";
import { useOrganizeTranscript } from "../model/use-organize-transcript";

const PRESETS = ["세 문단 요약", "회의록 형식", "핵심 불릿 정리", "블로그 글"];

export function OrganizePanel({ result }: { result: TranscriptionResult }) {
  const [style, setStyle] = useState("세 문단 요약");
  const { status, output, error, savedPath, start, cancel, save } = useOrganizeTranscript(result);
  const [saveError, setSaveError] = useState<string | null>(null);

  const running = status === "running";

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
      <div className="mb-2 flex items-center gap-1.5 text-[11px] font-medium text-[#b6bcaf]">
        <Sparkles size={12} className="text-[#d8ff65]" />
        Agent로 정리
      </div>

      <div className="mb-2 flex flex-wrap gap-1.5">
        {PRESETS.map((preset) => (
          <button
            key={preset}
            type="button"
            disabled={running}
            onClick={() => setStyle(preset)}
            className={`rounded-full px-2 py-1 font-mono text-[9px] uppercase tracking-wider transition-colors ${
              style === preset ? "bg-[#d8ff65]/15 text-[#d8ff65]" : "bg-white/[.05] text-[#858d80] hover:text-[#d9ddd3]"
            }`}
          >
            {preset}
          </button>
        ))}
      </div>

      <div className="flex gap-2">
        <Input
          className="h-9 flex-1 text-xs"
          value={style}
          disabled={running}
          placeholder="정리 방식을 입력하세요"
          onChange={(event) => setStyle(event.target.value)}
        />
        {running ? (
          <Button variant="secondary" size="sm" onClick={cancel}>
            <StopCircle size={13} />
            취소
          </Button>
        ) : (
          <Button size="sm" onClick={() => start(style)} disabled={!style.trim()}>
            <Sparkles size={13} />
            정리하기
          </Button>
        )}
      </div>

      {status !== "idle" && (
        <div className="mt-3">
          <div className="mb-1.5 flex items-center gap-1.5 text-[10px] text-[#8f9688]">
            {running && <LoaderCircle size={12} className="animate-spin text-[#d8ff65]" />}
            {status === "done" && <Check size={12} className="text-[#d8ff65]" />}
            {(status === "error" || status === "cancelled") && <CircleAlert size={12} className="text-red-400" />}
            <span>
              {running
                ? "정리 중…"
                : status === "done"
                  ? "정리 완료"
                  : status === "cancelled"
                    ? "취소됨"
                    : "오류"}
            </span>
          </div>
          {output && (
            <div className="transcript-scroll max-h-48 overflow-y-auto rounded-md bg-black/20 p-2.5 text-xs leading-5 whitespace-pre-wrap text-[#c7ccbf]">
              {output}
            </div>
          )}
          {error && <p className="mt-2 text-xs text-red-300">{error}</p>}
          {status === "done" && output.trim() && (
            <div className="mt-2 flex items-center gap-2">
              <Button variant="secondary" size="sm" onClick={onSave} disabled={!!savedPath}>
                <Save size={13} />
                {savedPath ? "저장됨" : "문서로 저장"}
              </Button>
              {savedPath && <span className="truncate text-[10px] text-[#596056]">{savedPath}</span>}
            </div>
          )}
          {saveError && <p className="mt-2 text-xs text-red-300">{saveError}</p>}
        </div>
      )}
    </div>
  );
}
