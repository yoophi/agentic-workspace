import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";
import { ModelStatusPanel } from "@/features/manage-whisper-model";

const models = ["tiny", "base", "small", "medium", "large"].map((name, index) => ({ name, downloaded: index < 3, path: `/Users/demo/.cache/whisper/${name}.pt` }));
const meta = { title: "Atomic Design/Molecules/Model Status", component: ModelStatusPanel, args: { models, selectedModel: "base", download: null, onSelect: fn() } } satisfies Meta<typeof ModelStatusPanel>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Installed: Story = {};
export const InstallationRequired: Story = { args: { selectedModel: "medium" } };
export const Downloading: Story = { args: { selectedModel: "large", download: { model: "large", progress: 46 } } };
export const Loading: Story = { args: { models: [] } };
