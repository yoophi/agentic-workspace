import type { Meta, StoryObj } from "@storybook/react-vite";

import {
  addExtraPanel,
  createInitialAgentRunAreaState,
} from "@/features/agent-run/model/agent-run-panel-slots";

import { WorkspacePromptComposer } from "./workspace-prompt-composer";

const workspace = addExtraPanel(createInitialAgentRunAreaState());
const meta = {
  title: "Organisms/Agent Run/Workspace Prompt Composer",
  component: WorkspacePromptComposer,
  args: {
    slots: workspace.slots,
    focusedPanelId: workspace.focusedPanelId,
    onSubmit: () => undefined,
  },
} satisfies Meta<typeof WorkspacePromptComposer>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
