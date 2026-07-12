import { useEffect, useMemo, useState, type RefObject } from "react";
import {
  formatAnnotationsForAgent,
  isFullBlockAnnotation,
} from "@yoophi/markdown-annotation-core";
import type {
  AnnotationAnchor,
  AnnotationDraft,
  AnnotationType,
  MarkdownBlock,
} from "@yoophi/markdown-annotation-core/types";
import {
  buildViewerAnnotationMaps,
  getSelectionAnchors,
  getSelectionRects,
  type SelectionRect,
} from "@yoophi/markdown-annotation-react";

export type AnnotationDraftTarget =
  | { kind: "block"; block: MarkdownBlock }
  | { kind: "selection"; anchors: AnnotationAnchor[]; text: string };

export function annotationsForDocument(
  annotationsByFile: Record<string, AnnotationDraft[]>,
  documentPath: string | null,
) {
  return documentPath ? (annotationsByFile[documentPath] ?? []) : [];
}

export function useMarkdownAnnotationWorkspace({
  blocks,
  contentRef,
  documentPath,
  previewRef,
}: {
  blocks: MarkdownBlock[];
  contentRef: RefObject<HTMLDivElement | null>;
  documentPath: string | null;
  previewRef: RefObject<HTMLDivElement | null>;
}) {
  const [annotationsByFile, setAnnotationsByFile] = useState<Record<string, AnnotationDraft[]>>({});
  const [draftTarget, setDraftTarget] = useState<AnnotationDraftTarget | null>(null);
  const [draftType, setDraftType] = useState<AnnotationType>("note");
  const [draftComment, setDraftComment] = useState("");
  const [editingAnnotationId, setEditingAnnotationId] = useState<string | null>(null);
  const [selectionAnchors, setSelectionAnchors] = useState<AnnotationAnchor[]>([]);
  const [selectionHighlightRects, setSelectionHighlightRects] = useState<SelectionRect[]>([]);
  const [selectionToolbarPosition, setSelectionToolbarPosition] = useState<{ left: number; top: number } | null>(null);
  const annotations = annotationsForDocument(annotationsByFile, documentPath);
  const viewerMaps = useMemo(() => buildViewerAnnotationMaps(annotations, blocks), [annotations, blocks]);
  const annotationPrompt = useMemo(
    () => (documentPath ? formatAnnotationsForAgent(documentPath, annotations, blocks) : ""),
    [annotations, blocks, documentPath],
  );

  function resetSelection() {
    setSelectionAnchors([]);
    setSelectionHighlightRects([]);
    setSelectionToolbarPosition(null);
    globalThis.getSelection?.()?.removeAllRanges();
  }

  function resetDraft() {
    setDraftTarget(null);
    setDraftType("note");
    setDraftComment("");
    setEditingAnnotationId(null);
  }

  useEffect(() => {
    resetDraft();
    resetSelection();
  }, [documentPath]);

  function removeAnnotation(annotationId: string) {
    if (!documentPath) return;
    setAnnotationsByFile((current) => {
      const target = (current[documentPath] ?? []).find(({ id }) => id === annotationId);
      return {
        ...current,
        [documentPath]: (current[documentPath] ?? []).filter((annotation) =>
          target?.groupId ? annotation.groupId !== target.groupId : annotation.id !== annotationId,
        ),
      };
    });
  }

  function requestBlockComment(block: MarkdownBlock) {
    setEditingAnnotationId(null);
    setDraftTarget({ kind: "block", block });
    setDraftType("note");
    setDraftComment("");
  }

  function toggleBlockDelete(block: MarkdownBlock) {
    if (!documentPath) return;
    const existing = annotations.find(
      (annotation) => annotation.type === "delete" && isFullBlockAnnotation(annotation, block),
    );
    if (existing) {
      removeAnnotation(existing.id);
      return;
    }
    addAnnotations([createAnnotation(fullBlockAnchor(block), documentPath, "delete", "", block)]);
  }

  function editAnnotation(annotationId: string) {
    const annotation = annotations.find(({ id }) => id === annotationId);
    if (!annotation) return;
    setEditingAnnotationId(annotation.id);
    setDraftType(annotation.type);
    setDraftComment(annotation.comment);
    setDraftTarget({ kind: "selection", anchors: [annotation.anchor], text: annotation.selectedText });
  }

  function addAnnotations(next: AnnotationDraft[]) {
    if (!documentPath) return;
    setAnnotationsByFile((current) => ({
      ...current,
      [documentPath]: [...(current[documentPath] ?? []), ...next],
    }));
  }

  function saveAnnotation() {
    if (!documentPath || !draftTarget || (draftType !== "delete" && !draftComment.trim())) return;
    const comment = draftComment.trim();
    if (editingAnnotationId) {
      setAnnotationsByFile((current) => {
        const items = current[documentPath] ?? [];
        const editing = items.find(({ id }) => id === editingAnnotationId);
        return {
          ...current,
          [documentPath]: items.map((annotation) =>
            annotation.id === editingAnnotationId || (editing?.groupId && annotation.groupId === editing.groupId)
              ? { ...annotation, comment, type: draftType }
              : annotation,
          ),
        };
      });
    } else if (draftTarget.kind === "block") {
      addAnnotations([createAnnotation(fullBlockAnchor(draftTarget.block), documentPath, draftType, comment, draftTarget.block)]);
    } else {
      const groupId = draftTarget.anchors.length > 1 ? crypto.randomUUID() : undefined;
      addAnnotations(draftTarget.anchors.map((anchor) => createAnnotation(anchor, documentPath, draftType, comment, undefined, groupId)));
    }
    resetDraft();
    resetSelection();
  }

  function captureSelection() {
    const anchors = getSelectionAnchors(previewRef.current);
    if (anchors.length === 0) {
      resetSelection();
      return;
    }
    const rects = getSelectionRects(contentRef.current);
    const last = rects[rects.length - 1];
    setSelectionAnchors(anchors);
    setSelectionHighlightRects(rects);
    setSelectionToolbarPosition(last ? { left: last.left + last.width + 8, top: last.top } : null);
  }

  function requestSelectionNote() {
    if (selectionAnchors.length === 0) return;
    setEditingAnnotationId(null);
    setDraftTarget({
      kind: "selection",
      anchors: selectionAnchors,
      text: selectionAnchors.map(({ selectedText }) => selectedText).filter(Boolean).join("\n"),
    });
    setDraftType("note");
    setDraftComment("");
    setSelectionToolbarPosition(null);
  }

  function deleteSelection() {
    if (!documentPath || selectionAnchors.length === 0) return;
    const groupId = selectionAnchors.length > 1 ? crypto.randomUUID() : undefined;
    addAnnotations(selectionAnchors.map((anchor) => createAnnotation(anchor, documentPath, "delete", "", undefined, groupId)));
    resetSelection();
  }

  return {
    annotationPrompt, annotations, deleteSelection, draftComment, draftTarget, draftType,
    editingAnnotationId, editAnnotation, captureSelection, removeAnnotation, requestBlockComment,
    requestSelectionNote, resetDraft, resetSelection, saveAnnotation, selectionHighlightRects,
    selectionToolbarPosition, setDraftComment, setDraftType, toggleBlockDelete, viewerMaps,
  };
}

export type MarkdownAnnotationWorkspaceModel = ReturnType<typeof useMarkdownAnnotationWorkspace>;

function fullBlockAnchor(block: MarkdownBlock): AnnotationAnchor {
  return { blockId: block.id, startLine: block.startLine, endLine: block.endLine, startOffset: 0, endOffset: block.content.length, selectedText: block.rawContent };
}

function createAnnotation(anchor: AnnotationAnchor, fileName: string, type: AnnotationType, comment: string, block?: MarkdownBlock, groupId?: string): AnnotationDraft {
  return { id: crypto.randomUUID(), groupId, fileName, anchor, selectedText: anchor.selectedText ?? block?.rawContent ?? "", comment, type, createdAt: new Date().toISOString() };
}
