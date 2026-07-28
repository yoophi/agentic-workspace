import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { orchestrationSessionFixture } from "@/shared/storybook/agent-orchestration-sample-data";

import { TaskActivityRail } from "./task-activity-rail";

describe("TaskActivityRail", () => {
  it("shows role, state, elapsed time, attention, and promotion action without color alone", () => {
    const html = renderToStaticMarkup(
      <TaskActivityRail
        session={orchestrationSessionFixture}
        now={Date.parse("2026-07-27T00:02:00Z")}
      />,
    );
    expect(html).toContain('aria-label="하위 에이전트 작업"');
    expect(html).toContain("Researcher");
    expect(html).toMatch(/실행 중|입력 필요|완료/);
    expect(html).toMatch(/패널로 열기|패널 분리/);
  });

  it("keeps a failed input response visible and announces one atomic command status", () => {
    const session = {
      ...orchestrationSessionFixture,
      tasks: orchestrationSessionFixture.tasks.map((task) => ({
        ...task,
        status: "inputRequired" as const,
      })),
      commands: [
        {
          id: "failed-input",
          requestId: "failed-input",
          payloadFingerprint: "failed-input",
          taskId: "task-research",
          nodeId: "node-researcher",
          runId: "run-researcher",
          attempt: 1,
          kind: "inputResponse" as const,
          message: "읽기 전용으로 진행",
          inputReportId: "report-1",
          delivery: "queue" as const,
          source: "user" as const,
          status: "failed" as const,
          failure: {
            code: "workerUnavailable",
            message: "응답 전달 실패",
            retryable: true,
          },
          createdAt: "2026-07-27T00:01:40Z",
          updatedAt: "2026-07-27T00:01:41Z",
        },
      ],
    };
    const html = renderToStaticMarkup(<TaskActivityRail session={session} />);
    expect(html).toContain("응답 전달 실패");
    expect(html).toContain('aria-atomic="true"');
    expect(html).toContain("입력 응답");
  });
});
