import type { Meta, StoryObj } from "@storybook/react-vite";
import { QueuePanel } from "../../widgets/queue-panel";
import { ResultGrid } from "../../widgets/result-grid";

const result = {
  url: "https://www.youtube.com/watch?v=hushline-demo",
  title: "로컬 Whisper로 만드는 조용한 영상 기록",
  transcript: "한 번 입력한 영상은 로컬 결과 저장소에서 찾아 다시 처리하지 않습니다. 언어를 바꾸면 새 설정으로 다시 음성을 인식할 수 있습니다.",
  transcript_path: "/Downloads/hushline-demo.txt",
  audio_path: "/Downloads/hushline-demo.wav",
  json_path: "/Downloads/hushline-demo.hushline.json",
  language: "ko",
  model: "base",
  cached: false,
  duration_seconds: 284,
};

const meta = { title: "Organisms/Queue and results", component: QueuePanel, parameters: { layout: "fullscreen" } } satisfies Meta<typeof QueuePanel>;
export default meta;
type Story = StoryObj<typeof meta>;

export const ProcessingQueue: Story = {
  args: {
    items: [
      { id: "1", url: result.url, language: "ko", model: "base", force: false, status: "processing", progress: 64, message: "Whisper가 음성을 듣고 있습니다" },
      { id: "2", url: "https://youtu.be/next-video", language: "auto", model: "base", force: false, status: "queued", progress: 0, message: "대기 중" },
      { id: "3", url: "https://youtu.be/cached-video", language: "en", model: "base", force: false, status: "cached", progress: 100, message: "기존 결과를 불러왔습니다" },
    ],
    chunks: [{ timestamp: "00:12 → 00:16", text: "생성된 텍스트가 처리 중에도 이 영역에 표시됩니다." }],
    onRemove: () => undefined,
  },
  render: (args) => <div className="min-h-screen bg-[#0b0d0a] p-10 text-[#f2f4ed]"><div className="ml-auto max-w-xl"><QueuePanel {...args}/></div></div>,
};

export const SavedResults: Story = {
  args: { items: [], onRemove: () => undefined },
  render: () => <div className="min-h-screen bg-[#0b0d0a] py-10 text-[#f2f4ed]"><ResultGrid results={[result, {...result, url: "https://youtu.be/cached", title: "이미 변환된 영상", cached: true}]} highlightedUrl={result.url} onRerun={() => undefined}/></div>,
};
