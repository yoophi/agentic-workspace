import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import {
  orchestrationSessionFixture,
  rejectedArtifactUnresolvedFixture,
  smokeRuntimeProfileFixture,
} from "@/shared/storybook/agent-orchestration-sample-data";

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
    // Optional rows stay absent when the report carries no artifacts or unresolved items.
    expect(html).not.toContain("산출물");
    expect(html).not.toContain("미해결");
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
    // FR-048: retryability is stated in words next to the preserved backend message.
    expect(html).toContain("재시도 가능");
    expect(html).toContain('data-command-retryable="true"');
  });

  // FR-048: a failed task must show its reason and whether retrying is possible, without
  // relying on color, and a non-retryable failure must not be labelled retryable.
  it("shows a task failure reason with its retryability", () => {
    const [first, ...rest] = orchestrationSessionFixture.tasks;
    const session = {
      ...orchestrationSessionFixture,
      tasks: [
        {
          ...first,
          status: "failed" as const,
          failure: {
            code: "readOnlyViolation",
            message: "쓰기 도구 호출이 거부되었습니다.",
            retryable: false,
            partialResultReportIds: [],
          },
        },
        ...rest,
      ],
    };

    const html = renderToStaticMarkup(<TaskActivityRail session={session} />);

    expect(html).toContain("쓰기 도구 호출이 거부되었습니다.");
    expect(html).toContain("재시도 불가");
    expect(html).toContain('data-task-failure-code="readOnlyViolation"');
    expect(html).toContain('data-task-failure-retryable="false"');
    expect(html).toContain("쓰기가 필요한 작업은 사용자가 직접 수행하세요.");
  });

  // FR-046: v1 assigns the promotion policy by role, so the Rail must not offer a control
  // that edits it, and attention must never move focus on its own (FR-014).
  it("offers no promotion-policy control and never steals focus", () => {
    const session = {
      ...orchestrationSessionFixture,
      nodes: orchestrationSessionFixture.nodes.map((node) => ({
        ...node,
        presentationStatus:
          node.kind === "child" ? ("attentionRequired" as const) : node.presentationStatus,
      })),
    };

    const html = renderToStaticMarkup(<TaskActivityRail session={session} />);

    expect(html).not.toContain("승격 정책");
    expect(html).not.toContain("promotionPolicy");
    expect(html).not.toContain("autofocus");
    expect(html).not.toContain("autoFocus");
  });

  // UI contract: every row carries progress, provider/profile/model and artifact count, so a
  // background task can be judged from the Rail alone without promoting it to a panel.
  it("shows progress, runtime profile and artifact count for a background task", () => {
    const [report] = orchestrationSessionFixture.reports;
    const session = {
      ...orchestrationSessionFixture,
      nodes: orchestrationSessionFixture.nodes.map((node) =>
        node.kind === "child"
          ? { ...node, runtimeProfile: smokeRuntimeProfileFixture }
          : node,
      ),
      reports: [
        {
          ...report,
          artifactRefs: [
            { kind: "file" as const, uri: "docs/notes.md", label: "notes" },
            { kind: "url" as const, uri: "https://example.com", label: "ref" },
          ],
        },
      ],
    };

    const html = renderToStaticMarkup(<TaskActivityRail session={session} />);

    expect(html).toContain("55%");
    expect(html).toContain("acp · claude-opus-5 · orchestration-smoke");
    expect(html).toContain("산출물 2개");
  });

  // FR-047: a rejected artifact reference is recorded in `unresolved`, and the Rail is where
  // the user learns the reference was dropped while the report body was kept. The rejection can
  // land on any report, so a later report without unresolved items must not hide it.
  it("shows unresolved items from earlier reports so a rejected artifact stays visible", () => {
    const [report] = orchestrationSessionFixture.reports;
    const session = {
      ...orchestrationSessionFixture,
      reports: [
        { ...report, unresolved: [rejectedArtifactUnresolvedFixture] },
        {
          ...report,
          id: "report-2",
          requestId: "request-report-2",
          type: "result" as const,
          summary: "조사 결과를 정리했습니다.",
        },
      ],
    };

    const html = renderToStaticMarkup(<TaskActivityRail session={session} />);

    expect(html).toContain("조사 결과를 정리했습니다.");
    expect(html).toContain("미해결 1건");
    expect(html).toContain("../outside.txt");
  });
});
