export type Stage = "idle" | "metadata" | "download" | "extract" | "transcribe" | "done" | "error";
export type ProgressEvent = { stage: Stage; progress: number; message: string; detail?: string };
export type TranscriptChunk = { text: string; timestamp?: string };
export type ToolStatus = { yt_dlp: boolean; yt_dlp_version?: string; yt_dlp_error?: string; ffmpeg: boolean; whisper: boolean; whisper_command?: string };
export type TranscriptionResult = { url: string; title: string; transcript: string; transcript_path: string; audio_path: string; json_path: string; language: string; model: string; cached: boolean; duration_seconds?: number };
export type ModelStatus = { name: string; downloaded: boolean; path: string };
export type QueueStatus = "queued" | "processing" | "completed" | "cached" | "error";
export type QueueItem = { id: string; url: string; language: string; model: string; force: boolean; status: QueueStatus; progress: number; message: string };
