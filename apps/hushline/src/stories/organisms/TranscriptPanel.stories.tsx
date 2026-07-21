import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { TranscriptPanel } from "@/widgets/transcript-panel";

const meta = { title: "Atomic Design/Organisms/Transcript Panel", component: TranscriptPanel, args: { result: null, busy: false, copied: false, language: "ko", stage: "idle", streamLines: [], chunks: [], onCopy: fn() } } satisfies Meta<typeof TranscriptPanel>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Empty: Story = {};
export const Streaming: Story = { args: { busy: true, stage: "transcribe", streamLines: ["67% | processing audio"], chunks: [{ timestamp: "00:00.000 --> 00:04.200", text: "안녕하세요. 지금부터 영상의 음성을 텍스트로 변환합니다." }, { timestamp: "00:04.200 --> 00:08.900", text: "생성되는 문장은 이곳에 실시간으로 나타납니다." }] } };
export const Completed: Story = { args: { stage: "done", result: { url: "https://youtu.be/sample", title: "로컬 Whisper로 만드는 안전한 Transcript", transcript: "안녕하세요. 지금부터 영상의 음성을 텍스트로 변환합니다.\n\n모든 작업은 사용자의 컴퓨터에서 처리됩니다.", transcript_path: "/tmp/transcript.txt", audio_path: "/tmp/audio.wav", json_path: "/tmp/result.hushline.json", language: "ko", model: "base", cached: false, duration_seconds: 188 } } };
