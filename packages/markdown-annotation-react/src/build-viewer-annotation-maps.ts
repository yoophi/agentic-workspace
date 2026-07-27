import { isFullBlockAnnotation } from "@yoophi/markdown-annotation-core";
import type { AnnotationDraft, MarkdownBlock } from "@yoophi/markdown-annotation-core/types";
import type { MarkdownViewerBlockNote, MarkdownViewerInlineAnnotation } from "./types";

export type ViewerAnnotationMaps = {
  annotatedBlockIds: Set<string>;
  deletedBlockIds: Set<string>;
  inlineAnnotationsByBlock: Map<string, MarkdownViewerInlineAnnotation[]>;
  noteAnnotationsByBlock: Map<string, MarkdownViewerBlockNote[]>;
};

/**
 * AnnotationDraft[] + blocks로부터 MarkdownViewer가 받는 4개 맵을 만든다.
 * MA AnnotatorPage의 매핑 정책을 정본으로 단일화한 것이다:
 * - 전체 블록 delete만 deletedBlockIds로 취해 블록 셸에 strike 처리
 * - inline `<mark>`는 부분 선택된 delete/note만 표시(전체 블록·offset 없음은 제외)
 * - note는 블록 노트 목록(아이콘 툴팁)에 모은다
 */
export function buildViewerAnnotationMaps(
  annotations: AnnotationDraft[],
  blocks: MarkdownBlock[],
): ViewerAnnotationMaps {
  const blocksById = new Map(blocks.map((block) => [block.id, block]));
  const activeAnnotations = annotations.filter((annotation) => blocksById.has(annotation.anchor.blockId));
  const annotatedBlockIds = new Set(activeAnnotations.map((annotation) => annotation.anchor.blockId));

  const deletedBlockIds = new Set(
    activeAnnotations
      .filter((annotation) => {
        if (annotation.type !== "delete") {
          return false;
        }
        const block = blocksById.get(annotation.anchor.blockId);
        return block !== undefined && isFullBlockAnnotation(annotation, block);
      })
      .map((annotation) => annotation.anchor.blockId),
  );

  const noteAnnotationsByBlock = new Map<string, MarkdownViewerBlockNote[]>();
  activeAnnotations
    .filter((annotation) => annotation.type === "note")
    .forEach((annotation) => {
      const blockNotes = noteAnnotationsByBlock.get(annotation.anchor.blockId) ?? [];
      blockNotes.push({ id: annotation.id, comment: annotation.comment });
      noteAnnotationsByBlock.set(annotation.anchor.blockId, blockNotes);
    });

  const inlineAnnotationsByBlock = new Map<string, MarkdownViewerInlineAnnotation[]>();
  activeAnnotations
    .filter((annotation) => annotation.type === "delete" || annotation.type === "note")
    .forEach((annotation) => {
      const block = blocksById.get(annotation.anchor.blockId);
      if (
        !block ||
        annotation.anchor.startOffset === undefined ||
        annotation.anchor.endOffset === undefined ||
        isFullBlockAnnotation(annotation, block)
      ) {
        return;
      }

      const blockAnnotations = inlineAnnotationsByBlock.get(annotation.anchor.blockId) ?? [];
      blockAnnotations.push({
        id: annotation.id,
        comment: annotation.comment,
        endOffset: annotation.anchor.endOffset,
        startOffset: annotation.anchor.startOffset,
        type: annotation.type,
      });
      inlineAnnotationsByBlock.set(annotation.anchor.blockId, blockAnnotations);
    });

  return { annotatedBlockIds, deletedBlockIds, inlineAnnotationsByBlock, noteAnnotationsByBlock };
}
