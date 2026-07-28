import { describe, expect, it } from "vitest";
import { parseMarkdownToBlocks } from "@yoophi/markdown-annotation-core";
import type { AnnotationDraft } from "@yoophi/markdown-annotation-core/types";
import { buildViewerAnnotationMaps } from "./build-viewer-annotation-maps";

const createdAt = "2026-07-28T00:00:00.000Z";

function annotation(blockId: string): AnnotationDraft {
  return { id: `note-${blockId}`, fileName: "fixture.md", anchor: { blockId, startOffset: 0, endOffset: 4 }, selectedText: "item", comment: "note", type: "note", createdAt };
}

describe("buildViewerAnnotationMaps", () => {
  it("keeps annotations scoped to an existing semantic block", () => {
    const blocks = parseMarkdownToBlocks("- Parent\n  - Child");
    const existing = blocks[1];
    expect(existing).toBeDefined();
    const maps = buildViewerAnnotationMaps([annotation(existing?.id ?? ""), annotation("stale-block")], blocks);

    expect(maps.inlineAnnotationsByBlock.has(existing?.id ?? "")).toBe(true);
    expect(maps.inlineAnnotationsByBlock.has("stale-block")).toBe(false);
    expect(maps.noteAnnotationsByBlock.has("stale-block")).toBe(false);
    expect(maps.annotatedBlockIds.has("stale-block")).toBe(false);
  });
});
