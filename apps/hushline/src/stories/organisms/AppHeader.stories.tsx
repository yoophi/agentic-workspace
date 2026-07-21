import type { Meta, StoryObj } from "@storybook/react-vite";
import { AppHeader } from "@/widgets/app-header";

const meta = { title: "Atomic Design/Organisms/App Header", component: AppHeader, parameters: { layout: "fullscreen" } } satisfies Meta<typeof AppHeader>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Ready: Story = { args: { tools: { yt_dlp: true, yt_dlp_version: "2026.07.18", ffmpeg: true, whisper: true } } };
export const MissingWhisper: Story = { args: { tools: { yt_dlp: true, yt_dlp_version: "2026.07.18", ffmpeg: true, whisper: false } } };
export const Checking: Story = { args: { tools: null } };
