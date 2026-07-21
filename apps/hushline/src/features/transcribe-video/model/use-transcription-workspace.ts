import { useEffect, useMemo, useRef, useState } from "react";
import type { ModelStatus, QueueItem, Stage, ToolStatus, TranscriptChunk, TranscriptionResult } from "../../../entities/transcription";
import { backend } from "../../../shared/api";

export const isYouTubeUrl = (value: string) => /^https?:\/\/(www\.)?(youtube\.com|youtu\.be)\//i.test(value.trim());
const newId = () => globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;

export function useTranscriptionWorkspace() {
  const [url, setUrl] = useState("");
  const [language, setLanguage] = useState("auto");
  const [model, setModel] = useState("base");
  const [outputDir, setOutputDir] = useState("");
  const [stage, setStage] = useState<Stage>("idle");
  const [progress, setProgress] = useState(0);
  const [message, setMessage] = useState("URL을 queue에 추가하세요");
  const [tools, setTools] = useState<ToolStatus | null>(null);
  const [streamLines, setStreamLines] = useState<string[]>([]);
  const [liveTranscript, setLiveTranscript] = useState<TranscriptChunk[]>([]);
  const [models, setModels] = useState<ModelStatus[]>([]);
  const [modelDownload, setModelDownload] = useState<{model: string; progress: number} | null>(null);
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [results, setResults] = useState<TranscriptionResult[]>([]);
  const [highlightedUrl, setHighlightedUrl] = useState<string | null>(null);
  const processingRef = useRef(false);
  const currentIdRef = useRef<string | null>(null);

  useEffect(() => {
    backend.dependencies().then(setTools).catch(() => setTools({ yt_dlp: false, ffmpeg: false, whisper: false }));
    backend.models().then(setModels).catch(() => setModels([]));
    const subscriptions = [
      backend.onProgress(({ payload }) => {
        setStage(payload.stage); setProgress(payload.progress); setMessage(payload.message);
        if (payload.detail) setStreamLines((lines) => [...lines, payload.detail!].slice(-80));
        if (currentIdRef.current) setQueue((items) => items.map((item) => item.id === currentIdRef.current ? {...item, progress: payload.progress, message: payload.message} : item));
      }),
      backend.onTranscript(({ payload }) => setLiveTranscript((chunks) => chunks[chunks.length - 1]?.text === payload.text ? chunks : [...chunks, payload])),
      backend.onModelProgress(({ payload }) => setModelDownload((current) => ({ model: current?.model || "model", progress: payload.progress }))),
    ];
    return () => subscriptions.forEach((subscription) => { subscription.then((unlisten) => unlisten()); });
  }, []);

  useEffect(() => {
    if (processingRef.current || !(tools?.yt_dlp && tools.ffmpeg && tools.whisper)) return;
    const next = queue.find((item) => item.status === "queued");
    if (!next) return;
    processingRef.current = true; currentIdRef.current = next.id;
    setQueue((items) => items.map((item) => item.id === next.id ? {...item, status: "processing", progress: 2, message: "작업을 시작합니다"} : item));
    setStreamLines([]); setLiveTranscript([]); setStage("metadata"); setProgress(2);
    backend.transcribe({ url: next.url, language: next.language, model: next.model, outputDir: outputDir || null, force: next.force })
      .then((result) => {
        setResults((items) => [result, ...items.filter((item) => item.url !== result.url)]);
        setQueue((items) => items.map((item) => item.id === next.id ? {...item, status: result.cached ? "cached" : "completed", progress: 100, message: result.cached ? "기존 결과를 불러왔습니다" : "변환 완료"} : item));
        setHighlightedUrl(result.url); setTimeout(() => setHighlightedUrl(null), 3000);
      })
      .catch((error) => {
        setStage("error"); setMessage(String(error));
        setQueue((items) => items.map((item) => item.id === next.id ? {...item, status: "error", message: String(error)} : item));
      })
      .finally(() => {
        processingRef.current = false; currentIdRef.current = null;
        setQueue((items) => [...items]);
      });
  }, [queue, tools, outputDir]);

  const busy = queue.some((item) => item.status === "processing");
  const urlValid = isYouTubeUrl(url);
  const canStart = Boolean(urlValid && tools?.yt_dlp && tools.ffmpeg && tools.whisper);
  const selectedModelDownloaded = models.find((item) => item.name === model)?.downloaded ?? false;
  const stageLabel = useMemo(() => ({ idle: "준비", metadata: "영상 확인", download: "다운로드", extract: "오디오 정리", transcribe: "음성 인식", done: "완료", error: "오류" }[stage]), [stage]);

  function addToQueue() {
    if (!canStart) return;
    const normalized = url.trim();
    const existing = results.find((item) => item.url === normalized);
    if (existing) { setHighlightedUrl(normalized); setTimeout(() => setHighlightedUrl(null), 3000); setMessage("이미 변환된 결과를 표시했습니다"); return; }
    if (queue.some((item) => item.url === normalized && ["queued", "processing"].includes(item.status))) { setMessage("이미 queue에 있는 URL입니다"); return; }
    setQueue((items) => [...items, { id: newId(), url: normalized, language, model, force: false, status: "queued", progress: 0, message: "대기 중" }]);
    setUrl(""); setMessage("Queue에 추가했습니다");
  }

  function removeQueueItem(id: string) { if (id !== currentIdRef.current) setQueue((items) => items.filter((item) => item.id !== id)); }
  function rerun(result: TranscriptionResult, nextLanguage: string) { setQueue((items) => [...items, { id: newId(), url: result.url, language: nextLanguage, model, force: true, status: "queued", progress: 0, message: "언어 변경 재실행 대기 중" }]); }

  async function downloadSelectedModel() { setModelDownload({ model, progress: 1 }); try { await backend.downloadModel(model); setModels(await backend.models()); } catch (error) { setMessage(String(error)); setStage("error"); } finally { setModelDownload(null); } }
  async function deleteSelectedModel() { if (!window.confirm(`${model.toUpperCase()} 모델을 이 컴퓨터에서 삭제할까요? 필요하면 다시 다운로드할 수 있습니다.`)) return; try { await backend.deleteModel(model); setModels(await backend.models()); } catch (error) { setMessage(String(error)); setStage("error"); } }
  async function chooseFolder() { const selected = await backend.chooseOutputDirectory(); if (typeof selected === "string") setOutputDir(selected); }
  function resetError() { setStage("idle"); setProgress(0); setMessage("URL을 queue에 추가하세요"); }

  return { url, setUrl, language, setLanguage, model, setModel, outputDir, stage, progress, message, tools, streamLines, liveTranscript, models, modelDownload, queue, results, highlightedUrl, busy, urlValid, canStart, selectedModelDownloaded, stageLabel, addToQueue, removeQueueItem, rerun, downloadSelectedModel, deleteSelectedModel, chooseFolder, resetError };
}
