import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";

import type { SpeckitFeature } from "@/features/worktree-workspace/model/speckit-files";
import { SpeckitFilesPanel } from "@/features/worktree-workspace/ui/speckit-files-panel";

const sampleFeatures: SpeckitFeature[] = [
  {
    id: "specs/001-alpha",
    name: "001-alpha",
    relativePath: "specs/001-alpha",
    status: "ready",
    taskProgress: {
      total: 2,
      completed: 1,
      remaining: 1,
      state: "inProgress",
      sourcePath: "specs/001-alpha/tasks.md",
    },
    documents: [
      {
        id: "specs/001-alpha/spec.md",
        featureId: "specs/001-alpha",
        type: "spec",
        label: "Spec",
        relativePath: "specs/001-alpha/spec.md",
        group: "core",
        size: 10,
        modifiedMs: 1,
        readState: "unknown",
      },
      {
        id: "specs/001-alpha/plan.md",
        featureId: "specs/001-alpha",
        type: "plan",
        label: "Plan",
        relativePath: "specs/001-alpha/plan.md",
        group: "core",
        size: 10,
        modifiedMs: 1,
        readState: "unknown",
      },
      {
        id: "specs/001-alpha/tasks.md",
        featureId: "specs/001-alpha",
        type: "tasks",
        label: "Tasks",
        relativePath: "specs/001-alpha/tasks.md",
        group: "core",
        size: 10,
        modifiedMs: 1,
        readState: "unknown",
      },
      {
        id: "specs/001-alpha/research.md",
        featureId: "specs/001-alpha",
        type: "research",
        label: "Research",
        relativePath: "specs/001-alpha/research.md",
        group: "core",
        size: 10,
        modifiedMs: 1,
        readState: "unknown",
      },
      {
        id: "specs/001-alpha/data-model.md",
        featureId: "specs/001-alpha",
        type: "dataModel",
        label: "Data model",
        relativePath: "specs/001-alpha/data-model.md",
        group: "core",
        size: 10,
        modifiedMs: 1,
        readState: "unknown",
      },
      {
        id: "specs/001-alpha/quickstart.md",
        featureId: "specs/001-alpha",
        type: "quickstart",
        label: "Quickstart",
        relativePath: "specs/001-alpha/quickstart.md",
        group: "core",
        size: 10,
        modifiedMs: 1,
        readState: "unknown",
      },
      {
        id: "specs/001-alpha/contracts/frontend-state.md",
        featureId: "specs/001-alpha",
        type: "contract",
        label: "Contract",
        relativePath: "specs/001-alpha/contracts/frontend-state.md",
        group: "contracts",
        size: 10,
        modifiedMs: 1,
        readState: "unknown",
      },
    ],
  },
];

describe("SpeckitFilesPanel", () => {
  it("renders feature rows collapsed by default", () => {
    const html = renderToStaticMarkup(
      <SpeckitFilesPanel
        features={sampleFeatures}
        selectedDocumentPath="specs/001-alpha/spec.md"
        onRefresh={() => undefined}
        onSelectDocument={() => undefined}
      />,
    );

    expect(html).toContain("001-alpha");
    expect(html).toContain("Path");
    expect(html).toContain("Task");
    expect(html).toContain("Speckit 정렬을 정순으로 변경");
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain("모든 Speckit feature 펼치기");
    expect(html).toContain("Spec");
    expect(html).toContain("Plan");
    expect(html).toContain("Tasks");
    expect(html).not.toContain(">yes<");
    expect(html).not.toContain(">no<");
    expect(html).not.toContain("specs/001-alpha/spec.md");
    expect(html).toContain("1/2 done");
  });

  it("sorts features by path descending by default", () => {
    const html = renderToStaticMarkup(
      <SpeckitFilesPanel
        features={[
          sampleFeatures[0],
          {
            ...sampleFeatures[0],
            id: "specs/002-beta",
            name: "002-beta",
            relativePath: "specs/002-beta",
          },
        ]}
        selectedDocumentPath={null}
        onRefresh={() => undefined}
        onSelectDocument={() => undefined}
      />,
    );

    expect(html.indexOf("002-beta")).toBeLessThan(html.indexOf("001-alpha"));
  });

  it("renders grouped document rows when a feature is expanded", () => {
    const html = renderToStaticMarkup(
      <SpeckitFilesPanel
        features={sampleFeatures}
        initialExpandedFeatureIds={["specs/001-alpha"]}
        selectedDocumentPath="specs/001-alpha/spec.md"
        onRefresh={() => undefined}
        onSelectDocument={() => undefined}
      />,
    );

    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain("모든 Speckit feature 접기");
    expect(html).toContain("Spec");
    expect(html).toContain("Contract");
    expect(html).toContain("specs/001-alpha/spec.md");
    expect(html).toContain('data-plan-artifact="true"');
    expect(html.indexOf("specs/001-alpha/plan.md")).toBeLessThan(
      html.indexOf("specs/001-alpha/research.md"),
    );
    expect(html.indexOf("specs/001-alpha/research.md")).toBeLessThan(
      html.indexOf("specs/001-alpha/data-model.md"),
    );
    expect(html.indexOf("specs/001-alpha/data-model.md")).toBeLessThan(
      html.indexOf("specs/001-alpha/contracts/frontend-state.md"),
    );
    expect(html.indexOf("specs/001-alpha/contracts/frontend-state.md")).toBeLessThan(
      html.indexOf("specs/001-alpha/quickstart.md"),
    );
    expect(html.indexOf("specs/001-alpha/quickstart.md")).toBeLessThan(
      html.indexOf("specs/001-alpha/tasks.md"),
    );
  });

  it("renders empty and error states", () => {
    const emptyHtml = renderToStaticMarkup(
      <SpeckitFilesPanel
        features={[]}
        selectedDocumentPath={null}
        onRefresh={() => undefined}
        onSelectDocument={() => undefined}
      />,
    );
    const errorHtml = renderToStaticMarkup(
      <SpeckitFilesPanel
        errorMessage="boom"
        features={[]}
        selectedDocumentPath={null}
        onRefresh={() => undefined}
        onSelectDocument={() => undefined}
      />,
    );

    expect(emptyHtml).toContain("Speckit 문서 없음");
    expect(errorHtml).toContain("Speckit 문서를 불러오지 못했습니다.");
    expect(errorHtml).toContain("boom");
  });

  it("renders no-task and missing-tasks states", () => {
    const html = renderToStaticMarkup(
      <SpeckitFilesPanel
        features={[
          {
            ...sampleFeatures[0],
            taskProgress: null,
          },
          {
            ...sampleFeatures[0],
            id: "specs/002-beta",
            name: "002-beta",
            relativePath: "specs/002-beta",
            taskProgress: {
              total: 0,
              completed: 0,
              remaining: 0,
              state: "noTasks",
              sourcePath: "specs/002-beta/tasks.md",
            },
          },
        ]}
        selectedDocumentPath={null}
        onRefresh={() => undefined}
        onSelectDocument={() => undefined}
      />,
    );

    expect(html).toContain("No tasks.md");
    expect(html).toContain("No checkbox tasks");
  });

  it("marks fully completed features with a distinct card state", () => {
    const html = renderToStaticMarkup(
      <SpeckitFilesPanel
        features={[
          {
            ...sampleFeatures[0],
            taskProgress: {
              total: 2,
              completed: 2,
              remaining: 0,
              state: "complete",
              sourcePath: "specs/001-alpha/tasks.md",
            },
          },
        ]}
        selectedDocumentPath={null}
        onRefresh={() => undefined}
        onSelectDocument={() => undefined}
      />,
    );

    expect(html).toContain('data-complete="true"');
    expect(html).toContain("2/2 done");
  });

  it("uses root-relative paths as document button identity", () => {
    const onSelectDocument = vi.fn();
    const document = sampleFeatures[0].documents[0];

    onSelectDocument(document.relativePath);

    expect(onSelectDocument).toHaveBeenCalledWith("specs/001-alpha/spec.md");
  });
});
