import { describe, expect, it } from "vitest";

import { createSddAction, getSddStageStates, readActiveFeaturePointer } from "@/features/worktree-workspace/model/sdd-workflow";
import type { SpeckitFeature } from "@/features/worktree-workspace/model/speckit-files";

const feature: SpeckitFeature = {
  id: "specs/031-alpha", name: "031-alpha", relativePath: "specs/031-alpha", status: "ready", taskProgress: null,
  documents: ["spec.md", "plan.md", "tasks.md"].map((name) => ({ id: `specs/031-alpha/${name}`, featureId: "specs/031-alpha", type: name === "spec.md" ? "spec" : name === "plan.md" ? "plan" : "tasks", label: name, relativePath: `specs/031-alpha/${name}`, group: "core", size: 0, modifiedMs: null, readState: "unknown" })),
};

describe("SDD workflow", () => {
  it("uses only a valid feature.json pointer", () => {
    expect(readActiveFeaturePointer('{"feature_directory":"specs/031-alpha"}', [feature])).toMatchObject({ status: "active", featurePath: "specs/031-alpha" });
    expect(readActiveFeaturePointer('{"feature_directory":"../specs/031-alpha"}', [feature])).toMatchObject({ status: "unavailable" });
    expect(readActiveFeaturePointer("not json", [feature])).toMatchObject({ status: "error" });
  });

  it("derives stage state and keeps unavailable start as a draft", () => {
    const active = readActiveFeaturePointer('{"feature_directory":"specs/031-alpha"}', [feature]);
    expect(getSddStageStates(feature, active).map((state) => state.status)).toEqual(["complete", "complete", "complete", "current"]);
    expect(createSddAction("plan", active)).toMatchObject({ delivery: "queue" });
    expect(createSddAction("specify", { status: "unavailable", featurePath: null, reason: "missing" })).toMatchObject({ delivery: "draft" });
  });
});
