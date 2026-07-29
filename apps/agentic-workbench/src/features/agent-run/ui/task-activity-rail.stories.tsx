import type { Meta, StoryObj } from "@storybook/react-vite";

import { orchestrationSessionFixture } from "@/shared/storybook/agent-orchestration-sample-data";

import { TaskActivityRail } from "./task-activity-rail";

const meta = {
  title: "Organisms/Agent Run/Task Activity Rail",
  component: TaskActivityRail,
  args: {
    session: orchestrationSessionFixture,
    now: Date.parse("2026-07-27T00:02:00Z"),
  },
} satisfies Meta<typeof TaskActivityRail>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const PendingCommand: Story = {
  args: {
    session: {
      ...orchestrationSessionFixture,
      commands: [
        {
          id: "command-pending",
          requestId: "request-pending",
          payloadFingerprint: "pending",
          taskId: "task-research",
          nodeId: "node-researcher",
          runId: "run-researcher",
          attempt: 1,
          kind: "message",
          message: "추가 근거를 조사해 주세요.",
          inputReportId: null,
          delivery: "queue",
          source: "coordinator",
          status: "dispatching",
          failure: null,
          createdAt: "2026-07-27T00:01:40Z",
          updatedAt: "2026-07-27T00:01:41Z",
        },
      ],
    },
  },
};

export const FailedInputResponse: Story = {
  args: {
    session: {
      ...orchestrationSessionFixture,
      tasks: orchestrationSessionFixture.tasks.map((task) => ({
        ...task,
        status: "inputRequired" as const,
      })),
      commands: [
        {
          id: "command-failed",
          requestId: "request-failed",
          payloadFingerprint: "failed",
          taskId: "task-research",
          nodeId: "node-researcher",
          runId: "run-researcher",
          attempt: 1,
          kind: "inputResponse",
          message: "읽기 전용으로 진행하세요.",
          inputReportId: "report-1",
          delivery: "queue",
          source: "user",
          status: "failed",
          failure: {
            code: "workerUnavailable",
            message: "Child runtime이 응답하지 않습니다.",
            retryable: true,
          },
          createdAt: "2026-07-27T00:01:40Z",
          updatedAt: "2026-07-27T00:01:41Z",
        },
      ],
    },
  },
};

export const ResultWithRejectedArtifact: Story = {
  args: {
    session: {
      ...orchestrationSessionFixture,
      nodes: orchestrationSessionFixture.nodes.map((node) =>
        node.kind === "child"
          ? {
              ...node,
              runtimeProfile: {
                agentProfileId: "orchestration-smoke",
                providerId: "acp",
                modelId: "claude-opus-5",
                accessPolicy: "readOnly" as const,
                supportsReadOnly: true,
              },
            }
          : node,
      ),
      reports: orchestrationSessionFixture.reports.map((report) => ({
        ...report,
        type: "result" as const,
        progressPercent: 100,
        summary: "조사 결과와 근거를 정리했습니다.",
        artifactRefs: [
          { kind: "file" as const, uri: "docs/research-notes.md", label: "조사 메모" },
        ],
        unresolved: [
          "Rejected artifact reference ../outside.txt: The artifact path escapes the workspace.",
        ],
      })),
    },
  },
};

export const EventGap: Story = {
  args: {
    runtimeStates: { "run-researcher": "gap" },
  },
};

export const RuntimeLost: Story = {
  args: {
    runtimeStates: { "run-researcher": "runtimeLost" },
  },
};
