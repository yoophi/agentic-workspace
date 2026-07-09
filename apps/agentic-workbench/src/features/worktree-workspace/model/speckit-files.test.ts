import { describe, expect, it } from "vitest";

import type { WorktreeFileEntry } from "@/entities/worktree-file/model/types";
import {
  buildSpeckitFeatures,
  classifySpeckitDocument,
  getTaskDocumentPaths,
  summarizeTaskProgress,
} from "@/features/worktree-workspace/model/speckit-files";

function file(relativePath: string, size = 10): WorktreeFileEntry {
  const name = relativePath.split("/").pop() ?? relativePath;
  return {
    name,
    path: `/repo/${relativePath}`,
    relativePath,
    isDir: false,
    size,
    modifiedMs: 1,
  };
}

function dir(relativePath: string): WorktreeFileEntry {
  const name = relativePath.split("/").pop() ?? relativePath;
  return {
    name,
    path: `/repo/${relativePath}`,
    relativePath,
    isDir: true,
    size: 0,
    modifiedMs: 1,
  };
}

describe("speckit files model", () => {
  it("groups root-relative specs entries into Speckit features", () => {
    const features = buildSpeckitFeatures([
      dir("specs"),
      dir("specs/001-alpha"),
      file("specs/001-alpha/spec.md"),
      file("specs/001-alpha/plan.md"),
      file("specs/001-alpha/tasks.md"),
      file("specs/002-beta/spec.md"),
    ]);

    expect(features.map((feature) => feature.relativePath)).toEqual([
      "specs/001-alpha",
      "specs/002-beta",
    ]);
    expect(features[0]?.documents.map((document) => document.type)).toEqual([
      "spec",
      "plan",
      "tasks",
    ]);
  });

  it("classifies contracts and checklists separately from core documents", () => {
    expect(classifySpeckitDocument("specs/001-alpha/spec.md")).toEqual({
      type: "spec",
      group: "core",
      label: "Spec",
    });
    expect(classifySpeckitDocument("specs/001-alpha/contracts/frontend-state.md")).toEqual({
      type: "contract",
      group: "contracts",
      label: "Contract",
    });
    expect(classifySpeckitDocument("specs/001-alpha/checklists/requirements.md")).toEqual({
      type: "checklist",
      group: "checklists",
      label: "Checklist",
    });
  });

  it("keeps duplicate basenames distinct by feature-relative path", () => {
    const features = buildSpeckitFeatures([
      file("specs/001-alpha/contracts/api.md"),
      file("specs/002-beta/contracts/api.md"),
    ]);

    expect(features.flatMap((feature) => feature.documents.map((document) => document.id))).toEqual([
      "specs/001-alpha/contracts/api.md",
      "specs/002-beta/contracts/api.md",
    ]);
  });

  it("summarizes checkbox task states", () => {
    expect(summarizeTaskProgress("", "specs/001/tasks.md")).toMatchObject({
      total: 0,
      completed: 0,
      remaining: 0,
      state: "noTasks",
    });
    expect(summarizeTaskProgress("- [ ] Todo\n- [ ] Next", "specs/001/tasks.md")).toMatchObject({
      total: 2,
      completed: 0,
      remaining: 2,
      state: "notStarted",
    });
    expect(summarizeTaskProgress("- [x] Done\n- [ ] Todo", "specs/001/tasks.md")).toMatchObject({
      total: 2,
      completed: 1,
      remaining: 1,
      state: "inProgress",
    });
    expect(summarizeTaskProgress("- [X] Done", "specs/001/tasks.md")).toMatchObject({
      total: 1,
      completed: 1,
      remaining: 0,
      state: "complete",
    });
  });

  it("assigns TaskProgressSummary to matching tasks.md documents", () => {
    const features = buildSpeckitFeatures(
      [file("specs/001-alpha/tasks.md"), file("specs/002-beta/tasks.md")],
      {
        "specs/001-alpha/tasks.md": "- [x] Done\n- [ ] Todo",
        "specs/002-beta/tasks.md": "No checkbox tasks",
      },
    );

    expect(features[0]?.taskProgress).toMatchObject({
      sourcePath: "specs/001-alpha/tasks.md",
      total: 2,
      completed: 1,
      remaining: 1,
      state: "inProgress",
    });
    expect(features[1]?.taskProgress).toMatchObject({
      sourcePath: "specs/002-beta/tasks.md",
      total: 0,
      state: "noTasks",
    });
  });

  it("returns task document paths for loading task summaries", () => {
    const features = buildSpeckitFeatures([
      file("specs/001-alpha/spec.md"),
      file("specs/001-alpha/tasks.md"),
      file("specs/002-beta/spec.md"),
    ]);

    expect(getTaskDocumentPaths(features)).toEqual(["specs/001-alpha/tasks.md"]);
  });

  it("ignores non-Speckit directories and non-markdown files under specs", () => {
    const features = buildSpeckitFeatures([
      file("README.md"),
      dir("specs/not-a-feature"),
      file("specs/not-a-feature/readme.txt"),
      file("specs/001-alpha/spec.md"),
    ]);

    expect(features).toHaveLength(1);
    expect(features[0]?.relativePath).toBe("specs/001-alpha");
  });
});
