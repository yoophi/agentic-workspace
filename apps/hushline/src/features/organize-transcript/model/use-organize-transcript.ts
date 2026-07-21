import { useCallback, useRef, useState } from "react";

import type { OrganizeStatus } from "../../../entities/organized-document";
import type { TranscriptionResult } from "../../../entities/transcription";
import { agent, DEFAULT_AGENT_ID } from "../../../shared/api";
import { baseNoExt, dirOf, fileOf, slug } from "../../../shared/lib/path";
import { useRunEventStream } from "../../../shared/lib/use-run-event-stream";

/**
 * 하나의 자막(result)을 사용자가 지정한 방식(style)으로 agent run을 통해 정리한다.
 * agentMessage 스트림을 누적해 output으로 노출하고, 완료 시 OrganizedDocument로 저장한다.
 */
export function useOrganizeTranscript(result: TranscriptionResult) {
  const [status, setStatus] = useState<OrganizeStatus>("idle");
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const styleRef = useRef<string>("");

  const { runIdRef, begin, setRunId, teardown } = useRunEventStream({
    onMessage: (text) => setOutput((prev) => prev + text),
    onError: (message) => {
      setError(message);
      setStatus("error");
    },
    onLifecycle: (lifecycle) => {
      if (lifecycle === "promptCompleted" || lifecycle === "completed") {
        setStatus((prev) => (prev === "running" ? "done" : prev));
      } else if (lifecycle === "cancelled") {
        setStatus("cancelled");
      }
    },
  });

  const start = useCallback(
    async (style: string) => {
      const trimmed = style.trim();
      if (!trimmed || status === "running") return;
      styleRef.current = trimmed;
      setOutput("");
      setError(null);
      setSavedPath(null);
      setStatus("running");
      begin();

      const dir = dirOf(result.transcript_path);
      const file = fileOf(result.transcript_path);
      const goal = `작업 폴더의 자막 파일 "${file}"을(를) 읽고, "${trimmed}" 방식으로 정리해줘. 정리한 본문만 출력해줘.`;
      try {
        const run = await agent.startAgentRun({
          goal,
          agentId: DEFAULT_AGENT_ID,
          cwd: dir,
          permissionMode: "readOnly",
        });
        setRunId(run.id);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setStatus("error");
        teardown();
      }
    },
    [result, status, begin, setRunId, teardown],
  );

  const cancel = useCallback(async () => {
    if (runIdRef.current) await agent.cancelAgentRun(runIdRef.current);
  }, [runIdRef]);

  const save = useCallback(async () => {
    if (!output.trim()) throw new Error("정리 내용이 비어 있습니다");
    const dir = dirOf(result.transcript_path);
    const style = styleRef.current || "정리";
    const baseName = `${baseNoExt(result.transcript_path)}-${slug(style)}`;
    const path = await agent.saveOrganizedDocument(dir, baseName, {
      sourceUrl: result.url,
      sourceTranscriptPath: result.transcript_path,
      style,
      title: `${result.title} · ${style}`,
      content: output,
      createdAt: new Date().toISOString(),
    });
    setSavedPath(path);
    return path;
  }, [output, result]);

  return { status, output, error, savedPath, start, cancel, save };
}
