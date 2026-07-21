import type { Meta, StoryObj } from "@storybook/react-vite";

import type { TranscriptionResult } from "../../entities/transcription";
import { OrganizePanel } from "../../features/organize-transcript";
import { ChatPanel } from "../../features/chat-with-document";

const sampleResult: TranscriptionResult = {
  url: "https://youtube.com/watch?v=demo",
  title: "데모 영상 자막",
  transcript: "안녕하세요. 이것은 데모 자막입니다...",
  transcript_path: "/Users/demo/Downloads/Hushline/demo.txt",
  audio_path: "/Users/demo/Downloads/Hushline/demo.wav",
  json_path: "/Users/demo/Downloads/Hushline/demo.hushline.json",
  language: "ko",
  model: "base",
  cached: false,
};

const meta = {
  title: "organisms/AgentPanels",
  parameters: { layout: "centered" },
  decorators: [
    (Story) => (
      <div className="panel w-[380px] overflow-hidden bg-[#0b0d0a] text-[#f2f4ed]">
        <Story />
      </div>
    ),
  ],
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

// 자막을 사용자가 지정한 방식으로 정리하는 패널(초기 상태).
export const Organize: Story = {
  render: () => <OrganizePanel result={sampleResult} />,
};

// 문서 기반 세션 내 다회 대화 패널(초기 상태).
export const Chat: Story = {
  render: () => <ChatPanel result={sampleResult} />,
};
