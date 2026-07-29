import type { Meta, StoryObj } from "@storybook/react-vite";

import { Badge } from "@/components/ui/badge";

function OrchestrationStatusBadges() {
  return (
    <div className="flex flex-wrap gap-2">
      <Badge>실행 중</Badge>
      <Badge variant="destructive">입력 필요</Badge>
      <Badge variant="secondary">완료</Badge>
      <span aria-label="확인 필요">⚠ 확인 필요</span>
    </div>
  );
}

const meta = {
  title: "Atoms/Agent Orchestration/Status",
  component: OrchestrationStatusBadges,
} satisfies Meta<typeof OrchestrationStatusBadges>;

export default meta;
type Story = StoryObj<typeof meta>;
export const States: Story = {};

