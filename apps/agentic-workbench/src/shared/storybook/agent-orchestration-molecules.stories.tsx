import type { Meta, StoryObj } from "@storybook/react-vite";

import { orchestrationSessionFixture } from "./agent-orchestration-sample-data";
import { TaskActivityItem } from "@/features/agent-run/ui/task-activity-item";

const meta = {
  title: "Molecules/Agent Orchestration/Task Activity Item",
  component: TaskActivityItem,
  args: {
    task: orchestrationSessionFixture.tasks[0],
    node: orchestrationSessionFixture.nodes[1],
    reports: orchestrationSessionFixture.reports,
    now: Date.parse("2026-07-27T00:02:00Z"),
  },
} satisfies Meta<typeof TaskActivityItem>;

export default meta;
type Story = StoryObj<typeof meta>;
export const Running: Story = {};

