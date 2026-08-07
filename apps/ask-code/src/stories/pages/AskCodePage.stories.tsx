import type { Meta, StoryObj } from "@storybook/react-vite";

import { AskCodePage } from "@/pages/ask-code";

const meta = {
  title: "pages/Ask Code",
  component: AskCodePage,
  parameters: {
    layout: "fullscreen",
  },
} satisfies Meta<typeof AskCodePage>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
