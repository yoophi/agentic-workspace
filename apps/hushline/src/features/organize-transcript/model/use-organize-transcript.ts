import { useCallback, useEffect, useRef, useState } from "react";

import type { OrganizeStatus } from "../../../entities/organized-document";
import type { TranscriptionResult } from "../../../entities/transcription";
import { agent } from "../../../shared/api";

// 정리에 사용할 기본 agent. 실제 실행 명령은 백엔드 카탈로그(env)가 해석한다.
const DEFAULT_AGENT_ID = "claude-code";

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

function slug(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "") || "style";
}

/**
 * 하나의 자막(result)을 사용자가 지정한 방식(style)으로 agent run을 통해 정리한다.
 * agentMessage 스트림을 누적해 output으로 노출하고, 완료 시 OrganizedDocument로 저장한다.
 */
export function useOrganizeTranscript(result: TranscriptionResult) {
  const [status, setStatus] = useState<OrganizeStatus>("idle");
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const runIdRef = useRef<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const styleRef = useRef<string>("");

  const teardown = useCallback(() => {
    unlistenRef.current?.();
    unlistenRef.current = null;
  }, []);

  useEffect(() => teardown, [teardown]);

  const start = useCallback(
    async (style: string) => {
      const trimmed = style.trim();
      if (!trimmed || status === "running") return;
      styleRef.current = trimmed;
      setOutput("");
      setError(null);
      setSavedPath(null);
      setStatus("running");
      teardown();

      unlistenRef.current = agent.listenRunEvents((envelope) => {
        if (runIdRef.current && envelope.runId !== runIdRef.current) return;
        const event = envelope.event;
        switch (event.type) {
          case "agentMessage":
            setOutput((prev) => prev + event.text);
            break;
          case "error":
            setError(event.message);
            setStatus("error");
            break;
          case "lifecycle":
            if (event.status === "promptCompleted" || event.status === "completed") {
              setStatus((prev) => (prev === "running" ? "done" : prev));
            } else if (event.status === "cancelled") {
              setStatus("cancelled");
            }
            break;
          default:
            break;
        }
      });

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
        runIdRef.current = run.id;
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setStatus("error");
        teardown();
      }
    },
    [result, status, teardown],
  );

  const cancel = useCallback(async () => {
    if (runIdRef.current) {
      await agent.cancelAgentRun(runIdRef.current);
    }
  }, []);

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
