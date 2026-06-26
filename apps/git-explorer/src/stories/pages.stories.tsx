import type { Meta, StoryObj } from "@storybook/react-vite";

import { RepositoryPage } from "@/pages/repository";

const meta = {
  title: "Atomic Design/Pages/Registered Components",
  parameters: {
    docs: {
      description: {
        component: "route 단위 화면 조립 결과입니다. Storybook Tauri mock 데이터로 실행됩니다.",
      },
    },
  },
} satisfies Meta;

export default meta;
type Story = StoryObj<typeof meta>;

export const RepositoryExplorer: Story = {
  render: () => (
    <div className="h-[780px] overflow-hidden rounded-md border">
      <RepositoryPage />
    </div>
  ),
};
