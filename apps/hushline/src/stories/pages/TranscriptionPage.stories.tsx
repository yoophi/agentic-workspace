import type { Meta, StoryObj } from "@storybook/react-vite";
import { TranscriptionPage } from "@/pages/transcription";

const meta = { title: "Atomic Design/Pages/Transcription", component: TranscriptionPage, parameters: { layout: "fullscreen", docs: { description: { component: "Atoms, molecules, organisms를 조합한 Hushline 전체 변환 화면입니다. Tauri API는 Storybook fixture로 대체됩니다." } } } } satisfies Meta<typeof TranscriptionPage>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Default: Story = {};
