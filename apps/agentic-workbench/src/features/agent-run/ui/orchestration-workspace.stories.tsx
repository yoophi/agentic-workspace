import type { Meta, StoryObj } from "@storybook/react-vite";

import { orchestrationSessionFixture } from "@/shared/storybook/agent-orchestration-sample-data";
import {
  addExtraPanel,
  createInitialAgentRunAreaState,
} from "@/features/agent-run/model/agent-run-panel-slots";

import { TaskActivityRail } from "./task-activity-rail";
import { WorkspacePromptComposer } from "./workspace-prompt-composer";

function OrchestrationWorkspacePreview() {
  const workspace = addExtraPanel(createInitialAgentRunAreaState());
  return (
    <div className="flex h-[640px] flex-col rounded-lg border">
      <div className="flex min-h-0 flex-1">
        <div className="grid flex-1 place-items-center bg-muted/20 text-muted-foreground">
          탭/타일 Agent Run 영역
        </div>
        <TaskActivityRail session={orchestrationSessionFixture} />
      </div>
      <WorkspacePromptComposer
        slots={workspace.slots}
        focusedPanelId={workspace.focusedPanelId}
        onSubmit={() => undefined}
      />
    </div>
  );
}

function MainUnavailablePreview() {
  const session = {
    ...orchestrationSessionFixture,
    coordinatorNotifications: [
      {
        id: "notification-pending",
        reportId: "report-1",
        taskId: "task-research",
        reportType: "progress" as const,
        generationId: "generation-1",
        mainRunId: null,
        status: "pending" as const,
        attemptCount: 0,
        failure: {
          code: "workerUnavailable",
          message: "Main runtime 연결을 기다리고 있습니다.",
          retryable: true,
        },
        collectedAt: null,
        createdAt: "2026-07-27T00:01:30Z",
        updatedAt: "2026-07-27T00:01:50Z",
      },
    ],
  };
  return (
    <div className="flex h-[640px] flex-col rounded-lg border">
      <div className="border-b bg-amber-50 px-3 py-2 text-sm text-amber-900">
        Main unavailable · report notification 재전송 대기
      </div>
      <div className="flex min-h-0 flex-1">
        <div className="grid flex-1 place-items-center bg-muted/20 text-muted-foreground">
          Background Child runtime은 계속 실행 중
        </div>
        <TaskActivityRail
          session={session}
          runtimeStates={{ "run-researcher": "ready" }}
        />
      </div>
    </div>
  );
}

const meta = {
  title: "Pages/Agent Run/Orchestration Workspace",
  component: OrchestrationWorkspacePreview,
} satisfies Meta<typeof OrchestrationWorkspacePreview>;

export default meta;
type Story = StoryObj<typeof meta>;
export const Default: Story = {};
export const MainUnavailable: StoryObj<typeof MainUnavailablePreview> = {
  render: () => <MainUnavailablePreview />,
};
