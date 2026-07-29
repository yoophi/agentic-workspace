import type {
  OrchestrationSession,
  WorkerRuntimeProfile,
} from "@/entities/agent-orchestration";

/** Runtime profile of the deterministic smoke Worker used by the quickstart scenarios. */
export const smokeRuntimeProfileFixture: WorkerRuntimeProfile = {
  agentProfileId: "orchestration-smoke",
  providerId: "acp",
  modelId: "claude-opus-5",
  accessPolicy: "readOnly",
  supportsReadOnly: true,
};

/** `unresolved` entry the backend records when it rejects an out-of-workspace artifact (FR-047). */
export const rejectedArtifactUnresolvedFixture =
  "Rejected artifact reference ../outside.txt: The artifact path escapes the workspace.";

export const sampleOrchestrationWorkspace = {
  workspaceId: "workspace-storybook",
  windowLabel: "worktree-storybook",
  worktreePath: "/workspace/agentic-workbench",
  mainNodeId: "node-main",
  revision: 7,
  children: [
    { nodeId: "node-researcher", role: "Researcher", status: "running" },
    { nodeId: "node-reviewer", role: "Reviewer", status: "waitingInput" },
    { nodeId: "node-tester", role: "Tester", status: "completed" },
  ],
} as const;

export const orchestrationSessionFixture: OrchestrationSession = {
  schemaVersion: 2,
  id: "workspace-storybook",
  worktreePath: "/workspace/agentic-workbench",
  boundWindowLabel: "worktree-storybook",
  mainNodeId: "main-agent-run",
  activeCoordinatorGenerationId: "generation-1",
  nodes: [
    {
      id: "main-agent-run",
      kind: "main",
      parentNodeId: null,
      role: {
        id: "main-coordinator",
        name: "Main",
        responsibility: "하위 에이전트 조율",
        expectedOutput: "종합 결과",
      },
      currentRunId: "run-main",
      assignedTaskId: null,
      executionStatus: "active",
      presentationStatus: "panel",
      promotionPolicy: "always",
      runtimeProfile: null,
      lastActivityAt: "2026-07-27T00:01:50Z",
      createdBy: "user",
      createdAt: "2026-07-27T00:00:00Z",
    },
    {
      id: "node-researcher",
      kind: "child",
      parentNodeId: "main-agent-run",
      role: {
        id: "researcher",
        name: "Researcher",
        responsibility: "근거 조사",
        expectedOutput: "출처가 있는 요약",
      },
      currentRunId: "run-researcher",
      assignedTaskId: "task-research",
      executionStatus: "active",
      presentationStatus: "background",
      promotionPolicy: "onAttention",
      runtimeProfile: null,
      lastActivityAt: "2026-07-27T00:01:30Z",
      createdBy: "coordinator",
      createdAt: "2026-07-27T00:00:10Z",
    },
  ],
  generations: [
    {
      id: "generation-1",
      ordinal: 1,
      mainNodeId: "main-agent-run",
      runId: "run-main",
      previousGenerationId: null,
      status: "active",
      startedAt: "2026-07-27T00:00:00Z",
      endedAt: null,
      handoffSummary: null,
      successorGenerationId: null,
    },
  ],
  tasks: [
    {
      id: "task-research",
      parentTaskId: null,
      coordinatorGenerationId: "generation-1",
      assignedNodeId: "node-researcher",
      title: "구현 전략 조사",
      objective: "현재 구조와 통합 지점을 조사한다.",
      constraints: ["read-only"],
      expectedResult: "근거가 있는 요약",
      dependencyTaskIds: [],
      status: "running",
      awaitingHandoff: false,
      accessPolicy: "readOnly",
      attempt: 1,
      latestResultReportId: null,
      failure: null,
      revision: 1,
      createdAt: "2026-07-27T00:00:10Z",
      startedAt: "2026-07-27T00:00:20Z",
      completedAt: null,
      updatedAt: "2026-07-27T00:01:30Z",
    },
  ],
  reports: [
    {
      id: "report-1",
      requestId: "request-report-1",
      taskId: "task-research",
      reporterNodeId: "node-researcher",
      reporterRunId: "run-researcher",
      type: "progress",
      progressPercent: 55,
      summary: "핵심 통합 지점을 확인하고 있습니다.",
      findings: [],
      artifactRefs: [],
      unresolved: [],
      confidence: null,
      createdAt: "2026-07-27T00:01:30Z",
    },
  ],
  commands: [],
  coordinatorNotifications: [],
  dispatches: [],
  revision: 7,
  createdAt: "2026-07-27T00:00:00Z",
  updatedAt: "2026-07-27T00:01:50Z",
};
