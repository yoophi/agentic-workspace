const modelStatuses = ["tiny", "base", "small", "medium", "large"].map((name) => ({
  name, downloaded: ["tiny", "base", "small"].includes(name), path: `/Users/demo/.cache/whisper/${name}.pt`,
}));

export async function invoke<T>(command: string): Promise<T> {
  if (command === "check_dependencies") return { yt_dlp: true, yt_dlp_version: "2026.07.18", ffmpeg: true, whisper: true, whisper_command: "/usr/local/bin/whisper" } as T;
  if (command === "get_model_status") return modelStatuses as T;
  if (command === "process_video") return { url: "https://youtu.be/storybook", title: "Storybook에서 살펴보는 Hushline", transcript: "샘플 Transcript입니다.", transcript_path: "/tmp/transcript.txt", audio_path: "/tmp/audio.wav", json_path: "/tmp/result.hushline.json", language: "ko", model: "base", cached: false, duration_seconds: 185 } as T;
  return undefined as T;
}
