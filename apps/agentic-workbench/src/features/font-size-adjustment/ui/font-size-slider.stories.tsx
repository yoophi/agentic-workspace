import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import type { FontSizeStep } from "@/entities/appearance-preferences/model/types";
import { FontSizeSlider } from "./font-size-slider";

const meta = {
  title: "Atomic Design/Molecules/Font Size Slider",
  component: FontSizeSlider,
  decorators: [
    (Story) => (
      <div className="w-[420px] rounded-lg border bg-background p-6">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof FontSizeSlider>;

export default meta;
type Story = StoryObj<typeof meta>;

function Controlled({ initial = 0 }: { initial?: FontSizeStep }) {
  const [value, setValue] = useState<FontSizeStep>(initial);
  return <FontSizeSlider value={value} onValueChange={setValue} />;
}

const requiredArgs = {
  value: 0 as FontSizeStep,
  onValueChange: () => undefined,
};

export const Smallest: Story = {
  args: requiredArgs,
  render: () => <Controlled initial={-2} />,
};
export const Default: Story = {
  args: requiredArgs,
  render: () => <Controlled initial={0} />,
};
export const Largest: Story = {
  args: requiredArgs,
  render: () => <Controlled initial={2} />,
};
export const Pending: Story = {
  args: {
    value: 1,
    isLoading: true,
    onValueChange: () => undefined,
  },
};
export const Error: Story = {
  args: {
    value: 0,
    error: "설정을 저장하지 못했습니다. 다시 시도해 주세요.",
    onValueChange: () => undefined,
  },
};
