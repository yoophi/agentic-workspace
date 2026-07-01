import { describe, expect, it } from "vitest";

import {
  buildDashboardQuickActions,
  buildProjectDashboard,
  buildProjectDashboardItem,
  getDashboardStatus,
} from "./dashboard-summary";
import type { ChangeSummary, SessionSummary, WorktreeSummary } from "../model";
import type { Project } from "../model/types";

const projects: Project[] = [
  {
    id: "alpha",
    name: "Alpha",
    workingDirectory: "/work/alpha",
    description: null,
  },
  {
    id: "beta",
    name: "Beta",
    workingDirectory: "/work/beta",
    description: "Second project",
  },
];

describe("dashboard summary", () => {
  it("maps dashboard status without confusing loading and empty", () => {
    expect(getDashboardStatus({ isLoading: true, projectCount: 0 })).toBe(
      "loading",
    );
    expect(getDashboardStatus({ isLoading: false, projectCount: 0 })).toBe(
      "empty",
    );
    expect(getDashboardStatus({ isLoading: false, projectCount: 1 })).toBe(
      "ready",
    );
    expect(
      getDashboardStatus({
        isLoading: false,
        projectCount: 1,
        errorMessage: "Failed",
      }),
    ).toBe("error");
  });

  it("keeps default start actions available without optional summaries", () => {
    const actions = buildDashboardQuickActions({ hasProjects: false });

    expect(actions).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ kind: "createProject", enabled: true }),
        expect.objectContaining({ kind: "openExistingProject", enabled: true }),
      ]),
    );
    expect(actions.find((action) => action.kind === "retry")?.enabled).toBe(
      false,
    );
  });

  it("sorts projects by recent activity and falls back to original order", () => {
    const dashboard = buildProjectDashboard({
      projects,
      isLoading: false,
      sessionsByProjectId: {
        alpha: session("alpha", 100),
        beta: session("beta", 200),
      },
    });

    expect(dashboard.projects.map((project) => project.projectId)).toEqual([
      "beta",
      "alpha",
    ]);

    const fallbackDashboard = buildProjectDashboard({
      projects,
      isLoading: false,
    });

    expect(fallbackDashboard.projects.map((project) => project.projectId)).toEqual([
      "alpha",
      "beta",
    ]);
  });

  it("uses resumable session as primary action when available", () => {
    const item = buildProjectDashboardItem({
      project: projects[0],
      sessionSummary: session("alpha", 200),
    });

    expect(item.primaryAction.kind).toBe("resumeSession");
    expect(item.primaryAction.target).toEqual({
      type: "route",
      to: "/session/alpha",
    });
  });

  it("marks unavailable optional summaries distinctly from clean states", () => {
    const worktreeSummary: WorktreeSummary = {
      projectId: "alpha",
      count: 0,
      activeCount: 0,
      status: "unavailable",
    };

    const item = buildProjectDashboardItem({
      project: projects[0],
      worktreeSummary,
    });

    expect(item.summaryStatus).toBe("unavailable");
    expect(item.unavailableReason).toContain("worktree");
  });

  it("keeps project entries visible when only one summary source fails", () => {
    const changeSummary: ChangeSummary = {
      status: "unavailable",
    };

    const item = buildProjectDashboardItem({
      project: projects[0],
      sessionSummary: session("alpha", 300),
      changeSummary,
    });

    expect(item.summaryStatus).toBe("partial");
    expect(item.primaryAction.enabled).toBe(true);
  });
});

function session(projectId: string, lastActivityMs: number): SessionSummary {
  return {
    projectId,
    sessionId: `session-${projectId}`,
    label: `${projectId} session`,
    lastActivityLabel: "방금 전",
    lastActivityMs,
    resumable: true,
    routeTarget: `/session/${projectId}`,
  };
}
