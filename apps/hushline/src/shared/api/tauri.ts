import { invoke } from "@tauri-apps/api/core";
import { listen, type EventCallback, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
type Stage = "idle" | "metadata" | "download" | "extract" | "transcribe" | "done" | "error";
type ProgressEvent = { stage: Stage; progress: number; message: string; detail?: string };
type TranscriptChunk = { text: string; timestamp?: string };
type ToolStatus = { yt_dlp: boolean; yt_dlp_version?: string; yt_dlp_error?: string; ffmpeg: boolean; whisper: boolean; whisper_command?: string };
type ModelStatus = { name: string; downloaded: boolean; path: string };
type TranscriptionResult = { url: string; title: string; transcript: string; transcript_path: string; audio_path: string; json_path: string; language: string; model: string; cached: boolean; duration_seconds?: number };

export const backend = {
  dependencies: () => invoke<ToolStatus>("check_dependencies"),
  models: () => invoke<ModelStatus[]>("get_model_status"),
  downloadModel: (model: string) => invoke<void>("download_model", { request: { model } }),
  deleteModel: (model: string) => invoke<void>("delete_model", { request: { model } }),
  transcribe: (request: {url: string; language: string; model: string; outputDir: string | null; force?: boolean}) => invoke<TranscriptionResult>("process_video", { request }),
  chooseOutputDirectory: () => open({ directory: true, multiple: false, title: "저장 폴더 선택" }),
  openArtifact: (path: string) => openPath(path),
  onProgress: (callback: EventCallback<ProgressEvent>): Promise<UnlistenFn> => listen("pipeline-progress", callback),
  onTranscript: (callback: EventCallback<TranscriptChunk>): Promise<UnlistenFn> => listen("transcript-chunk", callback),
  onModelProgress: (callback: EventCallback<ProgressEvent>): Promise<UnlistenFn> => listen("model-download-progress", callback),
};
