import { describe, expect, it } from "vitest";
import type { AnnotationDraft } from "@yoophi/markdown-annotation-core/types";
import { annotationsForDocument } from "./use-markdown-annotation-workspace";

const annotation = (id: string, fileName: string): AnnotationDraft => ({
  id, fileName, anchor: { blockId: "block-0" }, selectedText: id, comment: "note", type: "note", createdAt: "2026-07-12T00:00:00.000Z",
});

describe("annotationsForDocument", () => {
  it("isolates annotations by Speckit document path and preserves them for return navigation", () => {
    const state = { "specs/029/spec.md": [annotation("spec", "specs/029/spec.md")], "specs/029/plan.md": [annotation("plan", "specs/029/plan.md")] };
    expect(annotationsForDocument(state, "specs/029/spec.md").map(({ id }) => id)).toEqual(["spec"]);
    expect(annotationsForDocument(state, "specs/029/plan.md").map(({ id }) => id)).toEqual(["plan"]);
    expect(annotationsForDocument(state, null)).toEqual([]);
  });
});
